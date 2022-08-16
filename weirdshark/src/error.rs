#[derive(Debug)]
pub enum WeirdsharkError {
    GenericError,
    PacketIgnored,
    PacketIgnoredNonIp,
    IncompleteEthernetFrame,
    IncompleteIpPacket,
    IncompleteTcpSegment,
    UnsupportedTransportProtocol,
    WriteError(String),
    SerializeError(String),
    IoError(String),
}