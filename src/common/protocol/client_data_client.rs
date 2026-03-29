use crate::common::protocol::client_data::ClientData;
use crate::common::protocol::PLAINTEXT_SIZE;
use crate::common::{blake2b_u64, serialize_ip};
use std::net::IpAddr;

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
}

#[cfg(test)]
mod tests {
    use crate::common::protocol::client_data::ClientData;
    use crate::common::protocol::PLAINTEXT_SIZE;
    use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

    #[test]
    fn test_max_size() {
        let data = ClientData {
            cmd_hash: u64::MAX,
            counter: u128::MAX,
            strict: true,
            src_ip: Some(IpAddr::V6(Ipv6Addr::UNSPECIFIED)),
            dst_ip: IpAddr::V6(Ipv6Addr::UNSPECIFIED),
        }
        .serialize()
        .unwrap();

        assert_eq!(data.len(), PLAINTEXT_SIZE);
    }

    #[test]
    fn test_min_size() {
        let data = ClientData {
            cmd_hash: 0,
            counter: 0,
            strict: false,
            src_ip: Some(IpAddr::V4(Ipv4Addr::UNSPECIFIED)),
            dst_ip: IpAddr::V4(Ipv4Addr::UNSPECIFIED),
        }
        .serialize()
        .unwrap();

        assert_eq!(data.len(), PLAINTEXT_SIZE);
    }
}
