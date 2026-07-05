# src/commander/

The privileged executor (runs as root), a separate process/binary from the server for privilege
separation. Builds under `with-commander`, which links **no** OpenSSL and none of the UDP/decrypt
path (`with-server` is a superset of it). It trusts the Unix socket; see the threat model in
`.todo/03`.

- `mod.rs`: the `Commander` struct + accept loop (`run` -> `run_cycle` -> `read`). Reads a 24-byte
  `CommanderData` off the Unix socket, looks the `cmd_hash` up in its `cmds` map (built from
  `ConfigCommands`), and dispatches. Unknown hash -> error, no execution. Re-exports
  `run_commander`, `CliCommander`, `ConfigCommands`.
- `exec.rs`: socket lifecycle (`create_listener`, ownership/perms) and `run_command` (spawns
  `sh -c`, sets `$RUROCO_IP`, sanitizes the IP). Execution is sequential (no threads) with a
  timeout: stdout/stderr go to temp files (never pipes, so a chatty command can't dead-lock the
  poll loop), `try_wait` is polled every 50ms, and at the deadline the `sh` process (only, not its
  group) gets SIGKILL and is reaped. `run_commander(CliCommander)` is the entry point.
- `config.rs`: `ConfigCommander` (the commander's view of the shared `config.toml`: `config_dir` +
  `socket_user`/`socket_group`, ignoring the server-only fields), `ConfigCommands` (the
  `commands.toml` name -> shell map, looked up by `blake2b_u64`), and `CliCommander` (`--config` and
  `--commands`). The command set is read **only** by the commander. A commands.toml value is either
  a plain string (default 30s timeout) or `{ cmd = "...", timeout_sec = N }` (untagged serde enum);
  `get_hash_to_cmd` resolves both into `CommandSpec { cmd, timeout }`. Tests build the map via
  `ConfigCommands::from_map`.

The one thing shared with the server lives in `common::ipc`: the IPC contract `CommanderData` +
`get_commander_unix_socket_path` - the server *produces* `CommanderData` and connects; the commander
*consumes* it and binds. `config.toml` is read by both processes, but each through its own struct
(`ConfigCommander` here, `ConfigServer` in `server::config`); `config_dir` (and the optional
`socket_dir`, which relocates the socket to e.g. a `RuntimeDirectory`) overlap.
