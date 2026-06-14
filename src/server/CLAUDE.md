# src/server/

The network-facing daemon (unprivileged, `with-server`): owns the UDP socket, decrypts, validates,
and writes to the Unix socket. All modules here are server-only: `listener.rs` (the `Server` struct
+ run loop), `socket.rs`, `handler.rs`, `blocklist.rs`, `rate_limiter.rs`, `signal.rs`, `keys.rs`.

This is one half of the privilege separation; the **commander** (root) is the top-level
`src/commander/` module (separate process, separate binary, `with-commander` feature - no OpenSSL,
no UDP/decrypt code). The only thing shared between the two processes is the IPC contract
(`CommanderData` + the Unix socket path) in `src/common/ipc.rs`. `config.toml` is one physical file
read by both, but each side has its own view: `server::config::ConfigServer` (here) reads the
server-only fields, `commander::config::ConfigCommander` reads the commander-only ones; only
`config_dir` overlaps (so both resolve the same socket). `keys.rs`/`socket.rs` hang the server-only
inherent methods off `ConfigServer` (crypto handlers, UDP socket, blocklist) via separate `impl`
blocks.

Request flow: `socket.rs` receives a 93-byte datagram (supports systemd socket activation, falls
back to binding `[::]`) -> decrypt via `CryptoHandler` -> `RateLimiter::check` (per-IP, ~1s window,
`max_requests_per_second`, default 2) -> deserialize `ClientData` -> validate: replay (blocklist),
`dst_ip` in config, strict src_ip match -> on success, persist the new counter and send a 24-byte
`CommanderData` (cmd_hash[0:8] + ip[8:24]) to the commander. The commander runs the configured
shell command with `$RUROCO_IP` set to the client IP. The server never replies.

Config: `ConfigServer` (the server's view of `config.toml`, in `server::config`: `ips`,
`config_dir`, rate limit, clock skew). The commander reads the same file via its own
`ConfigCommander`, and the command set lives in a separate `commands.toml` (`ConfigCommands`,
`commander::config`). IPs are `normalize_ip`'d on load.

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
