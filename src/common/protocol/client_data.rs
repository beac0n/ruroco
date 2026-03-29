use serde::{Deserialize, Serialize};
use std::net::IpAddr;

#[derive(Debug, Deserialize, Serialize, PartialEq)]
pub(crate) struct ClientData {
    pub(crate) cmd_hash: u64,
    pub(crate) counter: u128,
    pub(crate) strict: bool,
    pub(crate) src_ip: Option<IpAddr>,
    pub(crate) dst_ip: IpAddr,
}

#[cfg(test)]
mod tests {
    use crate::common::blake2b_u64;
    use crate::common::protocol::client_data::ClientData;
    use crate::common::protocol::PLAINTEXT_SIZE;
    use std::net::{IpAddr, Ipv4Addr};

    #[test]
    fn test_roundtrip() {
        let data = ClientData::create(
            "some_kind_of_long_but_not_really_that_long_command",
            false,
            Some("192.168.178.123".parse().unwrap()),
            "192.168.178.124".parse().unwrap(),
            1725821510 * 1_000_000_000,
        )
        .unwrap()
        .serialize()
        .unwrap();
        assert_eq!(data.len(), PLAINTEXT_SIZE);

        assert_eq!(
            ClientData::deserialize(data).unwrap(),
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
