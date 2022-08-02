use clap::Parser;
use std::net::IpAddr;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
pub struct Args {
    /// Network interface to capture from
    #[clap(value_parser)]
    pub interface: String,

    /// Output path, including file name
    #[clap(short, long, value_parser, default_value = "weirdshark_capture")]
    pub path: String,

    /// Time interval in seconds after which a new report is generated
    /// (if not provided, only one report at the end is generated)
    #[clap(short, long, value_parser)]
    pub time_interval: Option<usize>,

    /// Filter by source ip
    #[clap(short, long, value_parser)]
    pub source_ip: Option<IpAddr>,

    /// Filter by destination ip
    #[clap(short, long, value_parser)]
    pub destination_ip: Option<IpAddr>,

    /// Filter by transport protocol
    #[clap(long, value_enum)]
    pub transport_protocol: Option<TransportProtocol>,

    /// Filter by source port
    #[clap(long, value_parser)]
    pub source_port: Option<u16>,

    /// Filter by destination port
    #[clap(long, value_parser)]
    pub destination_port: Option<u16>,
}

#[derive(clap::ValueEnum, Clone, Debug)]
pub enum TransportProtocol {
    TCP,
    UDP,
}
