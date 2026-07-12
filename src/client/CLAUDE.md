# src/client/

CLI client. Entry: `run_client(CliClient)` parses args and matches the `CommandsClient` enum
(Gen, Send, Update, Wizard, Reseed), dispatching to `Sender`, `Generator`, `Updater`, `Wizard`.

Submodules: `config/` (clap schema + conf-dir), `send/` (UDP), `update/` (self-update),
`wizard/` (server setup). Loose: `gen.rs` (key gen), `util.rs`, plus:

- `counter.rs`: monotonic replay counter, persisted as a raw big-endian `u128` (stable, unversioned:
  the layout is a single fixed-width integer with no room to change incompatibly); increment is
  overflow-checked; seeded to `now_nanos()` only when the counter file doesn't exist yet (first
  use). Any other read failure (corrupt/truncated file, permission problem, ...) is a hard error,
  not a silent reseed - overwriting an unreadable file could mask real corruption or move a
  legitimately future-dated counter backwards, which would then have every subsequent send
  rejected as a replay by the server. `ruroco-client reseed` is the explicit way to reset it.
- `lock.rs`: single-instance guard at `<conf_dir>/client.lock`, an exclusive non-blocking
  `flock(2)` on a persistent file (never removed) rather than a PID file - atomic by construction,
  so there is no stale-lock state to detect or clean up, and a crashed process releases its lock
  automatically when the kernel closes its file descriptors. Held for the whole invocation in both
  `run_client` and the GUI's `run_ui`/`run_ui_with_options` (not just around `send`) **by design**:
  it guards every mutation the client can make (config edits, key gen/rotation, binary self-update,
  the on-disk counter), so the GUI and CLI correctly refuse to run concurrently at all. Do not
  "fix" this by scoping the lock to the send/counter path only - that reopens races on the others.

Invariants: client only sends Blake2b-64 hashes of command names, never the commands. All paths
go through the conf dir (`config::get_conf_dir`). `anyhow::Result` + `.with_context()` throughout;
no panics in production.

Tests: `set_test_conf_dir()` returns a `tempfile::tempdir()` and sets `RUROCO_CONF_DIR` to isolate
each test; update tests are gated behind the `TEST_ONLINE` env (real network).
