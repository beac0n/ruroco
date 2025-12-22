use crate::common::protocol::PLAINTEXT_SIZE;
use crate::common::{blake2b_u64, deserialize_ip, serialize_ip};
use serde::{Deserialize, Serialize};
use std::net::IpAddr;

#[derive(Debug, Deserialize, Serialize, PartialEq)]
pub(crate) struct ClientData {
    pub(crate) cmd_hash: u64,          // hashed command name
    pub(crate) counter: u128,          // request counter
    pub(crate) strict: bool,           // strict
    pub(crate) src_ip: Option<IpAddr>, // source ip address
    pub(crate) dst_ip: IpAddr,         // host/destination ip address
}

impl ClientData {
    pub(crate) fn create(
        command: &str,
        strict: bool,
        src_ip: Option<IpAddr>,
        dst_ip: IpAddr,
        counter: u128,
    ) -> anyhow::Result<ClientData> {
        Ok(ClientData {
            cmd_hash: blake2b_u64(command)?,
            counter,
            strict,
            src_ip,
            dst_ip,
        })
    }

    pub(crate) fn serialize(&self) -> anyhow::Result<[u8; PLAINTEXT_SIZE]> {
        let mut out = [0u8; PLAINTEXT_SIZE];

        out[0..8].copy_from_slice(&self.cmd_hash.to_be_bytes());
        out[8..24].copy_from_slice(&self.counter.to_be_bytes());
        out[24] = self.strict as u8;
        out[25..41].copy_from_slice(&self.src_ip.map(|i| serialize_ip(&i)).unwrap_or([0u8; 16]));
        out[41..].copy_from_slice(&serialize_ip(&self.dst_ip));

        Ok(out)
    }

    pub(crate) fn deserialize(data: [u8; PLAINTEXT_SIZE]) -> anyhow::Result<Self> {
        let mut command_hash_bytes = [0u8; 8];
        command_hash_bytes.copy_from_slice(&data[0..8]);

        let mut counter_bytes = [0u8; 16];
        counter_bytes.copy_from_slice(&data[8..24]);

        let mut source_ip_bytes = [0u8; 16];
        source_ip_bytes.copy_from_slice(&data[25..41]);

        let mut host_ip_bytes = [0u8; 16];
        host_ip_bytes.copy_from_slice(&data[41..]);

        Ok(Self {
            cmd_hash: u64::from_be_bytes(command_hash_bytes),
            counter: u128::from_be_bytes(counter_bytes),
            strict: data[24] != 0,
            src_ip: (source_ip_bytes != [0u8; 16]).then(|| deserialize_ip(source_ip_bytes)),
            dst_ip: deserialize_ip(host_ip_bytes),
        })
    }

    pub(crate) fn is_source_ip_invalid(&self, source_ip: IpAddr) -> bool {
        self.strict && self.src_ip.is_some_and(|ip_sent| ip_sent != source_ip)
    }
}

#[cfg(test)]
mod tests {
    use crate::common::blake2b_u64;
    use crate::common::protocol::client_data::ClientData;
    use crate::common::protocol::PLAINTEXT_SIZE;
    use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

    #[test]
    fn test_max_size() {
        let server_data = ClientData {
            cmd_hash: u64::MAX,
            counter: u128::MAX,
            strict: true,
            src_ip: Some(IpAddr::V6(Ipv6Addr::UNSPECIFIED)),
            dst_ip: IpAddr::V6(Ipv6Addr::UNSPECIFIED),
        }
        .serialize()
        .unwrap();

        assert_eq!(server_data.len(), PLAINTEXT_SIZE);
    }

    #[test]
    fn test_min_size() {
        let server_data = ClientData {
            cmd_hash: 0,
            counter: 0,
            strict: false,
            src_ip: Some(IpAddr::V4(Ipv4Addr::UNSPECIFIED)),
            dst_ip: IpAddr::V4(Ipv4Addr::UNSPECIFIED),
        }
        .serialize()
        .unwrap();

        assert_eq!(server_data.len(), PLAINTEXT_SIZE);
    }

    #[test]
    fn test_get_minified_server_data() {
        let server_data = ClientData::create(
            "some_kind_of_long_but_not_really_that_long_command",
            false,
            Some("192.168.178.123".parse().unwrap()),
            "192.168.178.124".parse().unwrap(),
            1725821510 * 1_000_000_000,
        )
        .unwrap()
        .serialize()
        .unwrap();
        assert_eq!(server_data.len(), PLAINTEXT_SIZE);
        dbg!(String::from_utf8_lossy(&server_data).to_string());

        assert_eq!(
            ClientData::deserialize(server_data).unwrap(),
            ClientData {
                cmd_hash: blake2b_u64("some_kind_of_long_but_not_really_that_long_command")
                    .unwrap(),
                counter: 1725821510 * 1_000_000_000,
                strict: false,
                src_ip: Some(IpAddr::from(Ipv4Addr::new(192, 168, 178, 123))),
                dst_ip: IpAddr::from(Ipv4Addr::new(192, 168, 178, 124)),
            }
        );
    }
}
