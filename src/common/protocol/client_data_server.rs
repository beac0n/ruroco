use crate::common::protocol::client_data::ClientData;
use crate::common::protocol::PLAINTEXT_SIZE;
use crate::common::protocol::serialization_server::deserialize_ip;
use std::net::IpAddr;

impl ClientData {
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
