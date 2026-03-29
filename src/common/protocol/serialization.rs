use std::net::IpAddr;

const IP_SIZE: usize = 16;

pub(crate) fn serialize_ip(ip: &IpAddr) -> [u8; IP_SIZE] {
    match ip {
        IpAddr::V4(v4) => v4.to_ipv6_mapped().octets(),
        IpAddr::V6(v6) => v6.octets(),
    }
}
