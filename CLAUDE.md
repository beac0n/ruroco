# CLAUDE.md

## Quick Reference

Build: `make build`
Test: `make test` (runs `cargo nextest run --retries 2` with `TEST_UPDATER=1`)
Lint: `make format` (runs `cargo fix && cargo fmt && cargo clippy --tests --verbose -- -D warnings`)
Check: `make check` (runs `cargo check --locked` with and without default features)
Coverage: `make coverage`

All clippy warnings are treated as errors in CI (`-D warnings`).

## Project

Ruroco (Run Remote Command) — encrypted one-way UDP remote command execution.

```
ruroco-client --UDP(AES-256-GCM)--> ruroco-server --Unix socket--> ruroco-commander
```

Four binaries: `src/bin/client.rs`, `src/bin/client_ui.rs` (Slint GUI), `src/bin/server.rs`, `src/bin/commander.rs`.

Four modules: `src/client/`, `src/server/`, `src/common/` (crypto, protocol, fs, logging), `src/ui/` (Slint + Android).

## Code Conventions

- `anyhow::Result<T>` for all error handling. Propagate with `?`, add context with `.with_context(|| "...")`, use `bail!`/`anyhow!` for explicit errors.
- Prefer `pub(crate)` over `pub` for internal items.
- Max line width: 100 chars. 4-space indent. Full config in `rustfmt.toml`.
- **No panics in production code.** Never use `.unwrap()`, `.expect()`, `panic!()`, array indexing that can go out of bounds, or any other method that can panic. Always use fallible alternatives (e.g. `?`, `.ok_or()`, `.get()`, `.try_into()`). `unwrap()` is only allowed in test code (`allow-unwrap-in-tests = true` in `clippy.toml`).
- Logging: use `info()`/`error()` from `src/common/logging.rs` (custom minimal logger, no external crate).
- No unsafe code.

## Protocol (do not change sizes without understanding the full impact)

Defined in `src/common/protocol/constants.rs`:
- `MSG_SIZE` = 93 bytes (fixed packet size: 8B key ID + 12B IV + 16B tag + 57B ciphertext)
- `PLAINTEXT_SIZE` = 57, `CIPHERTEXT_SIZE` = 85, `KEY_ID_SIZE` = 8

## Crypto

- AES-256-GCM encryption via `openssl` crate (`src/common/crypto/handler.rs`)
- Key derivation: PBKDF2-HMAC-SHA256, 100k iterations
- Command names hashed with Blake2b-64 — never sent over the wire
- Replay prevention: monotonic counter per key ID, persisted to `blocklist.toml`

## Testing

- Unit tests: inline `#[cfg(test)]` modules in source files
- Integration tests: `tests/integration_test.rs` — uses `tempfile::tempdir()` for isolation
- End-to-end: `scripts/test_end_to_end.sh` (systemd services, requires sudo)
- Fixtures: `tests/conf_dir/` (keys/config), `tests/files/` (sample TOMLs)

## Build

- Nix for reproducible environments: `nix-shell nix/linux.nix --pure`
- Features: `release-build` (vendors OpenSSL), `android-build` (Slint Android backend)
- Release profile optimizes for size: `opt-level = "z"`, `strip = true`, `lto = true`, `panic = 'abort'`
- CI: GitHub Actions (`.github/workflows/rust.yml`) — check, typos, test, e2e test, coverage, format, release on `v*` tags

## Configuration

Server config: `config/config.toml`. Commands receive client IP via `$RUROCO_IP` env var.
Client state: `~/.config/ruroco/counter` (u128 big-endian), `~/.config/ruroco/client.lock` (file mutex).
Systemd service files in `systemd/` (socket activation on `[::]:80`, strict sandboxing).
