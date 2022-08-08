use std::collections::HashMap;

use std::mem;
use std::sync::mpsc::{Receiver, Sender};
use std::thread::{JoinHandle};
use std::path::Path;
use std::error::Error;
use std::thread;
use pnet::datalink::{channel, DataLinkReceiver, NetworkInterface};
use pnet::datalink::Channel::Ethernet;
use pnet::packet::ethernet;
use crate::{Record, RecordKey, RecordValue, handle_ethernet_frame};
pub use crate::capturer::builder::CaptureConfig;


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
    thread_handle: JoinHandle<()>,
    sender: Sender<WorkerCommand>,
}

impl Capturer {
    pub fn start(&self) {
        //TODO: manager error
        self.sender.send(WorkerCommand::Start).unwrap();
    }

    pub fn pause(&self) {
        //TODO: manager error
        self.sender.send(WorkerCommand::Pause).unwrap();
    }

    //TODO replace stop with drop implementation
    pub fn stop(self) {
        //TODO: manager error
        self.sender.send(WorkerCommand::WriteFile).unwrap();
        self.sender.send(WorkerCommand::Stop).unwrap();
        match self.thread_handle.join() {
            Ok(_) => {}
            Err(e) => println!("{:?}", e) //TODO manage properly
        }
    }
}

fn capture_thread_fn(cfg: CaptureConfig, sender: Sender<WorkerCommand>, receiver: Receiver<WorkerCommand>) {
    let mut map = HashMap::new();
    let scheduler = match cfg.report_interval {
        Some(interval) => Some(write_scheduler::WriteScheduler::new(interval, sender.clone())),
        None => None,
    };
    let mut is_paused = false;
    PnetCaptureAdapter::new(cfg.interface.as_ref().unwrap(), &sender).capture();

    loop {
        match receiver.recv() {
            Ok(command) =>
                match command {
                    WorkerCommand::Start => {
                        if let Some(s) = &scheduler {
                            s.start();
                        }
                        is_paused = false;
                    }
                    WorkerCommand::Pause => {
                        if let Some(s) = &scheduler {
                            s.stop();
                        }
                        is_paused = true;
                    }
                    WorkerCommand::Stop => break,
                    WorkerCommand::HandlePacket(p) => {
                        if is_paused { continue; }
                        match p {
                            Ok(packet) => {
                                //TODO: Proposal: Change this call stack to TCP using IP using layer2 to retrieve a TCP segment A.
                                handle_ethernet_frame(&ethernet::EthernetPacket::new(&packet).unwrap(), &mut map);
                            }
                            Err(e) => panic!("packetdump: unable to receive packet: {}", e), //TODO manage with errors
                        }
                    }
                    WorkerCommand::WriteFile => {
                        let old_map = mem::take(&mut map);
                        assert_eq!(map.len(), 0);
                        write_csv(old_map, cfg.report_path.as_ref().unwrap())
                            .expect("Weirdshark encountered an error while writing the file"); //TODO manage with errors?
                    }
                }
            Err(_) => todo!(),
        };
    }
}

struct PnetCaptureAdapter {
    worker_sender: Sender<WorkerCommand>,
    pnet_receiver: Box<dyn DataLinkReceiver>,
}

impl PnetCaptureAdapter {
    fn new(interface: &NetworkInterface, sender: &Sender<WorkerCommand>) -> Self {
        let worker_sender = sender.clone();
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

fn write_csv(map: HashMap<RecordKey, RecordValue>, path: &Path) -> Result<(), Box<dyn Error>> {
    let mut writer = csv::Writer::from_path(path)?;

    for (k, v) in map.into_iter() {
        let record = Record::from_key_value(k, v);
        writer.serialize(record)?;
    }

    writer.flush()?;
    Ok(())
}
