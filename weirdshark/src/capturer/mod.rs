use std::collections::HashMap;

use std::mem;
use std::sync::mpsc::{Receiver, Sender};
use std::thread::{JoinHandle};
use pnet::datalink::{channel, NetworkInterface};
use pnet::datalink::Channel::Ethernet;
use crate::{write_csv, RecordKey, RecordValue};
pub use crate::capturer::builder::CaptureConfig;
use chrono::Utc;

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
    // let start_time = Utc::now();
    let mut map = HashMap::new();
    let scheduler = match cfg.report_interval {
        Some(interval) => Some(write_scheduler::WriteScheduler::new(interval, sender.clone())),
        None => None,
    };
    let mut is_paused = false;
    pnet_capture_adapter(cfg.interface.as_ref().unwrap(), &sender);

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
                            Ok(data) => {
                                let parse_res = parser::parse_transport_packet(data);
                                if let Ok(packet_info) = parse_res {
                                    let k = RecordKey {
                                        source_ip: packet_info.source_ip,
                                        destination_ip: packet_info.destination_ip,
                                        transport_protocol: packet_info.transport_protocol,
                                        source_port: packet_info.source_port,
                                        destination_port: packet_info.destination_port,
                                    };
                                    let now = Utc::now();
                                    map.entry(k)
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
                        let old_map = mem::take(&mut map);
                        write_csv(old_map, cfg.report_path.as_ref().unwrap())
                            .expect("Weirdshark encountered an error while writing the file"); //TODO manage with errors?
                    }
                }
            Err(_) => todo!(),
        };
    }
}

fn pnet_capture_adapter(interface: &NetworkInterface, sender: &Sender<WorkerCommand>) {
    let t_interface = interface.clone();
    let t_sender = sender.clone();
    std::thread::spawn(move || {
        //TODO: build a pnet config from our config A.
        //Isn't default ok? M.
        let (_, mut receiver) = match channel(&t_interface, Default::default()) {
            Ok(Ethernet(tx, receiver)) => (tx, receiver),
            Ok(_) => panic!("packetdump: unhandled channel type"), //TODO manage with errors
            Err(e) => panic!("packetdump: unable to create channel: {}", e), //TODO manage with errors
        };

        loop {
            let packet = WorkerCommand::HandlePacket(
                match receiver.next() {
                    Ok(p) => Ok(p.to_vec()),
                    Err(e) => Err(e),
                }
            );
            match t_sender.send(packet) {
                Ok(_) => continue,
                Err(_) => break,
            }
        }
    });
}


