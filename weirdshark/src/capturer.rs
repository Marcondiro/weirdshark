pub mod capturer {
    use std::collections::HashMap;
    use std::sync::mpsc::{Receiver, Sender, TryRecvError};
    use std::thread::JoinHandle;
    use chrono::{Duration, Utc};
    use pnet::datalink::{channel, interfaces};
    use pnet::datalink::Channel::Ethernet;
    use pnet::packet::ethernet::{EthernetPacket};
    use crate::{handle_ethernet_frame, NetworkInterface, RecordKey, RecordValue};

    #[derive(PartialEq, PartialOrd)]
    enum CaptureType {
        Time(Duration),
        Packets(u32),
    }

    #[derive(PartialEq)]
    enum Command {
        Pause,
        Go,
        Stop,
    }

    //For now we don't need our struct to be Send
    struct CaptureController {
        thread_handle : JoinHandle<HashMap<RecordKey, RecordValue>>,
        control_chan: Sender<Command>,
    }

    struct CaptureConfig {
        iface: NetworkInterface,
        capture_limit: CaptureType,
    }

    impl CaptureConfig{

        pub fn new() -> Self {
            let interface = interfaces().into_iter()
                .next()
                .expect("Network interface not found"); // TODO: manage this with errors
            Self{iface: interface, capture_limit: CaptureType::Packets(100)}
        }

        pub fn interface_name(mut self, name: &str ) -> Self {
            //TODO: on windows filter for description
            let interface = interfaces().into_iter()
                .filter(|i: &NetworkInterface| i.name == name)
                .next()
                .expect("Network interface not found"); // TODO: manage this with errors
            self.iface = interface;
            self
        }

        pub fn interface_number(mut self, number: usize ) -> Self {
            let interface = interfaces().into_iter()
                .nth(number)
                .expect("Network interface not found"); // TODO: manage this with errors
            self.iface = interface;
            self
        }

        pub fn limit_duration(mut self, dur : Duration) -> Self{
            self.capture_limit = CaptureType::Time(dur);
            self
        }

        pub fn limit_packets(mut self, packs : u32) -> Self{
            self.capture_limit = CaptureType::Packets(packs);
            self
        }

        pub fn build_capture_controller(self) -> CaptureController {
            CaptureController::new(self)
        }
    }
    impl CaptureController {

        pub fn new(cfg: CaptureConfig) -> Self {
            let (control_chan,rx) = std::sync::mpsc::channel();
            let thread_handle = std::thread::spawn(move || capture_thread_fn(cfg, rx));
            Self{control_chan, thread_handle }
        }

        pub fn start(&self) {
            //TODO: manager error
            self.control_chan.send(Command::Go).unwrap();
        }

        pub fn stop(&self) {
            //TODO: manager error
            self.control_chan.send(Command::Stop).unwrap();
        }

        pub fn pause(&self) {
            //TODO: manager error
            self.control_chan.send(Command::Pause).unwrap();
        }


        pub fn get_capture(self) -> HashMap<RecordKey, RecordValue>{
            self.thread_handle.join().unwrap()
        }

        //TODO: add support for try_get_capture
    }

    fn capture_thread_fn(cfg: CaptureConfig, control_rec: Receiver<Command>) -> HashMap<RecordKey, RecordValue> {
        let start_time = Utc::now();
        let mut return_map = HashMap::new();

        //TODO: build a pnet config from our config
        let (_, mut rx) = match channel(&cfg.iface, Default::default()) {
            Ok(Ethernet(tx, rx)) => (tx, rx),
            Ok(_) => panic!("packetdump: unhandled channel type"), //TODO manage with errors
            Err(e) => panic!("packetdump: unable to create channel: {}", e),
        };

        let mut i = match cfg.capture_limit {
            CaptureType::Time(_) => CaptureType::Time(Utc::now() - start_time),
            CaptureType::Packets(_) => CaptureType::Packets(0),
        };
        // TODO support scheduled file generation and quit. Now capture only first 100 frames for test
        let mut chan_state = match control_rec.try_recv() {
            Ok(any) => any,
            Err(TryRecvError::Empty) => Command::Go,
            Err(TryRecvError::Disconnected) => Command::Stop,
        };


        while chan_state != Command::Stop && i < cfg.capture_limit {

            match chan_state {
                Command::Pause => {
                    match control_rec.recv() {
                        Ok(Command::Go) => (),
                        Ok(Command::Pause) => continue,
                        Ok(Command::Stop) => break,
                        Err(_) => break,
                    }
                }
                Command::Go => {
                    match rx.next() {
                        Ok(packet) => {
                            //TODO: Proposal: Change this call stack to TCP using IP using layer2 to retrieve a TCP segment
                            handle_ethernet_frame(&EthernetPacket::new(packet).unwrap(), &mut return_map);
                        }
                        Err(e) => panic!("packetdump: unable to receive packet: {}", e),
                    }
                },
                Command::Stop => unreachable!(),
            };

            //Update of states for next cycle
            chan_state = match control_rec.try_recv() {
                Ok(any) => any,
                Err(TryRecvError::Empty) => chan_state,
                Err(TryRecvError::Disconnected) => Command::Stop,
            };

            i = match cfg.capture_limit {
                CaptureType::Time(_) => CaptureType::Time(Utc::now() - start_time),
                CaptureType::Packets(i_val) => CaptureType::Packets(i_val+1),
            };
        }
        return_map
    }
}