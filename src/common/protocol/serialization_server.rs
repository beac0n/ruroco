use crate::common::normalize_ip;
use std::net::{IpAddr, Ipv6Addr};

const IP_SIZE: usize = 16;

pub(crate) fn deserialize_ip(data: [u8; IP_SIZE]) -> IpAddr {
    normalize_ip(IpAddr::V6(Ipv6Addr::from(data)))
}
