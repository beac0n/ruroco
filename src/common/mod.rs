#[cfg(target_os = "android")]
pub(crate) mod android;
pub(crate) mod crypto;
pub(crate) mod fs;
/// the server <-> commander IPC contract (Unix socket path + `CommanderData` wire format)
#[cfg(any(feature = "with-server", feature = "with-commander"))]
pub mod ipc;
pub(crate) mod logging;
pub(crate) mod protocol;

pub(crate) use crypto::blake2b_u64;
#[cfg(any(feature = "with-client", feature = "with-server"))]
pub use crypto::get_random_range;
#[cfg(any(feature = "with-client", feature = "with-server"))]
pub(crate) use crypto::handler as crypto_handler;
pub(crate) use fs::change_file_ownership;
pub(crate) use fs::resolve_path;
pub(crate) use logging::info;
#[cfg(any(feature = "with-client", feature = "with-server"))]
pub(crate) use protocol::client_data;
#[cfg(any(feature = "with-client", feature = "with-server"))]
pub(crate) use protocol::parser as data_parser;

#[cfg(any(feature = "with-server", feature = "with-commander"))]
pub(crate) fn normalize_ip(ip: std::net::IpAddr) -> std::net::IpAddr {
    match ip {
        std::net::IpAddr::V6(v6) => {
            v6.to_ipv4_mapped().map(std::net::IpAddr::V4).unwrap_or(std::net::IpAddr::V6(v6))
        }
        other => other,
    }
}

#[cfg(any(feature = "with-client", feature = "with-server"))]
pub(crate) fn now_nanos() -> anyhow::Result<u128> {
    use anyhow::Context;
    Ok(std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .with_context(|| "system clock before epoch")?
        .as_nanos())
}
