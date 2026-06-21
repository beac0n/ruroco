use crate::common::client_data::ClientData;
use crate::common::ipc::CommanderData;
use crate::common::logging::{error, info};
use crate::common::now_nanos;
use crate::server::Server;
use anyhow::{anyhow, Context};
use std::io::Write;
use std::net::IpAddr;
use std::os::unix::net::UnixStream;
use std::time::Duration;

impl Server {
    pub(super) fn validate_and_send_command(
        &mut self,
        key_id: [u8; crate::common::protocol::KEY_ID_SIZE],
        plaintext_data: [u8; crate::common::protocol::PLAINTEXT_SIZE],
        src_ip: IpAddr,
    ) -> anyhow::Result<()> {
        let max_future_counter = now_nanos()?
            .saturating_add(u128::from(self.config.max_clock_skew_seconds) * 1_000_000_000);

        match ClientData::deserialize(plaintext_data)? {
            client_data if self.blocklist.is_counter_replayed(key_id, client_data.counter) => {
                let server_counter = self.blocklist.get_counter(key_id);
                Err(anyhow!(
                    "Invalid counter for key {key_id:X?} - {} is on blocklist, expected > {server_counter:?}",
                    client_data.counter,
                ))
            }
            client_data if client_data.counter > max_future_counter => Err(anyhow!(
                "Future counter for key {key_id:X?} - {} exceeds now + skew ({max_future_counter}); not updating blocklist",
                client_data.counter
            )),
            client_data if !self.config.ips.contains(&client_data.dst_ip) => {
                let destination_ip = &client_data.dst_ip;
                let ips = &self.config.ips;
                Err(anyhow!("Invalid host IP for key {key_id:X?} - expected {ips:?} to contain {destination_ip}"))
            }
            client_data if client_data.is_source_ip_invalid(src_ip) => {
                let client_src_ip_str =
                    client_data.src_ip.map(|i| i.to_string()).unwrap_or("none".to_string());
                Err(anyhow!(
                    "Invalid source IP for key {:X?} - expected {client_src_ip_str}, actual {src_ip}",
                    key_id
                ))
            }
            client_data => {
                let cmd = client_data.cmd_hash;
                let server_counter = self.blocklist.get_counter(key_id);
                let client_counter = client_data.counter;
                let ip = client_data.src_ip.unwrap_or(src_ip);
                info(format!("Valid data for key {key_id:X?} - trying cmd {cmd} and counter {client_counter}|{server_counter:?} with {ip}"));
                // Persist the advanced counter before executing: if the blocklist can't be saved we
                // must not run the command, otherwise a replay could re-trigger it after a restart.
                self.update_block_list(key_id, client_data.counter)?;
                self.send_command(CommanderData { cmd_hash: cmd, ip });
                Ok(())
            }
        }
    }

    pub(super) fn update_block_list(
        &mut self,
        key_id: [u8; crate::common::protocol::KEY_ID_SIZE],
        counter: u128,
    ) -> anyhow::Result<()> {
        let previous = self.blocklist.get_counter(key_id).copied();
        self.blocklist.upsert(key_id, counter);
        if let Err(e) = self.blocklist.save() {
            // Persist failed: roll the in-memory advance back so this counter is not silently
            // consumed. The caller aborts before executing, so the client can retry the same
            // counter once the underlying issue (e.g. disk full) clears.
            match previous {
                Some(prev) => self.blocklist.upsert(key_id, prev),
                None => self.blocklist.remove(key_id),
            }
            return Err(e).with_context(|| "Could not update block list");
        }
        Ok(())
    }

    pub(super) fn send_command(&self, data: CommanderData) {
        match self.write_to_socket(data) {
            Ok(_) => info("Successfully sent data to commander"),
            Err(e) => error(format!(
                "Could not send data to commander via socket {:?}: {e}",
                &self.socket_path
            )),
        }
    }

    pub(super) fn write_to_socket(&self, data: CommanderData) -> anyhow::Result<()> {
        let mut stream = UnixStream::connect(&self.socket_path)
            .with_context(|| format!("Could not connect to socket {:?}", self.socket_path))?;
        // Bound the write so a hung commander can't stall the server's single-threaded loop. The
        // payload is tiny (24 bytes), so a second is generous for a healthy commander.
        stream
            .set_write_timeout(Some(Duration::from_secs(1)))
            .with_context(|| format!("Could not set write timeout for {:?}", self.socket_path))?;

        let data_to_send: [u8; crate::common::ipc::CMDR_DATA_SIZE] = data.into();
        stream.write_all(&data_to_send).with_context(|| {
            format!("Could not write {data_to_send:?} to socket {:?}", self.socket_path)
        })?;

        stream
            .flush()
            .with_context(|| format!("Could not flush stream for {:?}", self.socket_path))?;
        Ok(())
    }
}
