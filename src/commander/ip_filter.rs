//! Decides whether a client-supplied IP may be exposed to a command as `$RUROCO_IP`. The IP
//! reaches the executed shell command, so only globally-routable unicast peers are allowed:
//! reject loopback, private, and other non-routable ranges a client must not be able to whitelist.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

pub(crate) fn is_routable(ip: IpAddr) -> bool {
    let reject = ip.is_unspecified()
        || ip.is_loopback()
        || ip.is_multicast()
        || match ip {
            IpAddr::V4(v4) => is_ipv4_rejected(v4),
            IpAddr::V6(v6) => is_ipv6_rejected(v6),
        };

    !reject
}

fn is_ipv6_rejected(v6: Ipv6Addr) -> bool {
    v6.is_unique_local() || v6.is_unicast_link_local() || is_ipv6_documentation(v6)
}

/// IPv6 documentation range, RFC 3849 (`2001:db8::/32`). Hand-rolled because
/// `Ipv6Addr::is_documentation` is still behind the unstable `ip` feature (rust-lang/rust #27709);
/// switch to it once stabilized.
fn is_ipv6_documentation(v6: Ipv6Addr) -> bool {
    let segments = v6.segments();
    segments[0] == 0x2001 && segments[1] == 0x0db8
}

fn is_ipv4_rejected(v4: Ipv4Addr) -> bool {
    v4.is_broadcast()
        || v4.is_private()
        || v4.is_link_local()
        || v4.is_documentation()
        || is_ipv4_this_network(v4)
        || is_ipv4_shared_cgnat(v4)
        || is_ipv4_ietf_protocol_assignment(v4)
        || is_ipv4_benchmarking(v4)
        || is_ipv4_reserved(v4)
}

/// RFC 791 "this network" (`0.0.0.0/8`); `0.0.0.0` itself is already caught by `is_unspecified`
/// in `is_routable`.
fn is_ipv4_this_network(v4: Ipv4Addr) -> bool {
    v4.octets()[0] == 0
}

/// Carrier-grade NAT shared address space, RFC 6598 (`100.64.0.0/10`). Hand-rolled because
/// `Ipv4Addr::is_shared` is still behind the unstable `ip` feature (rust-lang/rust #27709); switch
/// to it once stabilized.
fn is_ipv4_shared_cgnat(v4: Ipv4Addr) -> bool {
    let octets = v4.octets();
    octets[0] == 100 && (64..=127).contains(&octets[1])
}

/// IETF protocol assignments, RFC 6890 (`192.0.0.0/24`).
fn is_ipv4_ietf_protocol_assignment(v4: Ipv4Addr) -> bool {
    let octets = v4.octets();
    octets[0] == 192 && octets[1] == 0 && octets[2] == 0
}

/// Benchmarking, RFC 2544 (`198.18.0.0/15`). Hand-rolled because `Ipv4Addr::is_benchmarking` is
/// still behind the unstable `ip` feature (rust-lang/rust #27709); switch to it once stabilized.
fn is_ipv4_benchmarking(v4: Ipv4Addr) -> bool {
    let octets = v4.octets();
    octets[0] == 198 && (octets[1] == 18 || octets[1] == 19)
}

/// Reserved for future use, RFC 1112 (`240.0.0.0/4`); the all-ones broadcast address is already
/// caught by `is_broadcast`. Hand-rolled because `Ipv4Addr::is_reserved` is still behind the
/// unstable `ip` feature (rust-lang/rust #27709); switch to it once stabilized.
fn is_ipv4_reserved(v4: Ipv4Addr) -> bool {
    v4.octets()[0] >= 240
}

#[cfg(test)]
mod tests {
    use super::is_routable;
    use std::net::IpAddr;

    #[test]
    fn test_is_routable_rejects_non_routable() {
        // Every category the guard rejects: unspecified, loopback, multicast, and the v4/v6
        // private/link-local/ULA/broadcast/documentation/CGNAT/benchmarking/reserved ranges.
        let rejected = [
            "0.0.0.0",         // unspecified v4
            "::",              // unspecified v6
            "127.0.0.1",       // loopback v4
            "::1",             // loopback v6
            "10.0.0.1",        // private v4 (10.0.0.0/8)
            "172.16.0.1",      // private v4 (172.16.0.0/12)
            "192.168.1.1",     // private v4 (192.168.0.0/16)
            "169.254.1.1",     // link-local v4
            "fe80::1",         // link-local v6
            "fc00::1",         // unique local v6 (fc00::/8)
            "fd00::1",         // unique local v6 (fd00::/8)
            "224.0.0.1",       // multicast v4
            "ff02::1",         // multicast v6
            "192.0.2.1",       // documentation v4 (TEST-NET-1)
            "198.51.100.1",    // documentation v4 (TEST-NET-2)
            "203.0.113.1",     // documentation v4 (TEST-NET-3)
            "255.255.255.255", // broadcast v4
            "0.1.2.3",         // "this network" v4 (0.0.0.0/8)
            "100.64.0.1",      // CGNAT shared space v4 (100.64.0.0/10)
            "100.127.255.254", // CGNAT shared space v4, top of range
            "192.0.0.1",       // IETF protocol assignment v4 (192.0.0.0/24)
            "198.18.0.1",      // benchmarking v4 (198.18.0.0/15)
            "198.19.255.254",  // benchmarking v4, top of range
            "240.0.0.1",       // reserved v4 (240.0.0.0/4)
            "2001:db8::1",     // documentation v6 (2001:db8::/32)
        ];

        for ip in rejected {
            let addr: IpAddr = ip.parse().unwrap();
            assert!(!is_routable(addr), "expected {ip} to be rejected");
        }
    }

    #[test]
    fn test_is_routable_accepts_public_unicast() {
        let allowed = [
            "1.2.3.4",              // public unicast v4
            "8.8.8.8",              // public unicast v4
            "2606:4700:4700::1111", // public unicast v6
            "100.63.255.255",       // just below the CGNAT range (100.64.0.0/10)
            "100.128.0.1",          // just above the CGNAT range
            "198.17.255.255",       // just below the benchmarking range (198.18.0.0/15)
            "198.20.0.1",           // just above the benchmarking range
        ];

        for ip in allowed {
            let addr: IpAddr = ip.parse().unwrap();
            assert!(is_routable(addr), "expected {ip} to be accepted");
        }
    }
}
