use rmp_serde::{Deserializer, Serializer};
use serde::{Deserialize, Serialize};
use std::net::IpAddr;

#[derive(Debug, Deserialize, Serialize, PartialEq)]
// use one char for each field, to minify serialized size
pub struct ClientData {
    pub c: String,         // command name
    pub d: u128,           // deadline in ns
    pub s: u8,             // strict -> 0 == false, 1 == true
    pub i: Option<IpAddr>, // source ip address
    pub h: IpAddr,         // host ip address
}

impl ClientData {
    pub fn create(
        command: &str,
        deadline: u16,
        strict: bool,
        source_ip: Option<String>,
        destination_ip: String,
        now_ns: u128,
    ) -> ClientData {
        ClientData {
            c: command.to_string(),
            d: now_ns + (u128::from(deadline) * 1_000_000_000),
            s: if strict { 1 } else { 0 },
            i: source_ip.and_then(|d| d.parse().ok()),
            h: destination_ip.parse().unwrap(),
        }
    }

    pub fn serialize(&self) -> Result<Vec<u8>, String> {
        let mut buf = Vec::new();
        <Self as Serialize>::serialize(self, &mut Serializer::new(&mut buf))
            .map_err(|e| format!("MsgPack serialize error: {e}"))?;
        Ok(buf)
    }

    pub fn deserialize(data: &[u8]) -> Result<Self, String> {
        let mut de = Deserializer::new(data);
        <Self as Deserialize>::deserialize(&mut de)
            .map_err(|e| format!("MsgPack deserialize error: {e}"))
    }

    pub fn is_strict(&self) -> bool {
        self.s == 1
    }

    pub fn source_ip(&self) -> Option<IpAddr> {
        self.i
    }

    pub fn validate_source_ip(&self, source_ip: IpAddr) -> bool {
        self.is_strict() && self.source_ip().is_some_and(|ip_sent| ip_sent != source_ip)
    }

    pub fn destination_ip(&self) -> IpAddr {
        self.h
    }

    pub fn deadline(&self) -> u128 {
        self.d
    }
}

#[cfg(test)]
mod tests {
    use crate::common::client_data::ClientData;

    #[test]
    fn test_get_minified_server_data() {
        let server_data = ClientData::create(
            "some_kind_of_long_but_not_really_that_long_command",
            5,
            false,
            Some("192.168.178.123".to_string()),
            "192.168.178.124".to_string(),
            1725821510 * 1_000_000_000,
        )
        .serialize()
        .unwrap();
        assert_eq!(server_data.len(), 96);
        dbg!(String::from_utf8_lossy(&server_data).to_string());
        assert_eq!(
            ClientData::deserialize(&server_data).unwrap(),
            ClientData {
                c: "some_kind_of_long_but_not_really_that_long_command".to_string(),
                d: 1725821515000000000,
                s: 0,
                i: Some("192.168.178.123".parse().unwrap()),
                h: "192.168.178.124".parse().unwrap()
            }
        );
    }
}
