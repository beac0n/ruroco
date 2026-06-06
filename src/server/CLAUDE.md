# src/server/

Two processes for privilege separation:

- **server** (unprivileged): owns the UDP socket, decrypts, validates, writes to the Unix socket.
- **commander** (root): owns the Unix socket, looks up the command by hash, and runs it.

Request flow: `socket.rs` receives a 93-byte datagram (supports systemd socket activation, falls
back to binding `[::]`) -> decrypt via `CryptoHandler` -> `RateLimiter::check` (per-IP, ~1s window,
`max_requests_per_second`, default 2) -> deserialize `ClientData` -> validate: replay (blocklist),
`dst_ip` in config, strict src_ip match -> on success, persist the new counter and send a 24-byte
`CommanderData` (cmd_hash[0:8] + ip[8:24]) to the commander. The commander runs the configured
shell command with `$RUROCO_IP` set to the client IP. The server never replies.

Config (`config.rs`): `ConfigServer` (shared by both processes) holds allowed `ips`, `config_dir`,
`socket_user`/`socket_group`, rate limit. Commands live in a separate `ConfigCommands`
(name -> shell string), loaded only by the commander via `ConfigCommands::create_from_path`. The
server CLI (`CliServer`) takes `--config`; the commander CLI (`CliCommander`) takes both `--config`
and `--commands` (default `/etc/ruroco/commands.toml`), so the command file is relocatable
independently of the server config. The network-facing server never reads commands. Install
`commands.toml` `root`-owned `0600` so the unprivileged server user cannot read the command set.
Commands are looked up by `blake2b_u64` of the name. IPs are `normalize_ip`'d on load.

Gotchas:
- Blocklist (`blocklist.rs`, msgpack-persisted) stores the max counter per key_id and blocks
  `counter <= last_seen` (the check is `>=`, so an *equal* counter is a replay). On startup every
  key's floor is seeded to `now_nanos()`, so packets older than process start are rejected.
  `handler.rs` also rejects `counter > now_nanos() + max_clock_skew_seconds` (default 3600) without
  touching the blocklist, so a future-dated packet can't permanently lock out a key.
- `rate_limiter.rs` is in-memory only (resets on restart); it is throttling, not replay defense.
- Strict mode only enforces src_ip when the client set `strict` and included a src_ip.
- `signal.rs` traps SIGTERM/SIGINT into an atomic checked each loop for clean shutdown.

Tests: encrypt a packet into `client_recv_data` via a helper, then assert validation/replay; an
integration test spins a commander thread and checks the command actually runs.
