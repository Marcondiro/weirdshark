use weirdshark;
use clap::Parser;

mod args;

fn main() {
    let args = args::Args::parse();
    let path = args.path.clone() + if args.path.ends_with(".csv") { "" } else { ".csv" };

    weirdshark::capture(args.interface, path).unwrap();
}
