//! # weirdshark
//!
//! `weirdshark` is a cross-platform library capable of **intercepting** incoming and outgoing
//! **traffic** through the network interfaces.
//!
//! The library allows to collect IP address, port and protocol type of observed traffic and will
//! generate a **report in csv format**.
//!
//! The report lists for each of the network address/port pairs that have been observed, the
//! protocols that was transported, the cumulated number of bytes transmitted, the timestamp of the
//! first and last occurrence of information exchange.
//!
//! Through CapturerBuilder's parameters it is possible to specify the network adapter to be
//! inspected, the output file to be generated, the time interval after which a new report is
//! generated and filters to apply to captured data.

use std::net::IpAddr;
use serde::Serialize;
use pnet::datalink::interfaces;
use pnet::datalink::{NetworkInterface};

pub use capturer::{Capturer,CapturerBuilder};
pub use filters::{Filter,DirectedFilter};
pub use error::WeirdsharkError;

mod capturer;
mod filters;
mod error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
pub enum TransportProtocols {
    TCP,
    UDP,
}

#[derive(Serialize)]
struct Record {
    source_ip: IpAddr,
    destination_ip: IpAddr,
    transport_protocol: TransportProtocols,
    source_port: u16,
    destination_port: u16,
    bytes: usize,
    first_seen: chrono::DateTime<chrono::Local>,
    last_seen: chrono::DateTime<chrono::Local>,
}

impl Record {
    fn from_key_value(k: RecordKey, v: RecordValue) -> Self {
        Self {
            source_ip: k.source_ip,
            destination_ip: k.destination_ip,
            transport_protocol: k.transport_protocol,
            source_port: k.source_port,
            destination_port: k.destination_port,
            bytes: v.bytes,
            first_seen: v.first_seen,
            last_seen: v.last_seen,
        }
    }
}

#[derive(PartialEq, Eq, Hash, Serialize, Debug)]
struct RecordKey {
    source_ip: IpAddr,
    destination_ip: IpAddr,
    transport_protocol: TransportProtocols,
    source_port: u16,
    destination_port: u16,
}

#[derive(Serialize, Debug)]
struct RecordValue {
    bytes: usize,
    first_seen: chrono::DateTime<chrono::Local>,
    last_seen: chrono::DateTime<chrono::Local>,
}

///Retrieve the list of Network Interfaces detected on your PC, in the same format as `pnet::NetworkInterface`
pub fn get_interfaces() -> Vec<NetworkInterface> {
    interfaces()
}

///Retrieve, if any, the network interface called `name`
pub fn get_interface_by_name(name: &str) -> Option<NetworkInterface> {
    interfaces().into_iter()
        .filter(|i: &NetworkInterface| i.name == name)
        .next()
}

///Retrieve, if any, the network interface with given `description`, useful on Windows hosts, where Interface name is just an UUID
pub fn get_interface_by_description(description: &str) -> Option<NetworkInterface> {
    interfaces().into_iter()
        .filter(|i: &NetworkInterface| i.description == description)
        .next()
}

///Retrieve, if any, the network interface with given `index`, useful on Windows hosts, where Interface name is just an UUID
pub fn get_interface_by_index(index: u32) -> Option<NetworkInterface> {
    interfaces().into_iter()
        .filter(|i: &NetworkInterface| i.index == index)
        .next()
}