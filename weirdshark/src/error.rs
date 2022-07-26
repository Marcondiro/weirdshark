use std::error::Error;
use std::fmt;

/// All Weirdshark custom errors.
#[derive(Debug, PartialEq, Eq)]
pub enum WeirdsharkError {
    GenericError,
    CapturerError(String),
    InterfaceNotSpecified,
    PacketIgnored,
    PacketIgnoredNonIp,
    IncompleteEthernetFrame,
    IncompleteIpPacket,
    IncompleteTcpSegment,
    IncompleteUdpSegment,
    UnsupportedTransportProtocol,
}

impl fmt::Display for WeirdsharkError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Weirdshark error: {:?}", &self)
    }
}

impl Error for WeirdsharkError {}
