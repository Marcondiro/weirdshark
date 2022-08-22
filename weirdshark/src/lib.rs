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
use chrono::{DateTime, Utc};
use pnet::datalink::interfaces;
pub use pnet::datalink::{NetworkInterface};
use crate::capturer::parser::TransportProtocols;

pub mod capturer;
pub mod filters;
pub mod error;

#[derive(Serialize)]
struct Record {
    source_ip: IpAddr,
    destination_ip: IpAddr,
    transport_protocol: TransportProtocols,
    source_port: u16,
    destination_port: u16,
    bytes: usize,
    first_seen: DateTime<Utc>,
    last_seen: DateTime<Utc>,
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
    first_seen: DateTime<Utc>,
    last_seen: DateTime<Utc>,
}

pub fn get_interfaces() -> Vec<NetworkInterface> {
    interfaces()
}

pub fn get_interface_by_name(name: &str) -> Option<NetworkInterface> {
    interfaces().into_iter()
        .filter(|i: &NetworkInterface| i.name == name)
        .next()
}

pub fn get_interface_by_description(description: &str) -> Option<NetworkInterface> {
    interfaces().into_iter()
        .filter(|i: &NetworkInterface| i.description == description)
        .next()
}
