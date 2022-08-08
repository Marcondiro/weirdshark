use std::collections::LinkedList;
use std::net::IpAddr;
use std::path::{Path, PathBuf};
use std::time;
use pnet::datalink;
use pnet::datalink::NetworkInterface;
use crate::capturer::{Capturer, CapturerWorker};
use crate::{get_interface_by_description, get_interface_by_name};
use crate::filters::{IpFilter, DirectionFilter};
use crate::error::WeirdsharkError;

pub struct CapturerBuilder {
    interface: Option<NetworkInterface>,
    report_path: PathBuf,
    report_name_prefix: String,
    report_interval: Option<time::Duration>,
    ip_filters: LinkedList<(IpFilter, DirectionFilter)>,
    //l3_filters : LinkedList<Filter<Ipv4Packet>>,
    //TODO filters
}

impl Default for CapturerBuilder {
    fn default() -> Self {
        let interface = datalink::interfaces().into_iter()
            .filter(|i| !i.is_loopback()
                && i.is_up()
                && if cfg!(unix) { i.is_running() } else { true } // is_running available only under unix
                && !i.ips.is_empty())
            .next()
            .expect("Weirdshark cannot find a default interface.");
        CapturerBuilder::new().interface(interface)
    }
}

impl CapturerBuilder {
    pub fn new() -> Self {
        Self {
            interface: None,
            report_path: PathBuf::new(),
            report_name_prefix: "weirdshark_capture".to_string(),
            report_interval: None,
            ip_filters: LinkedList::new(),
            //l3_filters : LinkedList::new(),
        }
    }

    //TODO: this should check that all Configs are correct in setters

    fn interface(mut self, interface: NetworkInterface) -> Self {
        self.interface = Some(interface);
        self
    }

    pub fn interface_by_name(mut self, name: &str) -> Self {
        self.interface = get_interface_by_name(name);
        //.expect("Network interface not found"); // TODO: manage this with errors?
        self
    }

    pub fn interface_by_description(mut self, name: &str) -> Self {
        self.interface = get_interface_by_description(name);
        //.expect("Network interface not found"); // TODO: manage this with errors?
        self
    }

    pub fn interface_by_index(mut self, index: u32) -> Self {
        let interface = datalink::interfaces().into_iter()
            .filter(|i| i.index == index)
            .next();
        //.expect("Network interface not found"); // TODO: manage this with errors
        self.interface = interface;
        self
    }

    pub fn report_path(mut self, path: &Path) -> Self {
        self.report_path = PathBuf::from(path);
        self
    }

    pub fn report_interval(mut self, duration: Option<time::Duration>) -> Self {
        self.report_interval = duration;
        self
    }

    pub fn add_filter_ip_addr(mut self, addr: IpAddr) -> Self {
        let filter = IpFilter::Single(addr);
        self.ip_filters.push_back((filter, DirectionFilter::Both));
        self
    }

    pub fn build(self) -> Result<Capturer, WeirdsharkError> {
        if self.interface.is_none() {
            return Err(WeirdsharkError::GenericError); //TODO refine error ?
        }

        let capturer_worker = CapturerWorker::new(
            self.interface.unwrap(),
            self.report_path,
            self.report_name_prefix,
            self.report_interval,
        );
        let worker_sender = capturer_worker.get_sender();
        let worker_thread_handle = capturer_worker.work();
        Ok(Capturer { worker_sender, worker_thread_handle })
    }
}