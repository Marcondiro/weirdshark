use std::error::Error;
use std::fmt;

#[derive(Debug)]
pub enum WeirdsharkError {
    GenericError,
    CapturerError(String),
    InterfaceNotSpecified,
    PacketIgnored,
    PacketIgnoredNonIp,
    IncompleteEthernetFrame,
    IncompleteIpPacket,
    IncompleteTcpSegment,
    UnsupportedTransportProtocol,
}

impl fmt::Display for WeirdsharkError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Weirdshark error: {:?}", &self)
    }
}

impl Error for WeirdsharkError {}
