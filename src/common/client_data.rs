use crate::common::blake2b_u64;
use serde::{Deserialize, Serialize};
use std::net::{IpAddr, Ipv6Addr};

#[derive(Debug, Deserialize, Serialize, PartialEq)]
pub struct ClientData {
    pub command_hash: u64,         // hashed command name
    pub deadline: u128,            // deadline in ns
    pub strict: bool,              // strict
    pub source_ip: Option<IpAddr>, // source ip address
    pub destination_ip: IpAddr,    // host/destination ip address
}

impl ClientData {
    pub fn create(
        command: &str,
        deadline: u16,
        strict: bool,
        source_ip: Option<IpAddr>,
        destination_ip: IpAddr,
        now_ns: u128,
    ) -> Result<ClientData, String> {
        Ok(ClientData {
            command_hash: blake2b_u64(command)?,
            deadline: now_ns + (u128::from(deadline) * 1_000_000_000),
            strict,
            source_ip,
            destination_ip,
        })
    }

    pub fn serialize(&self) -> Result<[u8; 57], String> {
        let mut out = [0u8; 57];

        let source_ip_bytes: [u8; 16] = match self.source_ip {
            Some(IpAddr::V4(v4)) => v4.to_ipv6_mapped().octets(),
            Some(IpAddr::V6(v6)) => v6.octets(),
            None => [0u8; 16],
        };

        let destination_ip_bytes: [u8; 16] = match self.destination_ip {
            IpAddr::V4(v4) => v4.to_ipv6_mapped().octets(),
            IpAddr::V6(v6) => v6.octets(),
        };

        out[0..8].copy_from_slice(&self.command_hash.to_be_bytes());
        out[8..24].copy_from_slice(&self.deadline.to_be_bytes());
        out[24] = self.strict as u8;
        out[25..41].copy_from_slice(&source_ip_bytes);
        out[41..57].copy_from_slice(&destination_ip_bytes);

        Ok(out)
    }

    pub fn deserialize(data: &[u8]) -> Result<Self, String> {
        let mut command_hash_bytes = [0u8; 8];
        let mut deadline_bytes = [0u8; 16];
        let mut source_ip_bytes = [0u8; 16];
        let mut host_ip_bytes = [0u8; 16];

        command_hash_bytes.copy_from_slice(&data[0..8]);
        deadline_bytes.copy_from_slice(&data[8..24]);
        let strict = data[24] != 0;
        source_ip_bytes.copy_from_slice(&data[25..41]);
        host_ip_bytes.copy_from_slice(&data[41..57]);

        let source_ip = if source_ip_bytes == [0u8; 16] {
            None
        } else if let Some(v4) = Ipv6Addr::from(source_ip_bytes).to_ipv4_mapped() {
            Some(IpAddr::V4(v4))
        } else {
            Some(IpAddr::V6(Ipv6Addr::from(source_ip_bytes)))
        };

        let v6 = Ipv6Addr::from(host_ip_bytes);
        let host_ip = if let Some(v4) = v6.to_ipv4_mapped() {
            IpAddr::V4(v4)
        } else {
            IpAddr::V6(v6)
        };

        Ok(Self {
            command_hash: u64::from_be_bytes(command_hash_bytes),
            deadline: u128::from_be_bytes(deadline_bytes),
            strict,
            source_ip,
            destination_ip: host_ip,
        })
    }

    pub fn validate_source_ip(&self, source_ip: IpAddr) -> bool {
        self.strict && self.source_ip.is_some_and(|ip_sent| ip_sent != source_ip)
    }
}

#[cfg(test)]
mod tests {
    use crate::common::blake2b_u64;
    use crate::common::client_data::ClientData;
    use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

    #[test]
    fn test_max_size() {
        let server_data = ClientData {
            command_hash: u64::MAX,
            deadline: u128::MAX,
            strict: true,
            source_ip: Some(IpAddr::V6(Ipv6Addr::UNSPECIFIED)),
            destination_ip: IpAddr::V6(Ipv6Addr::UNSPECIFIED),
        }
        .serialize()
        .unwrap();

        assert_eq!(server_data.len(), 57);
    }

    #[test]
    fn test_min_size() {
        let server_data = ClientData {
            command_hash: 0,
            deadline: 0,
            strict: false,
            source_ip: Some(IpAddr::V4(Ipv4Addr::UNSPECIFIED)),
            destination_ip: IpAddr::V4(Ipv4Addr::UNSPECIFIED),
        }
        .serialize()
        .unwrap();

        assert_eq!(server_data.len(), 57);
    }

    #[test]
    fn test_get_minified_server_data() {
        let server_data = ClientData::create(
            "some_kind_of_long_but_not_really_that_long_command",
            5,
            false,
            Some("192.168.178.123".parse().unwrap()),
            "192.168.178.124".parse().unwrap(),
            1725821510 * 1_000_000_000,
        )
        .unwrap()
        .serialize()
        .unwrap();
        assert_eq!(server_data.len(), 57);
        dbg!(String::from_utf8_lossy(&server_data).to_string());

        assert_eq!(
            ClientData::deserialize(&server_data).unwrap(),
            ClientData {
                command_hash: blake2b_u64("some_kind_of_long_but_not_really_that_long_command")
                    .unwrap(),
                deadline: 1725821515000000000u128,
                strict: false,
                source_ip: Some(IpAddr::from(Ipv4Addr::new(192, 168, 178, 123))),
                destination_ip: IpAddr::from(Ipv4Addr::new(192, 168, 178, 124)),
            }
        );
    }
}
