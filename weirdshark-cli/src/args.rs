use clap::{Parser, Args, Subcommand, ValueEnum, ArgGroup};
use std::net::IpAddr;
use crate::tuple2;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
pub struct Cli {
    #[clap(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// List available interfaces
    Interfaces,

    /// Start a network capture
    Capture(CaptureParams),
}

#[derive(Args, Debug)]
#[clap(group(ArgGroup::new("interface-selector")
.required(true)
.args(& ["interface-name", "interface-index", "interface-desc"]),
))]
pub struct CaptureParams {
    /// Name of the network interface to capture from
    #[clap()]
    pub interface_name: Option<String>,

    ///Select the capturing network interface by its index
    #[clap(short = 'i', long)]
    pub interface_index: Option<u32>,

    ///Select the capturing network interface by its description
    #[clap(short = 'd', long)]
    pub interface_desc: Option<String>,

    /// Output reports path
    #[clap(short = 'o', long, default_value = ".")]
    pub path: String,

    /// Time interval in seconds after which a new report is generated
    /// (if not provided, only one report at the end is generated)
    #[clap(short, long, value_parser = clap::value_parser ! (u64).range(1..))]
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

    /// Filter by ip, insert IP range to include in the report
    /// Packets which source OR destination IP is in the range are recorded
    #[clap(long, multiple_values = true)]
    pub ip_range: Vec<tuple2::Tuple2<IpAddr>>,

    /// Filter by source ip, insert IP range to include in the report
    #[clap(long, multiple_values = true)]
    pub source_ip_range: Vec<tuple2::Tuple2<IpAddr>>,

    /// Filter by destination ip, insert IP range to include in the report
    #[clap(long, multiple_values = true)]
    pub destination_ip_range: Vec<tuple2::Tuple2<IpAddr>>,

    /// Filter by transport protocol
    #[clap(long, value_enum)]
    pub transport_protocol: Option<TransportProtocol>,

    /// Filter by source port, insert the ports to include in the report
    /// Packets which source OR destination port is in the list are recorded
    #[clap(long, multiple_values = true)]
    pub ports: Vec<u16>,

    /// Filter by source port, insert the ports to include in the report
    #[clap(long, multiple_values = true)]
    pub source_ports: Vec<u16>,

    /// Filter by destination port, insert the ports to include in the report
    #[clap(long, multiple_values = true)]
    pub destination_ports: Vec<u16>,

    /// Filter by port, insert port range to include in the report
    /// Packets which source OR destination port is in the range are recorded
    #[clap(long, multiple_values = true)]
    pub port_range: Vec<tuple2::Tuple2<u16>>,

    /// Filter by source port, insert port range to include in the report
    #[clap(long, multiple_values = true)]
    pub source_port_range: Vec<tuple2::Tuple2<u16>>,

    /// Filter by destination port, insert port range to include in the report
    #[clap(long, multiple_values = true)]
    pub destination_port_range: Vec<tuple2::Tuple2<u16>>,

}

#[derive(ValueEnum, Clone, Debug)]
pub enum TransportProtocol {
    TCP,
    UDP,
}