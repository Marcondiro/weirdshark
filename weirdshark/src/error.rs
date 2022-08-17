#[derive(Debug)]
pub enum WeirdsharkError {
    GenericError,
    InterfaceNotSpecified,
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