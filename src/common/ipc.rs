//! The server <-> commander IPC contract: where the Unix socket lives and what flows over it.
//!
//! Shared because the server *produces* `CommanderData` (and connects to the socket) while the
//! commander *consumes* it (and binds the socket). Kept free of crypto/network code so the
//! commander can link it without OpenSSL.

use crate::common::protocol::serialization::{deserialize_ip, serialize_ip};
use crate::common::resolve_path;
use std::net::IpAddr;
use std::path::{Path, PathBuf};

pub(crate) const CMDR_DATA_SIZE: usize = 24;

/// The 24-byte message the server sends the commander over the Unix socket:
/// `cmd_hash` (`u64`, bytes 0:8) followed by the client IP (16 bytes, 8:24).
pub(crate) struct CommanderData {
    pub(crate) cmd_hash: u64,
    pub(crate) ip: IpAddr,
}

impl From<CommanderData> for [u8; CMDR_DATA_SIZE] {
    fn from(value: CommanderData) -> Self {
        let mut data = [0u8; CMDR_DATA_SIZE];
        data[..8].copy_from_slice(&value.cmd_hash.to_be_bytes());
        data[8..].copy_from_slice(&serialize_ip(&value.ip));
        data
    }
}

impl From<[u8; CMDR_DATA_SIZE]> for CommanderData {
    fn from(data: [u8; CMDR_DATA_SIZE]) -> Self {
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

pub fn get_commander_unix_socket_path(config_dir: &Path) -> PathBuf {
    resolve_path(config_dir).join("ruroco.socket")
}

#[cfg(test)]
mod tests {
    use crate::common::ipc::get_commander_unix_socket_path;
    use std::path::PathBuf;

    #[test]
    fn test_get_socket_path() {
        assert_eq!(
            get_commander_unix_socket_path(&PathBuf::from("/foo/bar/baz")),
            PathBuf::from("/foo/bar/baz/ruroco.socket")
        );
    }
}
