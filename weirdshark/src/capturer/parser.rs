use std::net::IpAddr;
use pnet::packet::ethernet::{EthernetPacket, EtherTypes};
use pnet::packet::ipv4::Ipv4Packet;
use pnet::packet::ipv6::Ipv6Packet;
use pnet::packet::tcp::TcpPacket;
use pnet::packet::udp::UdpPacket;


#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
enum TransportProtocols {
    TCP,
    UDP,
}

enum InternetPacket{
    Ipv4(Ipv4Packet),
    Ipv6(Ipv6Packet),
    DontCare,
}

pub struct TransportPacket{
    source_ip: IpAddr,
    destination_ip: IpAddr,
    transport_protocol: TransportProtocols,
    source_port: u16,
    destination_port: u16,
    bytes: usize,
    ethernet_packet: EthernetPacket,
}

pub fn parse_transport_packet(ethernet: &EthernetPacket) -> Option<TransportPacket>{

    None
}

fn parse_internet_packet(ethernet: &EthernetPacket) -> InternetPacket {
    return match ethernet.get_ethertype() {
        EtherTypes::Ipv4 => InternetPacket::Ipv4(Ipv4Packet::new(ethernet.payload())),
        EtherTypes::Ipv6 => InternetPacket::Ipv6(Ipv6Packet::new(ethernet.payload())),
        _ => InternetPacket::DontCare, // Ignore non-ip traffic
    }
}

fn parse_ipv4_packet(ethernet: &EthernetPacket) -> Ipv4Packet {
    let header = Ipv4Packet::new(ethernet.payload());
    if let Some(header) = header {
         
    }

}