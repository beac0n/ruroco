use serde::{Deserialize, Serialize};
use std::net::IpAddr;

#[derive(Debug, Deserialize, Serialize)]
pub struct CommanderData {
    pub command_name: String,
    pub ip: IpAddr,
}

impl CommanderData {
    pub fn deserialize(data: &str) -> Result<CommanderData, String> {
        toml::from_str::<CommanderData>(data)
            .map_err(|e| format!("Could not create CommanderData from {data}: {e}"))
    }

    pub fn serialize(&self) -> Result<String, String> {
        toml::to_string(&self)
            .map_err(|e| format!("Could not serialize CommanderData {:?}: {e}", &self))
    }
}
