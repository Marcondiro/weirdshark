use std::collections::HashMap;
use std::collections::linked_list::LinkedList;

use std::mem;
use std::net::IpAddr;
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, Sender};
use std::thread::{JoinHandle, sleep};
use std::time;
use pnet::datalink::{channel, interfaces, NetworkInterface};
use pnet::datalink::Channel::Ethernet;
use pnet::packet::ethernet;
use crate::{get_interface_by_description, get_interface_by_name, handle_ethernet_frame, write_csv};
use crate::capturer::config::CaptureConfig;

mod config;

pub enum WorkerCommand {
    Start,
    Pause,
    Stop,
    WriteFile,
    HandlePacket(Result<Vec<u8>, std::io::Error>),
}

pub struct CaptureController {
    thread_handle: JoinHandle<()>,
    sender: Sender<WorkerCommand>,
    //TODO add a status (cannot start if already started)
}

impl CaptureController {
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

    //TODO handle scheduled file generation (time based file generation in a new thread?)

    loop {
        match receiver.recv() {
            Ok(WorkerCommand::Start) => {
                pnet_capture_adapter(cfg.interface.as_ref().unwrap(), &sender);
            }
            Ok(WorkerCommand::Pause) => todo!(),
            Ok(WorkerCommand::Stop) => break,
            Ok(WorkerCommand::HandlePacket(p)) => {
                match p {
                    Ok(packet) => {
                        //TODO: Proposal: Change this call stack to TCP using IP using layer2 to retrieve a TCP segment A.
                        handle_ethernet_frame(&ethernet::EthernetPacket::new(&packet).unwrap(), &mut map);
                    }
                    Err(e) => panic!("packetdump: unable to receive packet: {}", e), //TODO manage with errors
                }
            }
            Ok(WorkerCommand::WriteFile) => {
                let old_map = mem::take(&mut map);
                write_csv(old_map, cfg.report_path.as_ref().unwrap())
                    .expect("Weirdshark encountered an error while writing the file"); //TODO manage with errors?
            }
            Err(_) => todo!(),
        };
    }

    drop(receiver);
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

pub enum IpFilter {
    Single(IpAddr),
    Range(IpAddr, IpAddr),
    List(Vec<IpAddr>),
}

impl IpFilter {
    pub fn filter(&self, x:&IpAddr) -> bool {
        use IpFilter::{Single,Range,List};
        match self {
            Single(addr)=> x == addr,
            Range(start_addr, end_addr) => start_addr <= x && x<=end_addr,
            List(list) => list.iter().any(|ip|{ip == x})
        }
    }
}

pub enum DirectionFilter {
    Source,
    Destination,
    Both,
}

#[derive(Debug)]
pub enum WeirdsharkError {
    GenericError,
}


