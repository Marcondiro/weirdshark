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


pub struct CaptureConfig {
    interface: Option<NetworkInterface>,
    report_path: Option<PathBuf>,
    report_interval: Option<time::Duration>,
    ip_filters: LinkedList<(IpFilter,DirectionFilter)>,
    //l3_filters : LinkedList<Filter<Ipv4Packet>>,
    //TODO filters
}

impl Default for CaptureConfig {
    fn default() -> Self {
        let interface = Some(interfaces().into_iter()
            .filter(|i| !i.is_loopback()
                && i.is_up()
                && is_interface_running(i)
                && !i.ips.is_empty())
            .next()
            .expect("Weirdshark cannot find a default interface."));
        Self {
            interface,
            report_path: Some(PathBuf::from("weirdshark_capture.csv")),
            report_interval: None,
            ip_filters: LinkedList::new(),
            //l3_filters : LinkedList::new(),
        }
    }
}

#[cfg(unix)]
fn is_interface_running(i: &NetworkInterface) -> bool {
    i.is_running()
}

#[cfg(not(unix))]
fn is_interface_running(_i: &NetworkInterface) -> bool {
    return true;
}

impl CaptureConfig {
    pub fn new() -> Self {

        Self {
            interface: None,
            report_path: None,
            report_interval: None,
            ip_filters: LinkedList::new(),
            //l3_filters : LinkedList::new(),
        }
    }

    //TODO: add fn set interface by description for windows

    pub fn set_interface_by_name(mut self, name: &str) -> Self {
        self.interface = get_interface_by_name(name);
            //.expect("Network interface not found"); // TODO: manage this with errors?
        self
    }

    pub fn set_interface_by_description(mut self, name: &str) -> Self {
        self.interface = get_interface_by_description(name);
            //.expect("Network interface not found"); // TODO: manage this with errors?
        self
    }

    pub fn set_interface_by_index(mut self, index: u32) -> Self {
        let interface = interfaces().into_iter()
            .filter(|i| i.index == index)
            .next();
            //.expect("Network interface not found"); // TODO: manage this with errors
        self.interface = interface;
        self
    }

    pub fn set_report_path(mut self, path: PathBuf ) ->Self{
        self.report_path = Some(path);
        self
    }

    pub fn set_report_interval(mut self, duration: Option<time::Duration>) -> Self {
        self.report_interval = duration;
        self
    }

    pub fn add_filter_ip_addr(mut self, addr: IpAddr, ) -> Self {
        let filter = IpFilter::Single(addr);
        self.ip_filters.push_back((filter,DirectionFilter::Both) );
        self
    }

    pub fn build_capturer(self) -> Result<CaptureController, WeirdsharkError> {
        //TODO: this should check thath all Configs are correct
        let (sender, receiver) = std::sync::mpsc::channel();
        let t_sender = sender.clone();
        let thread_handle = std::thread::spawn(move || capture_thread_fn(self, t_sender, receiver));
        Ok(CaptureController { sender, thread_handle })
    }
}

enum WorkerCommand {
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

enum IpFilter {
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

enum DirectionFilter {
    Source,
    Destination,
    Both,
}

#[derive(Debug)]
pub enum WeirdsharkError {
    GenericError,
}

mod write_scheduler {
    use std::sync::{Arc, Condvar, Mutex};
    use std::sync::mpsc::Sender;
    use std::thread;
    use std::time::Duration;
    use crate::capturer::WorkerCommand;

    #[derive(PartialEq)]
    enum WriteSchedulerStatus {
        Start,
        Stop,
    }

    pub(super) struct WriteScheduler {
        status: Arc<(Mutex<WriteSchedulerStatus>, Condvar)>,
    }

    impl WriteScheduler {
        pub(super) fn new(interval: Duration, sender: Sender<WorkerCommand>) -> Self {
            let status = Arc::new((
                Mutex::new(WriteSchedulerStatus::Stop),
                Condvar::new())
            );

            let t_status = status.clone();
            thread::spawn(move || {
                loop {
                    let (mutex, condvar) = t_status.as_ref();
                    let guard = mutex.lock().unwrap();
                    let _ = condvar.wait_while(guard, |g| *g != WriteSchedulerStatus::Start).unwrap();

                    thread::sleep(interval);
                    match sender.send(WorkerCommand::WriteFile) {
                        Ok(_) => continue,
                        Err(_) => break,
                    }
                }
            });

            Self { status }
        }

        pub(super) fn start(&self) {
            let (mutex, condvar) = self.status.as_ref();
            let mut guard = mutex.lock().unwrap();
            if *guard == WriteSchedulerStatus::Start {
                panic!("Weirdshark: The file generation scheduler is already running");
            }

            *guard = WriteSchedulerStatus::Start;
            condvar.notify_one();
        }

        pub(super) fn stop(&self) {
            let (mutex, condvar) = self.status.as_ref();
            let mut guard = mutex.lock().unwrap();
            if *guard == WriteSchedulerStatus::Stop {
                panic!("Weirdshark: The file generation scheduler is already stopped");
            }

            *guard = WriteSchedulerStatus::Stop;
            condvar.notify_one();
        }
    }
}
