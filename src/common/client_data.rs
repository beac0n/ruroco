use serde::de::Error;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Debug, Deserialize, Serialize, PartialEq)]
// use one char for each field, to minify serialized size
pub struct ClientData {
    pub c: String, // command name
    #[serde(serialize_with = "serialize", deserialize_with = "deserialize")]
    pub d: u128, // deadline in ns
    pub s: u8,     // strict -> 0 == false, 1 == true
    pub i: Option<String>, // source ip address
    pub h: String, // host ip address
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
            i: source_ip,
            h: destination_ip,
        }
    }

    pub fn deserialize(data: &[u8]) -> Result<ClientData, String> {
        let data_str = String::from_utf8_lossy(data).to_string();
        toml::from_str::<ClientData>(&data_str)
            .map_err(|e| format!("Could not deserialize ServerData {data_str}: {e}"))
    }

    pub fn serialize(&self) -> Result<Vec<u8>, String> {
        toml::to_string(&self)
            .map(|s| s.trim().replace(" = ", "=").as_bytes().to_vec())
            .map_err(|e| format!("Could not serialize data for server {:?}: {e}", &self))
    }

    pub fn is_strict(&self) -> bool {
        self.s == 1
    }

    pub fn source_ip(&self) -> Option<String> {
        self.i.clone()
    }

    pub fn validate_source_ip(&self, source_ip: &str) -> bool {
        self.is_strict() && self.source_ip().is_some_and(|ip_sent| ip_sent != source_ip)
    }

    pub fn destination_ip(&self) -> String {
        self.h.clone()
    }

    pub fn deadline(&self) -> u128 {
        self.d
    }
}

// Custom serialize function for timestamp
fn serialize<S>(timestamp: &u128, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    // Serialize the timestamp as a string
    serializer.serialize_str(&timestamp.to_string())
}

// Custom deserialize function for timestamp
fn deserialize<'de, D>(deserializer: D) -> Result<u128, D::Error>
where
    D: Deserializer<'de>,
{
    // Deserialize the timestamp from a string
    let s = String::deserialize(deserializer)?;
    s.parse::<u128>().map_err(Error::custom)
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
        let server_data_str = String::from_utf8_lossy(&server_data).to_string();

        assert_eq!(server_data_str, "c=\"some_kind_of_long_but_not_really_that_long_command\"\nd=\"1725821515000000000\"\ns=0\ni=\"192.168.178.123\"\nh=\"192.168.178.124\"");
        assert_eq!(
            ClientData::deserialize(&server_data).unwrap(),
            ClientData {
                c: "some_kind_of_long_but_not_really_that_long_command".to_string(),
                d: 1725821515000000000,
                s: 0,
                i: Some("192.168.178.123".to_string()),
                h: "192.168.178.124".to_string()
            }
        );
    }
}
