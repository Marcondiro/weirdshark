use std::net::IpAddr;
use pnet::packet::Packet;
use pnet::packet::PacketSize;
use pnet::packet::ethernet::{EthernetPacket, EtherTypes};
use pnet::packet::ipv4::Ipv4Packet;
use pnet::packet::ipv6::Ipv6Packet;
use pnet::packet::ip::{IpNextHeaderProtocols};
use pnet::packet::tcp::TcpPacket;
use pnet::packet::udp::UdpPacket;
use crate::error::WeirdsharkError;
use crate::TransportProtocols;

enum InternetProtocol {
    Ipv4,
    Ipv6,
    //DontCare,
}

struct InternetPacket {
    _ip_version: InternetProtocol,
    source: IpAddr,
    destination: IpAddr,
    next_protocol: TransportProtocols,
    payload: Vec<u8>,
}

pub struct TransportPacket {
    pub(super) source_ip: IpAddr,
    pub(super) destination_ip: IpAddr,
    pub(super) transport_protocol: TransportProtocols,
    pub(super) source_port: u16,
    pub(super) destination_port: u16,
    pub(super) bytes: usize,
}

pub fn parse_transport_packet(data: Vec<u8>) -> Result<TransportPacket, WeirdsharkError> {
    use TransportProtocols::{TCP, UDP};
    let ethernet = parse_ethernet_frame(data)?;

    let internet_packet = parse_internet_packet(&ethernet)?;

    let bytes = ethernet.packet_size();
    let (source_port, destination_port) = match internet_packet.next_protocol {
        TCP => {
            let transport_packet = parse_tcp_packet(&internet_packet.payload)?;
            (transport_packet.get_source(), transport_packet.get_destination())
        }
        UDP => {
            let transport_packet = parse_udp_packet(&internet_packet.payload)?;
            (transport_packet.get_source(), transport_packet.get_destination())
        }
    };
    Ok(TransportPacket {
        source_ip: internet_packet.source,
        destination_ip: internet_packet.destination,
        transport_protocol: internet_packet.next_protocol,
        source_port,
        destination_port,
        bytes,
    })
}

fn parse_ethernet_frame(data: Vec<u8>) -> Result<EthernetPacket<'static>, WeirdsharkError> {
    let eth = EthernetPacket::owned(data);
    return match eth {
        Some(frame) => Ok(frame),
        None => Err(WeirdsharkError::IncompleteEthernetFrame),
    };
}

fn parse_internet_packet(ethernet: &EthernetPacket) -> Result<InternetPacket, WeirdsharkError> {
    let (ip_version, source, destination, next_level_protocol, payload) = match ethernet.get_ethertype() {
        EtherTypes::Ipv4 => {
            let ip_packet = parse_ipv4_packet(ethernet)?;
            let source = IpAddr::from(ip_packet.get_source());
            let destination = IpAddr::from(ip_packet.get_destination());
            let next_protocol = ip_packet.get_next_level_protocol();
            let payload = ip_packet.payload().to_vec();
            (InternetProtocol::Ipv4, source, destination, next_protocol, payload)
        }
        EtherTypes::Ipv6 => {
            let ip_packet = parse_ipv6_packet(ethernet)?;
            let source = IpAddr::from(ip_packet.get_source());
            let destination = IpAddr::from(ip_packet.get_destination());
            let next_protocol = ip_packet.get_next_header();
            let payload = ip_packet.payload().to_vec();
            (InternetProtocol::Ipv6, source, destination, next_protocol, payload)
        }
        _ => return Err(WeirdsharkError::PacketIgnoredNonIp), // Ignore non-ip traffic
    };
    let next_protocol = match next_level_protocol {
        IpNextHeaderProtocols::Tcp => TransportProtocols::TCP,
        IpNextHeaderProtocols::Udp => TransportProtocols::UDP,
        _ => return Err(WeirdsharkError::UnsupportedTransportProtocol),
    };
    Ok(InternetPacket {
        _ip_version: ip_version,
        source,
        destination,
        next_protocol,
        payload,
    })
}

fn parse_ipv4_packet<'p>(ethernet: &'p EthernetPacket<'p>) -> Result<Ipv4Packet<'p>, WeirdsharkError> {
    let header = Ipv4Packet::new(ethernet.payload());
    return match header {
        Some(header) => Ok(header),
        None => Err(WeirdsharkError::IncompleteIpPacket),
    };
}

fn parse_ipv6_packet<'p>(ethernet: &'p EthernetPacket<'p>) -> Result<Ipv6Packet<'p>, WeirdsharkError> {
    let header = Ipv6Packet::new(ethernet.payload());
    return match header {
        Some(header) => Ok(header),
        None => Err(WeirdsharkError::IncompleteIpPacket),
    };
}

fn parse_tcp_packet(data: &[u8]) -> Result<TcpPacket, WeirdsharkError> {
    return match TcpPacket::new(data) {
        Some(segment) => Ok(segment),
        None => Err(WeirdsharkError::IncompleteTcpSegment),
    };
}

fn parse_udp_packet(data: &[u8]) -> Result<UdpPacket, WeirdsharkError> {
    return match UdpPacket::new(data) {
        Some(segment) => Ok(segment),
        None => Err(WeirdsharkError::IncompleteUdpSegment),
    };
}