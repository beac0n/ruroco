# src/common/

Code shared by client, server, commander, and ui. Subdirs: `crypto/`, `protocol/`, `android/`.

- `ipc.rs`: the server <-> commander IPC contract - `CommanderData` (cmd_hash[0:8] + ip[8:24]) plus
  `get_commander_unix_socket_path`. Server produces + connects, commander consumes + binds. Gated
  behind `any(with-server, with-commander)`; no crypto/network deps. (The config structs are *not*
  shared: `ConfigServer` lives in `server::config`, `ConfigCommander`/`ConfigCommands` in
  `commander::config`; they just read the same `config.toml`/`commands.toml` files.)

Cross-cutting helpers worth knowing:

- `logging.rs`: the project's own logger. `info()`/`error()` take `impl Display`, so pass owned
  values: `info(format!(...))` or `info("literal")`, never `info(&format!(...))` (that borrows a
  temporary and reads worse). No external log crate.
- `fs.rs`: `write_atomic()` (temp file + `fsync` + rename, used for counter/blocklist/commands
  list), `resolve_path()`, and ownership helpers (`nix`).
- `mod.rs`: `blake2b_u64()` (command-name hashing, shared by client and server) and
  `normalize_ip()` (collapse IPv6-mapped IPv4 back to v4).
