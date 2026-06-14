# ipc.rs

`src/common/ipc.rs` is the single thing the server and commander processes must agree on at
runtime: where their Unix socket lives and what flows over it. It lives in `common` (rather than in
either role's module) because both depend on it, and it carries no crypto or network code, so the
commander can link it without OpenSSL. It is gated behind `any(with-server, with-commander)`.

## The 24-byte wire format

```rust
pub(crate) const CMDR_DATA_SIZE: usize = 24;

pub(crate) struct CommanderData {
    pub(crate) cmd_hash: u64,
    pub(crate) ip: IpAddr,
}
```

| Bytes | Field | Encoding |
| --- | --- | --- |
| `[0:8]` | `cmd_hash` | `u64` big-endian (`to_be_bytes`) |
| `[8:24]` | `ip` | 16 bytes, IPv6-mapped (`serialize_ip`) |

The `From` conversions are infallible (the buffer is a fixed 24 bytes): one direction writes
`cmd_hash.to_be_bytes()` then `serialize_ip(&ip)`, the other reads them back and runs `normalize_ip`
on the IP, so an IPv4 client IP arrives at the commander as a plain `IpAddr::V4`. The server
*produces* a `CommanderData` (in `handler.rs`) and connects to the socket; the commander *consumes*
it and binds the socket.

## The socket path

```rust
pub fn get_commander_unix_socket_path(config_dir: &Path) -> PathBuf {
    resolve_path(config_dir).join("ruroco.socket")
}
```

Both the server (when deciding where to connect) and the commander (when deciding where to bind)
call this with their `config_dir`, so they always agree: `<resolved config_dir>/ruroco.socket`.
`resolve_path` is applied first so a relative config dir resolves consistently on both sides.

## One config file, two views

`config_dir` is the one configuration value that *must* match between the two processes (otherwise
they resolve different socket paths and the IPC silently breaks). It is therefore kept in a single
shared `config.toml` file read by both. But the two processes do **not** share a config struct: each
deserializes the same file through its own view, declaring only the fields it uses and ignoring the
rest.

- `server::config::ConfigServer` reads the server-only fields (`ips`, rate limit, clock skew) plus
  `config_dir`. See [Config and keys](../server/config-keys.md).
- `commander::config::ConfigCommander` reads `config_dir` plus the commander-only `socket_user` /
  `socket_group`. See [Commander](../commander.md).

The command set is a separate file again (`commands.toml`, `ConfigCommands`), read only by the
commander, so the network-facing server never loads it.
