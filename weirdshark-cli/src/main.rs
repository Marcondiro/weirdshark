use std::io;
use clap::Parser;
use weirdshark;

mod args;

fn main() {
    let args = args::Args::parse();
    let path = args.path.clone() + if args.path.ends_with(".csv") { "" } else { ".csv" };

    // weirdshark::capture(args.interface, path).unwrap();

    let mut cfg = weirdshark::capturer::CaptureConfig::new(
        &args.interface, &args.path, args.time_interval,
    );
    cfg.set_interface_by_name(&args.interface);
    let capturer = weirdshark::capturer::CaptureController::new(cfg);
    capturer.start();

    let mut buffer = String::new();
    let mut stdin = io::stdin(); // We get `Stdin` here.
    stdin.read_line(&mut buffer).unwrap();

    capturer.stop();
}
