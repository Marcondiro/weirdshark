use clap::Parser;
use std::net::IpAddr;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
pub struct Args {
    /// Network interface name to capture from
    #[clap(value_parser)]
    pub interface_name: Option<String>,

    ///Select the capturing network interface interface by its index
    ///this option overrides any interface_name added
    #[clap(short = 'i', long)]
    pub interface_index: Option<u32>,

    ///Select the capturing network interface interface by its description
    ///this option overrides any interface_name added
    #[clap(short = 'd', long)]
    pub interface_desc: Option<String>,

    /// Output path, including file name
    #[clap(short = 'o', long, value_parser, default_value = "./")]
    pub path: String,

    /// Print interface list
    #[clap(short, long)]
    pub list_interfaces: bool,

    /// Time interval in seconds after which a new report is generated
    /// (if not provided, only one report at the end is generated)
    #[clap(short, long, value_parser)]
    pub time_interval: Option<u64>,

    /// Filter by ip
    #[clap(long, value_parser)]
    pub both_ip: Option<IpAddr>,

    /// Filter by source ip
    #[clap(long, value_parser)]
    pub source_ip: Option<IpAddr>,

    /// Filter by destination ip
    #[clap(long, value_parser)]
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
