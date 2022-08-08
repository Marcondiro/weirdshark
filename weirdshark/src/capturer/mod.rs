use std::collections::HashMap;

use std::mem;
use std::sync::mpsc::{Receiver, Sender};
use std::thread::{JoinHandle};
use std::path::PathBuf;
use std::error::Error;
use std::thread;
use std::time::Duration;
use pnet::datalink::{channel, DataLinkReceiver, NetworkInterface};
use pnet::datalink::Channel::Ethernet;
use pnet::packet::ethernet;
use crate::{Record, RecordKey, RecordValue, handle_ethernet_frame};
pub use crate::capturer::builder::CaptureConfig;
use crate::capturer::write_scheduler::WriteScheduler;


pub mod builder;
mod write_scheduler;

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
    is_paused: bool,
    interface: NetworkInterface,
}

impl CapturerWorker {
    fn new(interface: NetworkInterface, report_path: PathBuf, report_interval: Option<Duration>) -> Self {
        let (sender, receiver) = std::sync::mpsc::channel();
        let map = HashMap::new();
        let report_scheduler = match report_interval {
            Some(interval) => Some(write_scheduler::WriteScheduler::new(interval, sender.clone())),
            None => None,
        };
        let is_paused = false;

        Self { sender, receiver, map, report_scheduler, report_path, is_paused, interface }
    }

    fn get_sender(&self) -> Sender<WorkerCommand> {
        self.sender.clone()
    }

    fn write_csv(&mut self) -> Result<(), Box<dyn Error>> {
        let file_name = chrono::Utc::now().to_string() + ".csv"; //TODO manage prefix parameter
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
                                    Ok(packet) => {
                                        //TODO: Proposal: Change this call stack to TCP using IP using layer2 to retrieve a TCP segment A.
                                        handle_ethernet_frame(&ethernet::EthernetPacket::new(&packet).unwrap(), &mut self.map);
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
