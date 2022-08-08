use clap::Parser;
use std::net::IpAddr;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
pub struct Args {
    /// Network interface name to capture from
    #[clap()]
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
    #[clap(short = 'o', long, default_value = "./")]
    pub path: String,

    /// Print interface list
    #[clap(short, long)]
    pub list_interfaces: bool,

    /// Time interval in seconds after which a new report is generated
    /// (if not provided, only one report at the end is generated)
    #[clap(short, long)]
    pub time_interval: Option<u64>,

    /// Filter by ip, insert IPs to include in the report
    /// Packets which source OR destination IP is in the list are recorded
    #[clap(long, multiple_values = true)]
    pub ips: Vec<IpAddr>,

    /// Filter by source ip, insert IPs to include in the report
    #[clap(long, multiple_values = true)]
    pub source_ips: Vec<IpAddr>,

    /// Filter by destination ip, insert IPs to include in the report
    #[clap(long, multiple_values = true)]
    pub destination_ips: Vec<IpAddr>,

    /// Filter by transport protocol
    #[clap(long, value_enum)]
    pub transport_protocol: Option<TransportProtocol>,

    /// Filter by source port, insert the ports to include in the report
    /// Packets which source OR destination port is in the list are recorded
    #[clap(long, multiple_values = true, value_parser = clap::value_parser ! (u16).range(1..))]
    pub ports: Vec<u16>,

    /// Filter by source port, insert the ports to include in the report
    #[clap(long, multiple_values = true, value_parser = clap::value_parser ! (u16).range(1..))]
    pub source_ports: Vec<u16>,

    /// Filter by destination port, insert the ports to include in the report
    #[clap(long, multiple_values = true, value_parser = clap::value_parser ! (u16).range(1..))]
    pub destination_ports: Vec<u16>,
}

#[derive(clap::ValueEnum, Clone, Debug)]
pub enum TransportProtocol {
    TCP,
    UDP,
}