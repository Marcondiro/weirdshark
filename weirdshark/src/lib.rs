use std::error::Error;
use serde::Serialize;
use chrono::{DateTime, Utc};

#[derive(Serialize)]
enum TransportProtocols {
    TCP,
    UDP,
}

#[derive(Serialize)]
struct Record {
    source_ip: std::net::IpAddr,
    destination_ip: std::net::IpAddr,
    transport_protocol: TransportProtocols,
    source_port: u16,
    destination_port: u16,
    bytes: u64,
    first_seen: DateTime<Utc>,
    last_seen: DateTime<Utc>,
}

pub fn test_save_csv() -> Result<(), Box<dyn Error>> {
    let mut wtr = csv::Writer::from_path("test.csv")?;

    wtr.serialize(Record {
        source_ip: std::net::IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 1)),
        destination_ip: std::net::IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 1)),
        transport_protocol: TransportProtocols::TCP,
        source_port: 80,
        destination_port: 80,
        bytes: 500,
        first_seen: Utc::now(),
        last_seen: Utc::now(),
    })?;
    wtr.flush()?;
    Ok(())
}
