use serde::de::Error;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Debug, Deserialize, Serialize)]
pub struct CommanderData {
    pub command_name: String,
    pub ip: String,
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
// use one char for each field, to minify serialized size
pub struct ServerData {
    pub c: String, // command name
    #[serde(serialize_with = "serialize", deserialize_with = "deserialize")]
    pub d: u128, // deadline in ns
    pub s: u8,     // strict - 0 == false, 1 == true
    pub i: Option<String>, // ip address
}

impl ServerData {
    pub fn is_strict(&self) -> bool {
        self.s == 1
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
    s.parse::<u128>().map_err(D::Error::custom)
}
