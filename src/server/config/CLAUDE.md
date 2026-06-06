# src/server/config/

Server-side config schemas, split per process. `mod.rs` re-exports everything, so callers keep
using `crate::server::config::{...}`.

- `server.rs`: `ConfigServer` (the shared `config.toml`: `ips`, `config_dir`, socket user/group,
  rate limit, clock skew) + `CliServer` (`--config`). Read by **both** processes. Holds the
  `default_*` helpers, `deserialize_ips` (normalizes to IPv6-mapped), and `create_from_path`.
  Inherent methods that act on `config_dir` (keys, UDP socket, blocklist, commander socket path)
  live in `keys.rs`/`socket.rs` as separate `impl ConfigServer` blocks, not here.
- `commander.rs`: `ConfigCommands` (the `commands.toml` name -> shell map, looked up by
  `blake2b_u64`) + `CliCommander` (`--config` and `--commands`). Read **only** by the commander.

Why split: the network-facing server must never load the command set. `commands.toml` is a
separate file, installed `root`-owned `0600`, relocatable via `--commands` independently of
`config.toml`. `ConfigServer::create_from_path` and `ConfigCommands::create_from_path` are
deliberately parallel (same `"Could not read {path:?}: {e}"` shape).
