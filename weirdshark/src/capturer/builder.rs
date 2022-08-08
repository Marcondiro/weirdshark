use std::collections::LinkedList;
use std::net::IpAddr;
use std::path::PathBuf;
use std::time;
use pnet::datalink;
use pnet::datalink::NetworkInterface;
use crate::capturer::{Capturer, CapturerWorker};
use crate::{get_interface_by_description, get_interface_by_name};
use crate::filters::{IpFilter, DirectionFilter};
use crate::error::WeirdsharkError;

pub struct CaptureConfig {
    pub(crate) interface: Option<NetworkInterface>,
    pub(crate) report_path: Option<PathBuf>,
    pub(crate) report_interval: Option<time::Duration>,
    pub(crate) ip_filters: LinkedList<(IpFilter, DirectionFilter)>,
    //l3_filters : LinkedList<Filter<Ipv4Packet>>,
    //TODO filters
}

impl Default for CaptureConfig {
    fn default() -> Self {
        let interface = Some(datalink::interfaces().into_iter()
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

    pub fn report_path(mut self, path: PathBuf) -> Self {
        self.report_path = Some(path);
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
        //TODO: this should check that all Configs are correct
        let capturer_worker = CapturerWorker::new(
            &self.interface.unwrap(),
            self.report_path.unwrap(),
            self.report_interval,
        );
        let worker_sender = capturer_worker.get_sender();
        let worker_thread_handle = capturer_worker.work();
        Ok(Capturer { worker_sender, worker_thread_handle })
    }
}