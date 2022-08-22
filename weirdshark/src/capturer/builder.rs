use std::collections::linked_list::LinkedList;
use std::net::IpAddr;
use std::path::{Path, PathBuf};
use std::{fs, time};
use std::error::Error;
use pnet::datalink;
use pnet::datalink::NetworkInterface;
use crate::capturer::{Capturer, CapturerWorker};
use crate::{get_interface_by_description, get_interface_by_name, TransportProtocols};
use crate::filters::{Filter, DirectedFilter};
use crate::error::WeirdsharkError;

pub struct CapturerBuilder {
    interface: Option<NetworkInterface>,
    report_path: PathBuf,
    report_interval: Option<time::Duration>,
    ip_filters: LinkedList<DirectedFilter<IpAddr>>,
    port_filters: LinkedList<DirectedFilter<u16>>,
    transport_protocol: Option<TransportProtocols>,
}

impl Default for CapturerBuilder {
    fn default() -> Self {
        let interface = datalink::interfaces().into_iter()
            .filter(|i| !i.is_loopback()
                && i.is_up()
                && is_interface_running(i) // is_running available only under unix
                && !i.ips.is_empty())
            .next()
            .expect("Weirdshark cannot find a default interface.");
        CapturerBuilder::new().interface(interface)
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

impl CapturerBuilder {
    pub fn new() -> Self {
        Self {
            interface: None,
            report_path: PathBuf::new(),
            report_interval: None,
            ip_filters: LinkedList::new(),
            port_filters: LinkedList::new(),
            transport_protocol: None,
        }
    }

    fn interface(mut self, interface: NetworkInterface) -> Self {
        self.interface = Some(interface);
        self
    }

    pub fn interface_by_name(mut self, name: &str) -> Self {
        self.interface = get_interface_by_name(name);
        self
    }

    pub fn interface_by_description(mut self, name: &str) -> Self {
        self.interface = get_interface_by_description(name);
        self
    }

    pub fn interface_by_index(mut self, index: u32) -> Self {
        let interface = datalink::interfaces().into_iter()
            .filter(|i| i.index == index)
            .next();
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

    pub fn add_directed_filter_ip(mut self, filter: DirectedFilter<IpAddr>) -> Self {
        self.ip_filters.push_back(filter);
        self
    }

    pub fn add_undirected_filter_ip(mut self, filter: Filter<IpAddr>) -> Self {
        self.ip_filters.push_back(DirectedFilter::both_directions(filter));
        self
    }

    pub fn add_directed_filter_port(mut self, filter: DirectedFilter<u16>) -> Self {
        self.port_filters.push_back(filter);
        self
    }

    pub fn add_undirected_filter_port(mut self, filter: Filter<u16>) -> Self {
        self.port_filters.push_back(DirectedFilter::both_directions(filter));
        self
    }

    pub fn add_transport_protocol_filter(mut self, transport_protocol: Option<TransportProtocols>) -> Self {
        self.transport_protocol = transport_protocol;
        self
    }

    pub fn build(self) -> Result<Capturer, Box<dyn Error>> {
        if self.interface.is_none() {
            return Err(WeirdsharkError::InterfaceNotSpecified.into());
        }

        // Create reports directory if doesn't exist
        fs::create_dir_all(&self.report_path)?;

        let capturer_worker = CapturerWorker::new(
            self.interface.unwrap(),
            self.report_path,
            self.report_interval,
            self.ip_filters,
            self.port_filters,
            self.transport_protocol,
        );
        let worker_sender = capturer_worker.get_sender();
        let worker_thread_handle = capturer_worker.work();
        Ok(Capturer { worker_sender, worker_thread_handle })
    }
}