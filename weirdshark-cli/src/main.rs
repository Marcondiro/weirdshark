use std::io;
use std::net::IpAddr;
use clap::Parser;
use weirdshark;
use weirdshark::TransportProtocols;
use crate::args::{CaptureParams, TransportProtocol};

mod args;

fn main() {
    let args = args::Cli::parse();

    match args.command {
        args::Command::Interfaces => list_interfaces(),
        args::Command::Capture(params) => capture(params),
    }
}

fn list_interfaces() {
    let interfaces = weirdshark::get_interfaces();
    println!("Available interfaces: ");
    for i in interfaces {
        if cfg!(windows) {
            println!("{}:", i.description);
        }
        println!("{}", i);
    }
    return;
}

fn capture(args: CaptureParams) {
    let mut capturer_cfg = weirdshark::CapturerBuilder::new()
        .report_path(args.path.as_ref());

    if let Some(i_name) = &args.interface_name {
        capturer_cfg = capturer_cfg.interface_by_name(i_name);
    } else if let Some(interface_desc) = &args.interface_desc {
        capturer_cfg = capturer_cfg.interface_by_description(interface_desc);
    } else if let Some(interface_index) = args.interface_index {
        capturer_cfg = capturer_cfg.interface_by_index(interface_index);
    } else {
        unreachable!();
    }

    if let Some(time_interval) = args.time_interval {
        capturer_cfg = capturer_cfg.report_interval(Some(std::time::Duration::from_secs(time_interval)));
    }

    if !args.ips.is_empty() {
        capturer_cfg = capturer_cfg.add_undirected_filter_ip(weirdshark::Filter::from_vec(args.ips));
    }

    if !args.source_ips.is_empty() {
        let filter = weirdshark::Filter::from_vec(args.source_ips);
        capturer_cfg = capturer_cfg.add_directed_filter_ip(weirdshark::DirectedFilter::only_source(filter));
    }

    if !args.destination_ips.is_empty() {
        let filter = weirdshark::Filter::from_vec(args.destination_ips);
        capturer_cfg = capturer_cfg.add_directed_filter_ip(weirdshark::DirectedFilter::only_destination(filter));
    }

    if !args.ip_range.is_empty() {
        let vec: Vec<weirdshark::Filter<IpAddr>> = args.ip_range.into_iter()
            .map(|tuple| { weirdshark::Filter::from_range(tuple._0, tuple._1) })
            .collect();
        for filter in vec {
            capturer_cfg = capturer_cfg.add_undirected_filter_ip(filter);
        }
    }

    if !args.source_ip_range.is_empty() {
        let vec: Vec<weirdshark::Filter<IpAddr>> = args.source_ip_range.into_iter()
            .map(|tuple| { weirdshark::Filter::from_range(tuple._0, tuple._1) })
            .collect();
        for filter in vec {
            capturer_cfg = capturer_cfg.add_directed_filter_ip(weirdshark::DirectedFilter::only_source(filter));
        }
    }

    if !args.destination_ip_range.is_empty() {
        let vec: Vec<weirdshark::Filter<IpAddr>> = args.destination_ip_range.into_iter()
            .map(|tuple| { weirdshark::Filter::from_range(tuple._0, tuple._1) })
            .collect();
        for filter in vec {
            capturer_cfg = capturer_cfg.add_directed_filter_ip(weirdshark::DirectedFilter::only_destination(filter));
        }
    }

    if !args.ports.is_empty() {
        capturer_cfg = capturer_cfg.add_undirected_filter_port(weirdshark::Filter::from_vec(args.ports));
    }

    if !args.source_ports.is_empty() {
        let filter = weirdshark::Filter::from_vec(args.source_ports);
        capturer_cfg = capturer_cfg.add_directed_filter_port(weirdshark::DirectedFilter::only_source(filter));
    }

    if !args.destination_ports.is_empty() {
        let filter = weirdshark::Filter::from_vec(args.destination_ports);
        capturer_cfg = capturer_cfg.add_directed_filter_port(weirdshark::DirectedFilter::only_destination(filter));
    }

    if !args.port_range.is_empty() {
        let vec: Vec<weirdshark::Filter<u16>> = args.port_range.into_iter()
            .map(|tuple| { weirdshark::Filter::from_range(tuple._0, tuple._1) })
            .collect();
        for filter in vec {
            capturer_cfg = capturer_cfg.add_undirected_filter_port(filter);
        }
    }

    if !args.source_port_range.is_empty() {
        let vec: Vec<weirdshark::Filter<u16>> = args.source_port_range.into_iter()
            .map(|tuple| { weirdshark::Filter::from_range(tuple._0, tuple._1) })
            .collect();
        for filter in vec {
            capturer_cfg = capturer_cfg.add_directed_filter_port(weirdshark::DirectedFilter::only_source(filter));
        }
    }

    if !args.destination_port_range.is_empty() {
        let vec: Vec<weirdshark::Filter<u16>> = args.destination_port_range.into_iter()
            .map(|tuple| { weirdshark::Filter::from_range(tuple._0, tuple._1) })
            .collect();
        for filter in vec {
            capturer_cfg = capturer_cfg.add_directed_filter_port(weirdshark::DirectedFilter::only_destination(filter));
        }
    }

    let protocol_filter: Option<TransportProtocols> = if let Some(TransportProtocol::TCP) = args.transport_protocol {
        Some(TransportProtocols::TCP)
    } else if let Some(TransportProtocol::UDP) = args.transport_protocol {
        Some(TransportProtocols::UDP)
    } else {
        None
    };

    capturer_cfg = capturer_cfg.add_transport_protocol_filter(protocol_filter);

    let capturer = match capturer_cfg.build() {
        Ok(cap) => cap,
        Err(err) => {
            eprintln!("Cannot start capture for: {:?}", err);
            return;
        }
    };

    capturer.start().unwrap();
    println!("Capture started");
    print_capture_help();

    loop {
        let mut buffer = String::new();
        let stdin = io::stdin();
        stdin.read_line(&mut buffer).unwrap();

        match buffer.to_lowercase().trim_end() {
            "start" => {
                capturer.start().unwrap();
                println!("Capture started");
            }
            "pause" => {
                capturer.pause().unwrap();
                println!("Capture paused");
            }
            "stop" => break,
            "help" => print_capture_help(),
            _ => println!("Unknown command. Type `help` for a list of valid commands."),
        }
    }

    println!("Capture stopped");
    capturer.stop().unwrap();
}

fn print_capture_help() {
    let msg =
        "Valid commands:\n\
        \tstart\tResume the capture after a pause\n\
        \tpause\tPause the capture\n\
        \tstop\tTerminate the capture, save the report and then terminate the program\n\
        \thelp\tPrint this help message\n";
    println!("{}", msg);
}
