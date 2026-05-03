use std::net::IpAddr;
#[cfg(feature = "with-server")]
use std::net::Ipv6Addr;

const IP_SIZE: usize = 16;

pub(crate) fn serialize_ip(ip: &IpAddr) -> [u8; IP_SIZE] {
    match ip {
        IpAddr::V4(v4) => v4.to_ipv6_mapped().octets(),
        IpAddr::V6(v6) => v6.octets(),
    }
}

#[cfg(feature = "with-server")]
pub(crate) fn deserialize_ip(data: [u8; IP_SIZE]) -> IpAddr {
    crate::common::normalize_ip(IpAddr::V6(Ipv6Addr::from(data)))
}
