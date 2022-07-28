use clap::Parser;
use weirdshark;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Network interface to capture from
    #[clap(value_parser)]
    interface: String,

    /// Output file name
    #[clap(short = 'n', long, value_parser, default_value = "weirdshark_capture")]
    file_name: String,

    /// Time interval in seconds after which a new report is generated (0 to have only one report at the end)
    #[clap(short, long, value_parser, default_value_t = 0)]
    time_interval: usize,

    // TODO Choose filter details in weirdshark lib and implement this param accordingly
    /// Filter
    #[clap(short, long, value_parser, default_value = "")]
    filter: String,
}

fn main() {
    let args = Args::parse();
    weirdshark::test_save_csv().unwrap();
}