# CLAUDE.md

All CLAUDE.md files in this repo (root and nested) MUST be concise and short. Per-directory detail lives in nested
`src/**/CLAUDE.md` files, loaded on demand.

## Project

Ruroco (Run Remote Command): encrypted, one-way UDP remote command execution in Rust. Client sends a single 93-byte
AES-256-GCM packet; server decrypts, checks replay, forwards to
commander over a Unix socket.

## Commands

`make build` (all binaries) · `make test` · `make format` · `make check`. Full target list is in the `Makefile`; feature
flags and binary mapping are in `Cargo.toml`. Each binary builds with `--no-default-features` plus its own feature (
`with-client`/`with-gui`/`with-server`).

## Code Rules

- `anyhow::Result<T>` everywhere. Propagate with `?`, add context with `.with_context()`.
- **No panics in production code.** No `.unwrap()`, `.expect()`, `panic!()`, fallible indexing.  `unwrap()` is allowed
  only in tests.
- `pub(crate)` over `pub` for internal items.
- Max line width 100, 4-space indent (`rustfmt.toml`). All clippy warnings are errors.
- Logging: `info()`/`error()` from `src/common/logging.rs` (custom logger). Both take
  `impl Display`: `info(format!(...))` or `info("literal")`, never `&format!(...)`.
- No unsafe code.

## Architecture invariants

- Server and Commander are separate processes (privilege separation via Unix socket).
- Client never knows actual commands: only sends Blake2b-64 hashes of command names.
- Server never sends responses (completely unidirectional).
- All IPs stored internally as IPv6-mapped (16 bytes).
- Counter is a nanosecond timestamp (u128), not sequential; gaps are expected.

## Testing

- Unit tests: inline `#[cfg(test)]` modules. Integration: `tests/integration_test.rs`.
  E2E: `scripts/test_end_to_end.sh` (systemd, sudo).
- Use `tempfile::tempdir()` for isolation; never hardcode paths.
- `ConfigServer` implements `Default`: use struct update syntax in tests.
- HTTP download tests: local `TcpListener` on port 0.
- Locale gotcha: don't parse `id` output, system locale affects error messages.

## Repo etiquette

When adding a CLI subcommand to `ruroco-client`, update `README.md`: add a row to the commands
table under `### commands` and a `### <command>` section under `## client usage`.
