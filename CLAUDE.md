# CLAUDE.md

All CLAUDE.md files in this repo (root and nested) MUST be concise and short. Per-directory detail lives in nested
`src/**/CLAUDE.md` files, loaded on demand.

## Project

Ruroco (Run Remote Command): encrypted, one-way UDP remote command execution in Rust. Client sends a single 94-byte
AES-256-GCM-SIV packet; server decrypts, checks replay, forwards to
commander over a Unix socket.

## Commands

`make build` (all binaries) · `make test` · `make format` (fmt only) · `make lint_fix` (clippy -D warnings + cargo fix)
· `make check`. Full target list is in the `Makefile`; feature flags and binary mapping are in `Cargo.toml`. Each
binary builds with `--no-default-features` plus its own feature (`with-client`/`with-gui`/`with-server`).

## Code Rules

- `anyhow::Result<T>` everywhere. Propagate with `?`, add context with `.with_context()`.
- **No panics in production code.** No `.unwrap()`, `.expect()`, `panic!()`, fallible indexing.  `unwrap()` is allowed
  only in tests.
- `pub(crate)` over `pub` for internal items.
- Max line width 100, 4-space indent (`rustfmt.toml`). All clippy warnings are errors.
- Logging: `info()`/`debug()`/`error()` from `src/common/logging.rs` (custom logger). All take
  `impl Display`: `info(format!(...))` or `info("literal")`, never `&format!(...)`. `debug()` only
  prints when `RUROCO_LOG=debug`.
- No unsafe code, except the handful of audited FFI/syscall spots each carrying its own
  `#[allow(unsafe_code)]` with a SAFETY comment (systemd socket activation, signal handler
  registration, the Android JNI bridge); enforced by `#![deny(unsafe_code)]` in `src/lib.rs`.

## Architecture invariants

- Server and Commander are separate processes (privilege separation via Unix socket).
- Client never knows actual commands: only sends Blake2b-64 hashes of command names.
- Server never sends responses (completely unidirectional).
- All IPs stored internally as IPv6-mapped (16 bytes).
- Counter is a nanosecond timestamp (u128), not sequential; gaps are expected.

## Testing

- Unit tests: inline `#[cfg(test)]` modules (some moved to a sibling `<mod>_tests.rs` via `#[path]`
  when the inline module got large). Integration: `tests/integration_test.rs`.
  E2E: `scripts/test_end_to_end.sh` (systemd, sudo).
- Run tests via `make test` (nextest, process-per-test). Many tests mutate process-global env vars
  (`RUROCO_CONF_DIR`, `LISTEN_PID`, ...); plain `cargo test`'s in-process thread-per-test racing
  will flake on those. Integration tests additionally require the `testing` feature.
- Network-dependent tests (DNS resolution, GitHub API) are gated behind
  `#[test_with::env(TEST_ONLINE)]`; `make test` sets it, so they run by default there.
- Use `tempfile::tempdir()` for isolation; never hardcode paths. Keep the `TempDir` guard alive for
  as long as anything on disk under it is used (e.g. a `Server` holding an open blocklist).
- `ConfigServer` implements `Default`: use struct update syntax in tests.
- HTTP download tests: local `TcpListener` on port 0.
- Locale gotcha: don't parse `id` output, system locale affects error messages.
- **No Android CI/e2e coverage.** `android-build` only compiles under `target_os = "android"`; no
  workflow cross-compiles it or runs it on an emulator/device, so a change that compiles and passes
  every test here can still be silently broken on Android (e.g. code that assumes a writable
  platform temp dir - see `src/common/android/CLAUDE.md`). Reason explicitly about Android code
  paths when touching anything under `with-gui`/`android-build`; don't rely on this suite to catch it.

## Repo etiquette

When adding a CLI subcommand to `ruroco-client`, update `README.md`: add a row to the commands
table under `### commands` and a `### <command>` section under `## client usage`.
