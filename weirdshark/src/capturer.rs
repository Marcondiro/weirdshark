use std::collections::HashMap;
use std::mem;
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, Sender};
use std::thread::JoinHandle;
use chrono::Duration;
use pnet::datalink::{channel, interfaces};
use pnet::datalink::Channel::Ethernet;
use pnet::packet::ethernet::EthernetPacket;
use crate::{handle_ethernet_frame, NetworkInterface, write_csv};

pub struct CaptureConfig {
    interface: NetworkInterface,
    report_path: PathBuf,
    report_interval: Option<Duration>,
    //TODO filters
}

impl Default for CaptureConfig {
    fn default() -> Self {
        let interface = interfaces().into_iter()
            .filter(|i| !i.is_loopback() && i.is_up() && i.is_running())
            .next().expect("Weirdshark cannot find a default interface.");
        Self {
            interface,
            report_path: PathBuf::from("weirdshark_capture.csv"),
            report_interval: None,
        }
    }
}

impl CaptureConfig {
    pub fn new(interface_name: &str, report_path: &str, report_interval_seconds: Option<i64>) -> Self {
        let interface = CaptureConfig::get_interface_by_name(interface_name)
            .expect("Network interface not found");
        // TODO negative duration check
        let report_path = PathBuf::from(report_path);
        let report_interval = match report_interval_seconds {
            Some(s) => Some(Duration::seconds(s)),
            None => None,
        };
        Self {
            interface,
            report_path,
            report_interval,
        }
    }

    fn get_interface_by_name(name: &str) -> Option<NetworkInterface> {
        interfaces().into_iter()
            .filter(|i: &NetworkInterface| i.name == name)
            .next()
    }

    //TODO: add fn set interface by description for windows

    pub fn set_interface_by_name(&mut self, name: &str) {
        self.interface = CaptureConfig::get_interface_by_name(name)
            .expect("Network interface not found"); // TODO: manage this with errors?
    }

    pub fn set_interface_by_number(&mut self, number: usize) {
        let interface = interfaces().into_iter()
            .nth(number)
            .expect("Network interface not found"); // TODO: manage this with errors
        self.interface = interface;
    }

    pub fn set_report_interval(&mut self, seconds: Option<i64>) {
        // TODO negative duration check
        self.report_interval = match seconds {
            Some(s) => Some(Duration::seconds(s)),
            None => None,
        };
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
    pub fn new(cfg: CaptureConfig) -> Self {
        let (sender, receiver) = std::sync::mpsc::channel();
        let t_sender = sender.clone();
        let thread_handle = std::thread::spawn(move || capture_thread_fn(cfg, t_sender, receiver));
        Self { sender, thread_handle }
    }

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
                pnet_capture_adapter(&cfg.interface, &sender);
            }
            Ok(WorkerCommand::Pause) => todo!(),
            Ok(WorkerCommand::Stop) => break,
            Ok(WorkerCommand::HandlePacket(p)) => {
                match p {
                    Ok(packet) => {
                        //TODO: Proposal: Change this call stack to TCP using IP using layer2 to retrieve a TCP segment A.
                        handle_ethernet_frame(&EthernetPacket::new(&packet).unwrap(), &mut map);
                    }
                    Err(e) => panic!("packetdump: unable to receive packet: {}", e), //TODO manage with errors
                }
            }
            Ok(WorkerCommand::WriteFile) => {
                let old_map = mem::take(&mut map);
                write_csv(old_map, &cfg.report_path)
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
