use std::net::IpAddr;

pub enum IpFilter {
    Single(IpAddr),
    Range(IpAddr, IpAddr),
    List(Vec<IpAddr>),
}

impl IpFilter {
    pub fn filter(&self, x: &IpAddr) -> bool {
        use IpFilter::{Single, Range, List};
        match self {
            Single(addr) => x == addr,
            Range(start_addr, end_addr) => start_addr <= x && x <= end_addr,
            List(list) => list.iter().any(|ip| { ip == x })
        }
    }
}

pub enum DirectionFilter {
    Source,
    Destination,
    Both,
}