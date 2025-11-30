use crate::common::serialization_util::{deserialize_ip, serialize_ip};
use std::net::IpAddr;

pub const CMDR_DATA_SIZE: usize = 24;

pub struct CommanderData {
    pub cmd_hash: u64,
    pub ip: IpAddr,
}

impl CommanderData {
    pub fn serialize(&self) -> [u8; CMDR_DATA_SIZE] {
        let mut data = [0u8; CMDR_DATA_SIZE];
        data[..8].copy_from_slice(&self.cmd_hash.to_be_bytes());
        data[8..].copy_from_slice(&serialize_ip(&self.ip));
        data
    }

    pub fn deserialize(data: [u8; CMDR_DATA_SIZE]) -> Self {
        let mut cmd_hash_bytes = [0u8; 8];
        cmd_hash_bytes.copy_from_slice(&data[0..8]);
        let mut ip_bytes = [0u8; 16];
        ip_bytes.copy_from_slice(&data[8..]);

        Self {
            cmd_hash: u64::from_be_bytes(cmd_hash_bytes),
            ip: deserialize_ip(ip_bytes),
        }
    }
}
