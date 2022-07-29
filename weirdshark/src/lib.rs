use std::collections::HashMap;
use std::error::Error;
use std::net::IpAddr;
use serde::Serialize;
use chrono::{DateTime, Utc};

use pnet::datalink::{interfaces, NetworkInterface, channel};
use pnet::datalink::Channel::Ethernet;
use pnet::packet::ethernet::{EtherTypes, EthernetPacket};
use pnet::packet::ip::{IpNextHeaderProtocol, IpNextHeaderProtocols};
use pnet::packet::Packet;
use pnet::packet::ipv4::Ipv4Packet;
use pnet::packet::ipv6::Ipv6Packet;
use pnet::packet::tcp::TcpPacket;
use pnet::packet::udp::UdpPacket;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
enum TransportProtocols {
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

fn write_csv(map: HashMap<RecordKey, RecordValue>) -> Result<(), Box<dyn Error>> {
    let mut wtr = csv::Writer::from_path("test.csv")?;

    for (k, v) in map.into_iter() {
        let record = Record::from_key_value(k, v);
        wtr.serialize(record)?;
    }

    wtr.flush()?;
    Ok(())
}

pub fn capture(interface: String) -> Result<(), String> {
    // Find the network interface with the provided name
    let interface = interfaces().into_iter()
        .filter(|i: &NetworkInterface| i.name == interface)
        .next()
        .expect("Network interface not found"); // TODO manage this with errors

    // No need for custom config, promiscuous by default
    let (_, mut rx) = match channel(&interface, Default::default()) {
        Ok(Ethernet(tx, rx)) => (tx, rx),
        Ok(_) => panic!("packetdump: unhandled channel type"), //TODO manage with errors
        Err(e) => panic!("packetdump: unable to create channel: {}", e),
    };

    let mut map = HashMap::new();

    // TODO support scheduled file generation and quit. Now capture only first 100 frames for test
    for _ in 0..100 {
        match rx.next() {
            Ok(packet) => {
                handle_ethernet_frame(&EthernetPacket::new(packet).unwrap(), &mut map);
            }
            Err(e) => panic!("packetdump: unable to receive packet: {}", e),
        }
    }

    write_csv(map).unwrap();

    Ok(())
}

fn handle_udp_packet(source: IpAddr, destination: IpAddr, packet: &[u8], map: &mut HashMap<RecordKey, RecordValue>) {
    let udp = UdpPacket::new(packet);

    if let Some(udp) = udp {
        let k = RecordKey {
            source_ip: source,
            destination_ip: destination,
            transport_protocol: TransportProtocols::UDP,
            source_port: udp.get_source(),
            destination_port: udp.get_destination(),
        };
        let now = Utc::now();
        map.entry(k)
            .and_modify(|v| {
                v.bytes += packet.len();
                v.last_seen = now;
            })
            .or_insert(RecordValue { bytes: packet.len(), first_seen: now, last_seen: now });
    } else {
        println!("Malformed UDP Packet");
    }
}

fn handle_tcp_packet(source: IpAddr, destination: IpAddr, packet: &[u8], map: &mut HashMap<RecordKey, RecordValue>) {
    let tcp = TcpPacket::new(packet);
    if let Some(tcp) = tcp {
        let k = RecordKey {
            source_ip: source,
            destination_ip: destination,
            transport_protocol: TransportProtocols::TCP,
            source_port: tcp.get_source(),
            destination_port: tcp.get_destination(),
        };
        let now = Utc::now();
        map.entry(k)
            .and_modify(|v| {
                v.bytes += packet.len();
                v.last_seen = now;
            })
            .or_insert(RecordValue { bytes: packet.len(), first_seen: now, last_seen: now });
    } else {
        println!("Malformed TCP Packet");
    }
}

fn handle_transport_protocol(
    source: IpAddr,
    destination: IpAddr,
    protocol: IpNextHeaderProtocol,
    packet: &[u8],
    map: &mut HashMap<RecordKey, RecordValue>,
) {
    match protocol {
        IpNextHeaderProtocols::Udp => {
            handle_udp_packet(source, destination, packet, map)
        }
        IpNextHeaderProtocols::Tcp => {
            handle_tcp_packet(source, destination, packet, map)
        }
        _ => return, // Ignore all the rest
    }
}

fn handle_ipv4_packet(ethernet: &EthernetPacket, map: &mut HashMap<RecordKey, RecordValue>) {
    let header = Ipv4Packet::new(ethernet.payload());
    if let Some(header) = header {
        handle_transport_protocol(
            IpAddr::V4(header.get_source()),
            IpAddr::V4(header.get_destination()),
            header.get_next_level_protocol(),
            header.payload(),
            map,
        );
    } else {
        println!("Malformed IPv4 Packet");
    }
}

fn handle_ipv6_packet(ethernet: &EthernetPacket, map: &mut HashMap<RecordKey, RecordValue>) {
    let header = Ipv6Packet::new(ethernet.payload());
    if let Some(header) = header {
        handle_transport_protocol(
            IpAddr::V6(header.get_source()),
            IpAddr::V6(header.get_destination()),
            header.get_next_header(),
            header.payload(),
            map,
        );
    } else {
        println!("Malformed IPv6 Packet");
    }
}

fn handle_ethernet_frame(ethernet: &EthernetPacket, map: &mut HashMap<RecordKey, RecordValue>) {
    match ethernet.get_ethertype() {
        EtherTypes::Ipv4 => handle_ipv4_packet(ethernet, map),
        EtherTypes::Ipv6 => handle_ipv6_packet(ethernet, map),
        _ => return, // Ignore non-ip traffic
    }
}
