use std::io;
use std::path::PathBuf;
use clap::{Parser};
use weirdshark;

mod args;

fn main() {
    let args = args::Args::parse();

    if args.list_interfaces {
        return list_interfaces();
    }
    let mut capturer_cfg = weirdshark::capturer::CaptureConfig::new();

    if let Some(ref i_name) = args.interface_name {
        //let i_name = args.interface_name.unwrap();
        //interface = weirdshark::get_interface_by_name(i_name);
        capturer_cfg = capturer_cfg.interface_by_name(&i_name);
    }

    if let Some(ref interface_desc) = args.interface_desc {
        //let i_name = args.interface_name.unwrap();
        //interface = weirdshark::get_interface_by_name(i_name);
        capturer_cfg = capturer_cfg.interface_by_description(&interface_desc);
    }

    if let Some(interface_index) = args.interface_index {
        capturer_cfg = capturer_cfg.interface_by_index(interface_index);
    }

    if let Some(time_interval) = args.time_interval {
        capturer_cfg = capturer_cfg.report_interval(Some(std::time::Duration::from_secs(time_interval)));
    }

    if args.interface_name.is_none() && args.interface_index.is_none() && args.interface_desc.is_none() {
        eprintln!("To start a capture you need to provide a network interface");
        println!("To see a list of the available network intefaces run weirdshark-cli -l");
        println!("For any other information run weirdshark-cli -h");
        return;
    }

    capturer_cfg = capturer_cfg.report_path(PathBuf::from(args.path));

    let capturer = match capturer_cfg.build() {
        Ok(cap) => cap,
        Err(err) => {
            eprintln!("Cannot start capture for: {:?}", err);
            return;
        }
    };

    capturer.start();
    println!("Capture started");

    loop {
        let mut buffer = String::new();
        let stdin = io::stdin();
        stdin.read_line(&mut buffer).unwrap();

        match buffer.to_lowercase().trim_end() {
            "start" => {
                capturer.start();
                println!("Capture started");
            }
            "pause" => {
                capturer.pause();
                println!("Capture paused");
            }
            "stop" => break,
            _ => println!("Unknown command."), //TODO add a help command?
        }
    }

    println!("Capture stopped");
    capturer.stop();
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
