use std::collections::HashMap;
use std::collections::linked_list::LinkedList;
use std::error::Error;

use std::mem;
use std::net::IpAddr;
use std::sync::mpsc::{Receiver, Sender};
use std::thread::{JoinHandle};
use std::path::PathBuf;
use std::thread;
use std::time::Duration;
use pnet::datalink::{channel, DataLinkReceiver, NetworkInterface};
use pnet::datalink::Channel::Ethernet;
use crate::{Record, RecordKey, RecordValue};
pub use crate::capturer::builder::CapturerBuilder;
use crate::capturer::parser::TransportPacket;
use crate::capturer::write_scheduler::WriteScheduler;
use crate::error::WeirdsharkError;
use crate::filters::{DirectedFilter};
use crate::TransportProtocols;

mod builder;
mod write_scheduler;
mod parser;

/// Capture manager.
///
/// This struct manages a capture, it can pause, start and stop the capture.
pub struct Capturer {
    worker_thread_handle: JoinHandle<()>,
    worker_sender: Sender<WorkerCommand>,
}

impl Capturer {
    /// Start the capture
    pub fn start(&self) -> Result<(), WeirdsharkError> {
        self.worker_sender.send(WorkerCommand::Start)
            .map_err(|_| WeirdsharkError::CapturerError("Cannot start capturer.".to_string()))
    }

    /// Pause the capture
    pub fn pause(&self) -> Result<(), WeirdsharkError> {
        self.worker_sender.send(WorkerCommand::Pause)
            .map_err(|_| WeirdsharkError::CapturerError("Cannot pause capturer.".to_string()))
    }

    /// Stop the capture
    pub fn stop(self) -> Result<(), WeirdsharkError> {
        self.worker_sender.send(WorkerCommand::WriteFile)
            .map_err(|_| WeirdsharkError::CapturerError("Cannot save report.".to_string()))?;
        self.worker_sender.send(WorkerCommand::Stop)
            .map_err(|_| WeirdsharkError::CapturerError("Cannot stop capturer.".to_string()))?;
        self.worker_thread_handle.join()
            .map_err(|e| WeirdsharkError::CapturerError(e.downcast::<&str>().unwrap().to_string()))
    }
}

#[derive(Debug)]
enum WorkerCommand {
    Start,
    Pause,
    Stop,
    WriteFile,
    HandlePacket(Result<Vec<u8>, std::io::Error>),
}

struct CapturerWorker {
    sender: Sender<WorkerCommand>,
    receiver: Receiver<WorkerCommand>,
    map: HashMap<RecordKey, RecordValue>,
    report_scheduler: Option<WriteScheduler>,
    report_path: PathBuf,
    is_paused: bool,
    interface: NetworkInterface,
    ip_filters: LinkedList<DirectedFilter<IpAddr>>,
    port_filters: LinkedList<DirectedFilter<u16>>,
    protocol_filter: Option<TransportProtocols>,
}

impl CapturerWorker {
    fn new(interface: NetworkInterface,
           report_path: PathBuf,
           report_interval: Option<Duration>,
           ip_filters: LinkedList<DirectedFilter<IpAddr>>,
           port_filters: LinkedList<DirectedFilter<u16>>,
           protocol_filter: Option<TransportProtocols>,
    ) -> Self {
        let (sender, receiver) = std::sync::mpsc::channel();
        let map = HashMap::new();
        let report_scheduler = match report_interval {
            Some(interval) => Some(WriteScheduler::new(interval, sender.clone())),
            None => None,
        };
        let is_paused = false;

        Self { sender, receiver, map, report_scheduler, report_path, is_paused, interface, ip_filters, port_filters, protocol_filter }
    }

    fn get_sender(&self) -> Sender<WorkerCommand> {
        self.sender.clone()
    }

    fn write_csv(&mut self) -> Result<(), Box<dyn Error>> {
        let file_name = ("weirdshark_capture_".to_string() +
            self.interface.name.as_str() +
            "_" +
            &chrono::Local::now().to_string() +
            ".csv"
        ).replace(":", "-");
        let path = self.report_path.join(&file_name);
        let mut writer = csv::Writer::from_path(&path)?;
        let map = mem::take(&mut self.map);

        for (k, v) in map.into_iter() {
            let record = Record::from_key_value(k, v);
            writer.serialize(record)?;
        }

        writer.flush()?;
        Ok(())
    }

    fn apply_filters(&self, packet_info: &TransportPacket) -> bool {
        if !self.ip_filters.is_empty() && !self.ip_filters.iter().any(|filter: &DirectedFilter<IpAddr>| {
            filter.filter(&packet_info.source_ip, &packet_info.destination_ip)
        }) {
            return false;
        }

        if !self.port_filters.is_empty() && !self.port_filters.iter().any(|filter: &DirectedFilter<u16>| {
            filter.filter(&packet_info.source_port, &packet_info.destination_port)
        }) {
            return false;
        }

        if self.protocol_filter.is_some() && self.protocol_filter.unwrap() != packet_info.transport_protocol {
            return false;
        }

        true
    }

    fn work(mut self) -> JoinHandle<()> {
        PnetCaptureAdapter::new(&self.interface, self.get_sender()).capture();
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
                                            if !self.apply_filters(&packet_info) {
                                                continue;
                                            }
                                            let k = RecordKey {
                                                source_ip: packet_info.source_ip,
                                                destination_ip: packet_info.destination_ip,
                                                transport_protocol: packet_info.transport_protocol,
                                                source_port: packet_info.source_port,
                                                destination_port: packet_info.destination_port,
                                            };

                                            let now = chrono::Local::now();
                                            self.map.entry(k)
                                                .and_modify(|v| {
                                                    v.bytes += packet_info.bytes;
                                                    v.last_seen = now;
                                                })
                                                .or_insert(RecordValue { bytes: packet_info.bytes, first_seen: now, last_seen: now });
                                        }
                                    }
                                    Err(e) => panic!("Weirdshark is unable to receive packet: {}", e),
                                }
                            }
                            WorkerCommand::WriteFile => {
                                self.write_csv().unwrap();
                            }
                        }
                    Err(_) => break,
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
            Ok(_) => panic!("Weirdshark: unhandled channel type"),
            Err(e) => panic!("Weirdshark: unable to create channel: {}", e),
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
