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

/// Capturer builder.
///
/// Struct used to create a Capturer following the builder pattern.
/// Exposes different methods to set up the capture and a build method.
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
                && (cfg!(windows) || i.is_up())
                && is_interface_running(i) // is_running available only under unix
                && i.mac.is_some()
                && !i.ips.is_empty())
            .reduce(|a, b| if a.ips.len() > b.ips.len() { a } else { b }) // get interface with most ips
            .expect(&format!("Weirdshark cannot find a default interface. {:?}",datalink::interfaces()));
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
    /// Construct a new CapturerBuilder
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

    /// Set the interface to capture from providing its name
    pub fn interface_by_name(mut self, name: &str) -> Self {
        self.interface = get_interface_by_name(name);
        self
    }

    /// Set the interface to capture from providing its description
    pub fn interface_by_description(mut self, name: &str) -> Self {
        self.interface = get_interface_by_description(name);
        self
    }

    /// Set the interface to capture from providing its index
    pub fn interface_by_index(mut self, index: u32) -> Self {
        let interface = datalink::interfaces().into_iter()
            .filter(|i| i.index == index)
            .next();
        self.interface = interface;
        self
    }

    /// Set the path where to save reports
    pub fn report_path(mut self, path: &Path) -> Self {
        self.report_path = PathBuf::from(path);
        self
    }

    /// Set the timeout after which a report is generated
    pub fn report_interval(mut self, duration: Option<time::Duration>) -> Self {
        self.report_interval = duration;
        self
    }

    ///Add a filter on Ip address. Note that more filters of the same kind will be placed in logic OR while filters of different kinds will be placed in logic AND.
    ///Example: filter on IpAddr: 10.12.0.1, (192.168.0.1,192.168.0.255) will accept either packets from 10.12.0.1 or any among 192.168.0.0/24, but if we add the TCP filter
    ///will accept only TCP traffic on given ip addresses
    pub fn add_directed_filter_ip(mut self, filter: DirectedFilter<IpAddr>) -> Self {
        self.ip_filters.push_back(filter);
        self
    }

    ///Add an undirected filter on Ip address. Note that more filters of the same kind will be placed in logic OR while filters of different kinds will be placed in logic AND.
    ///Example: filter on IpAddr: 10.12.0.1, (192.168.0.1,192.168.0.255) will accept either packets from 10.12.0.1 or any among 192.168.0.0/24, but if we add the TCP filter
    ///will accept only TCP traffic on given ip addresses
    pub fn add_undirected_filter_ip(mut self, filter: Filter<IpAddr>) -> Self {
        self.ip_filters.push_back(DirectedFilter::both_directions(filter));
        self
    }

    ///Add a filter on ports. Note that more filters of the same kind will be placed in logic OR while filters of different kinds will be placed in logic AND.
    ///Example: filter on ports: 1024, (443-500) will accept either packets from 1024 or any in the range 443-500 , but if we add the UDP filter
    ///will accept only UDP traffic on given ports
    pub fn add_directed_filter_port(mut self, filter: DirectedFilter<u16>) -> Self {
        self.port_filters.push_back(filter);
        self
    }

    ///Add a filter on ports. Note that more filters of the same kind will be placed in logic OR while filters of different kinds will be placed in logic AND.
    ///Example: filter on ports: 1024, (443-500) will accept either packets from 1024 or any in the range 443-500 , but if we add the UDP filter
    ///will accept only UDP traffic on given ports
    pub fn add_undirected_filter_port(mut self, filter: Filter<u16>) -> Self {
        self.port_filters.push_back(DirectedFilter::both_directions(filter));
        self
    }

    ///Add a filter on which transport protocol capture.
    pub fn add_transport_protocol_filter(mut self, transport_protocol: Option<TransportProtocols>) -> Self {
        self.transport_protocol = transport_protocol;
        self
    }

    /// Generate the Capturer
    pub fn build(&self) -> Result<Capturer, Box<dyn Error>> {
        if self.interface.is_none() {
            return Err(WeirdsharkError::InterfaceNotSpecified.into());
        }

        // Create reports directory if doesn't exist
        fs::create_dir_all(&self.report_path)?;

        let capturer_worker = CapturerWorker::new(
            self.interface.as_ref().unwrap().clone(),
            self.report_path.clone(),
            self.report_interval,
            self.ip_filters.clone(),
            self.port_filters.clone(),
            self.transport_protocol,
        );
        let worker_sender = capturer_worker.get_sender();
        let worker_thread_handle = capturer_worker.work();
        Ok(Capturer { worker_sender, worker_thread_handle })
    }
}

#[cfg(test)]
mod tests {
    use crate::capturer::CapturerBuilder;
    use crate::error::WeirdsharkError;

    #[test]
    fn default_builder_is_valid() {
        let builder = CapturerBuilder::default();
        let capturer = builder.build();
        assert!(capturer.is_ok());
    }

    #[test]
    fn build_fails_with_no_interface() {
        let builder = CapturerBuilder::new();
        let capturer = builder.build();
        assert!(capturer.is_err());
        if let Err(e) = capturer {
            assert_eq!(*e.downcast::<WeirdsharkError>().unwrap(), WeirdsharkError::InterfaceNotSpecified);
        }
    }
}
