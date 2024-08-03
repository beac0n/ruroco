use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct CommanderData {
    pub command_name: String,
    pub ip: String,
}
