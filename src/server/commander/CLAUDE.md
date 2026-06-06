# src/server/commander/

The privileged executor (runs as root). `mod.rs` re-exports `run_commander` and the IPC types, so
callers keep using `crate::server::commander::{Commander, CommanderData, CMDR_DATA_SIZE}`.

- `mod.rs`: the `Commander` struct + accept loop (`run` -> `run_cycle` -> `read`). Reads a 24-byte
  message off the Unix socket, looks the `cmd_hash` up in its `cmds` map (built from
  `ConfigCommands`), and dispatches. Unknown hash -> error, no execution.
- `exec.rs`: socket lifecycle (`create_listener`, ownership/perms) and `run_command` (spawns
  `sh -c`, sets `$RUROCO_IP`, sanitizes the IP). `run_commander(CliCommander)` is the entry point.
- `data.rs`: `CommanderData` (cmd_hash[0:8] + ip[8:24]) and `CMDR_DATA_SIZE` = the Unix-socket wire
  format. Shared IPC: the server *produces* it (`handler.rs`), the commander *consumes* it; it
  lives here because the commander owns the socket.

The commander never touches crypto, keys, or the network. It trusts the Unix socket (see the
threat-model discussion in `.todo/03`).
