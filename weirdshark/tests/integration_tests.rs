use std::net::IpAddr;
use weirdshark::*;
use pnet::datalink::channel;
use pnet::datalink::Channel::Ethernet;

#[test]
fn packet_is_recorded() {
    // a sample DNS query for www.mozilla.org
    let packet = [0x08, 0x00, 0x45, 0x00, 0x00, 0x48, 0x5a, 0x7d, 0x40, 0x00, 0x40, 0x11,
        0xa3, 0xb2, 0xac, 0x10, 0x85, 0xfb, 0x0a, 0x60, 0x00, 0x0a, 0x93, 0xc2, 0x00, 0x35, 0x00,
        0x34, 0x3c, 0xbb, 0x82, 0xad, 0x01, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01,
        0x03, 0x77, 0x77, 0x77, 0x07, 0x6d, 0x6f, 0x7a, 0x69, 0x6c, 0x6c, 0x61, 0x03, 0x6f, 0x72,
        0x67, 0x00, 0x00, 0x01, 0x00, 0x01, 0x00, 0x00, 0x29, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00,
    ];
    let source_ip = [172u8, 16, 133, 251];
    let dest_ip = [10u8, 96, 0, 10];
    let source_port = 37826;
    let dest_port = 53;
    let transport_protocol = TransportProtocols::UDP;

    let path = "./tests".as_ref();
    let capturer_builder = CapturerBuilder::default()
        .report_path(path)
        .add_directed_filter_ip(DirectedFilter::only_source(
            Filter::from_vec([IpAddr::from(source_ip)].to_vec()))
        )
        .add_directed_filter_ip(DirectedFilter::only_destination(
            Filter::from_vec([IpAddr::from(dest_ip)].to_vec()))
        )
        .add_directed_filter_port(DirectedFilter::only_source(
            Filter::from_vec([source_port].to_vec())
        ))
        .add_directed_filter_port(DirectedFilter::only_destination(
            Filter::from_vec([dest_port].to_vec())
        ))
        .add_transport_protocol_filter(Some(transport_protocol));
    let interface = capturer_builder.get_interface().unwrap();

    let mac = interface.mac.unwrap().octets();
    let mut frame = Vec::new();
    frame.extend_from_slice(&mac); // destination MAC
    frame.extend_from_slice(&mac); // source MAC
    frame.extend_from_slice(&packet);

    let mut pnet_sender = match channel(&interface, Default::default()) {
        Ok(Ethernet(sender, _)) => sender,
        Ok(_) => panic!("Unhandled channel type"),
        Err(e) => panic!("Unable to create channel: {}", e),
    };

    let capturer = capturer_builder.build().unwrap();
    capturer.start().unwrap();
    std::thread::sleep(std::time::Duration::from_millis(500));
    pnet_sender.send_to(&frame, None).unwrap().unwrap();
    std::thread::sleep(std::time::Duration::from_millis(500));
    capturer.stop().unwrap();

    let paths = std::fs::read_dir(path).unwrap();
    let report = paths.filter(
        |p| match p {
            Ok(f) => f.file_name().to_str().unwrap().ends_with(".csv"),
            _ => false
        }
    ).next().unwrap().unwrap().path();

    let mut rdr = csv::Reader::from_path(&report).unwrap();
    let record = rdr.deserialize().filter(
        |res: &Result<Record, csv::Error>| match res {
            Ok(r) => r.source_ip == IpAddr::from(source_ip) &&
                r.destination_ip == IpAddr::from(dest_ip) &&
                r.transport_protocol == TransportProtocols::UDP &&
                r.source_port == source_port &&
                r.destination_port == dest_port,
            _ => panic!("Cannot deserialize csv"),
        }
    ).next().expect("Packet not recorded.").unwrap();

    std::fs::remove_file(&report).unwrap();
}
