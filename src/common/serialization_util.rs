use std::net::{IpAddr, Ipv6Addr};

pub fn serialize_ip(ip: &IpAddr) -> [u8; 16] {
    match ip {
        IpAddr::V4(v4) => v4.to_ipv6_mapped().octets(),
        IpAddr::V6(v6) => v6.octets(),
    }
}

pub fn deserialize_ip(data: [u8; 16]) -> IpAddr {
    let v6 = Ipv6Addr::from(data);
    if let Some(v4) = v6.to_ipv4_mapped() {
        IpAddr::V4(v4)
    } else {
        IpAddr::V6(v6)
    }
}
