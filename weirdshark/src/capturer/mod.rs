use std::collections::HashMap;
use std::collections::linked_list::LinkedList;

use std::mem;
use std::net::IpAddr;
use std::sync::mpsc::{Receiver, Sender};
use std::thread::{JoinHandle};
use std::path::PathBuf;
use std::thread;
use std::time::Duration;
use chrono::Utc;
use pnet::datalink::{channel, DataLinkReceiver, NetworkInterface};
use pnet::datalink::Channel::Ethernet;
use crate::{Record, RecordKey, RecordValue, TransportProtocols};
pub use crate::capturer::builder::CapturerBuilder;
use crate::capturer::write_scheduler::WriteScheduler;
use crate::error::WeirdsharkError;
use crate::filters::{DirectedFilter};


pub mod builder;
mod write_scheduler;
pub mod parser;

pub enum WorkerCommand {
    Start,
    Pause,
    Stop,
    WriteFile,
    HandlePacket(Result<Vec<u8>, std::io::Error>),
}

pub struct Capturer {
    worker_thread_handle: JoinHandle<()>,
    worker_sender: Sender<WorkerCommand>,
}

impl Capturer {
    pub fn start(&self) {
        //TODO: manager error
        self.worker_sender.send(WorkerCommand::Start).unwrap();
    }

    pub fn pause(&self) {
        //TODO: manager error
        self.worker_sender.send(WorkerCommand::Pause).unwrap();
    }

    //TODO replace stop with drop implementation
    pub fn stop(self) {
        //TODO: manager error
        self.worker_sender.send(WorkerCommand::WriteFile).unwrap();
        self.worker_sender.send(WorkerCommand::Stop).unwrap();
        match self.worker_thread_handle.join() {
            Ok(_) => {}
            Err(e) => println!("{:?}", e) //TODO manage properly
        }
    }
}

struct CapturerWorker {
    sender: Sender<WorkerCommand>,
    receiver: Receiver<WorkerCommand>,
    map: HashMap<RecordKey, RecordValue>,
    report_scheduler: Option<WriteScheduler>,
    report_path: PathBuf,
    report_name_prefix: String,
    is_paused: bool,
    interface: NetworkInterface,
    ip_filters: LinkedList<DirectedFilter<IpAddr>>,
    port_filters: LinkedList<DirectedFilter<u16>>,
    protocol_filter: Option<TransportProtocols>,
}

impl CapturerWorker {
    fn new(interface: NetworkInterface,
           report_path: PathBuf,
           report_name_prefix: String,
           report_interval: Option<Duration>,
           ip_filters: LinkedList<DirectedFilter<IpAddr>>,
           port_filters: LinkedList<DirectedFilter<u16>>,
            protocol_filter : Option<TransportProtocols>,
    ) -> Self {
        let (sender, receiver) = std::sync::mpsc::channel();
        let map = HashMap::new();
        let report_scheduler = match report_interval {
            Some(interval) => Some(WriteScheduler::new(interval, sender.clone())),
            None => None,
        };
        let is_paused = false;

        Self { sender, receiver, map, report_scheduler, report_path, report_name_prefix, is_paused, interface, ip_filters, port_filters, protocol_filter }
    }

    fn get_sender(&self) -> Sender<WorkerCommand> {
        self.sender.clone()
    }

    fn write_csv(&mut self) -> Result<(), WeirdsharkError> {
        let file_name = (self.report_name_prefix.clone() +
            &Utc::now().to_string()).replace(":", "-").replace(".", "_") +
            ".csv"; //TODO: manage prefix parameter
        let path = self.report_path.join(&file_name);
        let mut writer = match csv::Writer::from_path(&path) {
            Ok(writer) => writer,
            Err(os_err) => {
                let path_str = path.to_str().unwrap();
                let err = format!("Cannot write to : {} for \n{}", path_str, os_err);
                return Err(WeirdsharkError::WriteError(err));
            }
        };
        let map = mem::take(&mut self.map);

        for (k, v) in map.into_iter() {
            let record = Record::from_key_value(k, v);
            match writer.serialize(record) {
                Ok(()) => (),
                Err(error) => return Err(WeirdsharkError::SerializeError(format!("{}", error)))
            };
        }

        match writer.flush() {
            Ok(()) => (),
            Err(error) => return Err(WeirdsharkError::IoError(format!("{}", error)))
        };
        Ok(())
    }

    fn work(mut self) -> JoinHandle<()> {
        PnetCaptureAdapter::new(&self.interface, self.get_sender()).capture(); //TODO check and throw eventual errors
        thread::spawn(move ||
            loop {
                match self.receiver.recv() {
                    Ok(command) =>
                        match command {
                            WorkerCommand::Start => {
                                if let Some(s) = &self.report_scheduler {
                                    s.start();
                                }
                                self.is_paused = false;
                            }
                            WorkerCommand::Pause => {
                                if let Some(s) = &self.report_scheduler {
                                    s.stop();
                                }
                                self.is_paused = true;
                            }
                            WorkerCommand::Stop => break,
                            WorkerCommand::HandlePacket(p) => {
                                if self.is_paused { continue; }
                                match p {
                                    Ok(data) => {
                                        let parse_res = parser::parse_transport_packet(data);
                                        if let Ok(packet_info) = parse_res {
                                            if !self.ip_filters.is_empty(){
                                                if !self.ip_filters.iter().any(|filter: &DirectedFilter<IpAddr>|{
                                                    filter.filter(&packet_info.source_ip,&packet_info.destination_ip)
                                                }) {
                                                    continue;
                                                }
                                            }

                                            if !self.port_filters.is_empty(){
                                                if !self.port_filters.iter().any(|filter: &DirectedFilter<u16>|{
                                                    filter.filter(&packet_info.source_port,&packet_info.destination_port)
                                                }) {
                                                    continue;
                                                }
                                            }

                                            match self.protocol_filter {
                                                Some(protcol) => if protcol != packet_info.transport_protocol {continue}
                                                None => (),
                                            }

                                            let k = RecordKey {
                                                source_ip: packet_info.source_ip,
                                                destination_ip: packet_info.destination_ip,
                                                transport_protocol: packet_info.transport_protocol,
                                                source_port: packet_info.source_port,
                                                destination_port: packet_info.destination_port,
                                            };

                                            let now = Utc::now();
                                            self.map.entry(k)
                                                .and_modify(|v: &mut RecordValue| {
                                                    v.bytes += packet_info.bytes;
                                                    v.last_seen = now;
                                                })
                                                .or_insert(RecordValue { bytes: packet_info.bytes, first_seen: now, last_seen: now });
                                        }
                                    }
                                    Err(e) => panic!("packetdump: unable to receive packet: {}", e), //TODO manage with errors
                                }
                            }
                            WorkerCommand::WriteFile => {
                                self.write_csv()
                                    .expect("Weirdshark encountered an error while writing the file"); //TODO manage with errors?
                            }
                        }
                    Err(_) => todo!(),
                };
            }
        )
    }
}

struct PnetCaptureAdapter {
    worker_sender: Sender<WorkerCommand>,
    pnet_receiver: Box<dyn DataLinkReceiver>,
}

impl PnetCaptureAdapter {
    fn new(interface: &NetworkInterface, worker_sender: Sender<WorkerCommand>) -> Self {
        let pnet_receiver = match channel(&interface, Default::default()) {
            Ok(Ethernet(_, receiver)) => receiver,
            Ok(_) => panic!("packetdump: unhandled channel type"), //TODO manage with errors
            Err(e) => panic!("packetdump: unable to create channel: {}", e), //TODO manage with errors
        };
        Self { worker_sender, pnet_receiver }
    }

    fn capture(mut self) {
        thread::spawn(move || {
            loop {
                let packet = WorkerCommand::HandlePacket(
                    match self.pnet_receiver.next() {
                        Ok(p) => Ok(p.to_vec()),
                        Err(e) => Err(e),
                    }
                );
                match self.worker_sender.send(packet) {
                    Ok(_) => continue,
                    Err(_) => break,
                }
            }
        });
    }
}
