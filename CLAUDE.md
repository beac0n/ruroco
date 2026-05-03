# CLAUDE.md

## Project

Ruroco (Run Remote Command) — encrypted, one-way UDP remote command execution in Rust.
Client encrypts a command hash + counter with AES-256-GCM and sends a single 93-byte UDP packet.
Server decrypts, validates replay protection, then forwards to Commander via Unix socket IPC.

## Commands

```
make build        # Build all 4 binaries (each with different --features)
make test         # cargo nextest run --retries 2 (needs TEST_UPDATER=1)
make format       # cargo fix && cargo fmt && cargo clippy --tests -- -D warnings
make check        # cargo check --locked (with and without default features)
make coverage     # cargo tarpaulin --timeout 360
```

Binaries are built individually with `--no-default-features` plus specific feature flags.
Each binary needs different features: `with-client`, `with-gui`, `with-server`.

## Code Rules

- `anyhow::Result<T>` everywhere. Propagate with `?`, add context with `.with_context()`.
- **No panics in production code.** No `.unwrap()`, `.expect()`, `panic!()`, fallible indexing.
  `unwrap()` is only allowed in tests (`clippy.toml: allow-unwrap-in-tests = true`).
- `pub(crate)` over `pub` for internal items.
- Max line width: 100 chars. 4-space indent. Config in `rustfmt.toml`.
- All clippy warnings are errors in CI (`-D warnings`).
- Logging: use `info()`/`error()` from `src/common/logging.rs` (custom logger, no external crate).
  Both take `impl Display`: `info(format!(...))` or `info("literal")` — never `&format!(...)`.
- No unsafe code.

## Architecture

```
src/bin/           4 binaries: client, client_ui (Slint GUI), server, commander
src/client/        CLI parsing, key gen, UDP send, counter, lock, self-update, wizard
src/server/        UDP listener, config, commander IPC, blocklist (replay protection)
src/common/        Shared: crypto/, protocol/, logging.rs, fs.rs
src/ui/            Slint GUI + Android JNI bridge
```

- Server and Commander are separate processes (privilege separation via Unix socket).
- Client never knows actual commands — only sends Blake2b-64 hashes of command names.
- Server never sends responses (completely unidirectional).
- All IPs stored internally as IPv6-mapped format (16 bytes).
- Counter is nanosecond timestamp (u128), not sequential — gaps are expected.

## Protocol (do not change sizes without understanding full impact)

Defined in `src/common/protocol/constants.rs`:

- `MSG_SIZE` = 93 bytes: KEY_ID(8) + IV(12) + TAG(16) + CIPHERTEXT(57)
- `PLAINTEXT_SIZE` = 57, `CIPHERTEXT_SIZE` = 85, `KEY_ID_SIZE` = 8

## Testing

- Unit tests: inline `#[cfg(test)]` modules in source files.
- Integration tests: `tests/integration_test.rs` — uses `tempfile::tempdir()` for isolation.
- E2E: `scripts/test_end_to_end.sh` (systemd, requires sudo).
- Use `tempfile::tempdir()` for all test isolation; never hardcode paths.
- `ConfigServer` implements `Default` — use struct update syntax in tests.
- For HTTP download tests, use local `TcpListener` on port 0.
- Locale gotcha: don't parse `id` command output — system locale affects error messages.

## Features (conditional compilation)

```
default       = []                           # empty; all builds use --no-default-features
release-build = ["openssl/vendored"]
android-build = ["dep:ndk-context", "dep:jni"]
with-server   = ["dep:toml"]
with-gui      = ["dep:slint", "dep:slint-build", "dep:toml", "with-client"]
with-client   = ["dep:ureq", "dep:tempfile"]
```

## Key Files

- `config/config.toml` — example server config (commands receive `$RUROCO_IP` env var)
- `systemd/` — service files (socket activation on `[::]:80`)
- `.github/workflows/rust.yml` — CI pipeline
- `build.rs` — Slint compilation (only when `with-gui` enabled)

## After Code Changes

After every code change, run `make format && make test` to verify formatting and tests pass.

## On Compaction

Preserve: project overview, build commands, no-panic rule, protocol sizes, architecture layout,
feature flags, and test conventions. These are the most commonly needed during development.
