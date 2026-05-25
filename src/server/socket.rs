use crate::common::info;
use crate::server::config::ConfigServer;
use anyhow::{anyhow, Context};
use std::env;
use std::net::UdpSocket;
use std::os::fd::{FromRawFd, RawFd};

/// Default UDP port ruroco listens on when no address is supplied via argument,
/// `RUROCO_LISTEN_ADDRESS`, or systemd socket activation.
///
/// Derived from the alphabet indices of the letters in "ruroco":
/// r=18, u=21, r=18, o=15, c=3, o=15 → distinct values multiplied together × 2:
/// 18 × 21 × 15 × 3 × 2 = 34020
pub(crate) const DEFAULT_PORT: u16 = 34020;

impl ConfigServer {
    pub(crate) fn create_server_udp_socket(
        &self,
        address: Option<String>,
    ) -> anyhow::Result<UdpSocket> {
        match (
            env::var("LISTEN_PID").ok(),
            env::var("LISTEN_FDS").ok(),
            env::var("RUROCO_LISTEN_ADDRESS").ok(),
            address,
        ) {
            (_, _, _, Some(address)) => {
                info(format!("UdpSocket bind to {address} - argument"));
                UdpSocket::bind(&address)
                    .with_context(|| format!("Could not UdpSocket bind {address:?}"))
            }
            (_, _, Some(address), _) => {
                info(format!("UdpSocket bind to {address} - RUROCO_LISTEN_ADDRESS"));
                UdpSocket::bind(&address)
                    .with_context(|| format!("Could not UdpSocket bind {address:?}"))
            }
            (Some(listen_pid), Some(listen_fds), _, _)
                if listen_pid == std::process::id().to_string() && listen_fds == "1" =>
            {
                let fd: RawFd = 3;
                info(format!("UdpSocket from_raw_fd {fd} (systemd socket activation)"));
                // SAFETY: systemd socket activation guarantees that FD 3 is the first
                // passed socket when LISTEN_FDS=1 and LISTEN_PID matches the current
                // process (both checked above). Ownership of the fd transfers to the
                // returned UdpSocket; nothing else in this process will close it.
                let sock = unsafe { UdpSocket::from_raw_fd(fd) };
                Ok(sock)
            }
            (Some(_), Some(listen_fds), _, _) if listen_fds != "1" => {
                Err(anyhow!("LISTEN_FDS was set to {listen_fds}, expected 1"))
            }
            (Some(listen_pid), Some(_), _, _) if listen_pid != std::process::id().to_string() => {
                Err(anyhow!("LISTEN_PID ({listen_pid}) does not match current PID"))
            }
            _ => {
                let address = format!("[::]:{}", DEFAULT_PORT);
                info(format!("UdpSocket bind to {address} - fallback"));
                UdpSocket::bind(&address)
                    .with_context(|| format!("Could not UdpSocket bind {address:?}"))
            }
        }
    }
}
