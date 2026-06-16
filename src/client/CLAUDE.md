# src/client/

CLI client. Entry: `run_client(CliClient)` parses args and matches the `CommandsClient` enum
(Gen, Send, Update, Wizard, Reseed), dispatching to `Sender`, `Generator`, `Updater`, `Wizard`.

Submodules: `config/` (clap schema + conf-dir), `send/` (UDP), `update/` (self-update),
`wizard/` (server setup). Loose: `gen.rs` (key gen), `util.rs`, plus:

- `counter.rs`: monotonic replay counter, persisted as a raw big-endian `u128` (stable, unversioned:
  the layout is a single fixed-width integer with no room to change incompatibly); increment is
  overflow-checked; seeded to `now_nanos()` on first use.
- `lock.rs`: PID-based single-instance lock at `<conf_dir>/client.lock`; stale locks (dead PID)
  are auto-removed so a crashed run never wedges the next one.

Invariants: client only sends Blake2b-64 hashes of command names, never the commands. All paths
go through the conf dir (`config::get_conf_dir`). `anyhow::Result` + `.with_context()` throughout;
no panics in production.

Tests: `set_test_conf_dir()` returns a `tempfile::tempdir()` and sets `RUROCO_CONF_DIR` to isolate
each test; update tests are gated behind the `TEST_UPDATER` env (real network).
