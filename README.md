# weirdshark

`weirdshark` is a cross-platform library written in Rust capable of **intercepting** incoming and outgoing **traffic** through the network interfaces.

The library allows to collect
- IP address
- port
- layer 4 protocol (TCP/UDP)

of observed traffic and will generate a textual **report in csv format**.

The report lists for each of the network address/port pairs that have been observed and protocol, the cumulated number of bytes transmitted, the timestamp of the first and last occurrence of information exchange.

The most importat parameters defineable by the user are
- network adapter to be inspected
- output file to be generated
- time interval after which a new report is generated
- filters to apply to captured data.

## Example

Basic example that captures traffic on the default interface for 10 seconds and at the end writes the report.
```
use weirdshark;

fn main() {
    let capturer_builder = weirdshark::capturer::CapturerBuilder::default();
    let capturer = capturer_builder.build().unwrap();
    capturer.start().unwrap();
    std::thread::sleep(std::time::Duration::from_secs(10));
    capturer.stop().unwrap();
}
```

A more complete example is the weirdshark-cli program itself

## weirdshard-cli

Weirdshark-cli, as the name suggests, is a command line interface program that exploits most of the capabilities of weirdshark exposed before.

To have usage details run `weirdshark-cli -h`
