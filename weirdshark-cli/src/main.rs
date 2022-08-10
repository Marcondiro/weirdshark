use std::io;
use clap::{Parser};
use weirdshark;
use crate::args::CaptureParams;

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
    let mut capturer_cfg = weirdshark::capturer::CapturerBuilder::new();

    if let Some(i_name) = &args.interface_name {
        capturer_cfg = capturer_cfg.interface_by_name(i_name);
    } else if let Some(interface_desc) = &args.interface_desc {
        capturer_cfg = capturer_cfg.interface_by_description(interface_desc);
    } else if let Some(interface_index) = args.interface_index {
        capturer_cfg = capturer_cfg.interface_by_index(interface_index);
    }

    if let Some(time_interval) = args.time_interval {
        capturer_cfg = capturer_cfg.report_interval(Some(std::time::Duration::from_secs(time_interval)));
    }

    capturer_cfg = capturer_cfg.report_path(args.path.as_ref());

    let capturer = match capturer_cfg.build() {
        Ok(cap) => cap,
        Err(err) => {
            eprintln!("Cannot start capture for: {:?}", err);
            return;
        }
    };

    capturer.start();
    println!("Capture started"); //TODO print instructions

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
