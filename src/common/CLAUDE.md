# src/common/

Code shared by client, server, commander, and ui. Subdirs: `crypto/`, `protocol/`, `android/`.

- `ipc.rs`: the server <-> commander IPC contract - `CommanderData` (cmd_hash[0:8] + ip[8:24]) plus
  `get_commander_unix_socket_path`. Server produces + connects, commander consumes + binds. Gated
  behind `any(with-server, with-commander)`; no crypto/network deps. (The config structs are *not*
  shared: `ConfigServer` lives in `server::config`, `ConfigCommander`/`ConfigCommands` in
  `commander::config`; they just read the same `config.toml`/`commands.toml` files.)

Cross-cutting helpers worth knowing:

- `logging.rs`: the project's own logger. `info()`/`debug()`/`error()` take `impl Display`, so
  pass owned values: `info(format!(...))` or `info("literal")`, never `info(&format!(...))` (that
  borrows a temporary and reads worse). `debug()` only prints when `RUROCO_LOG=debug`
  (case-insensitive, read once). ANSI colors only on a TTY; millisecond UTC timestamps. No
  external log crate.
- `fs.rs`: `write_atomic()`/`write_atomic_with_mode()` (temp file + `fsync` + rename, optional
  permission bits; used for counter/blocklist/commands list and the self-update binary swap; gated
  behind `any(with-client, with-server)`, and their tests carry the same gate),
  `resolve_path()`, and ownership helpers (`nix`).
- `blake2b_u64()` (command-name hashing, shared by client and server) lives in `crypto/mod.rs`
  and is re-exported via `common`; `mod.rs` has `normalize_ip()` (collapse IPv6-mapped IPv4 back
  to v4).
