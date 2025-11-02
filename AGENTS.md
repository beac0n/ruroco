# Repository Guidelines

## Project Structure & Module Organization
The crate exposes four binaries in `src/bin/`, backed by modules under `src/client`, `src/server`, `src/ui`, `src/common`, and helpers in `src/config`. Integration fixtures live in `tests/conf_dir` and `tests/files`; the primary suite is `tests/integration_test.rs`. Deployment samples reside in `config/`, `fail2ban/`, and `systemd/`. Automation scripts sit in `scripts/`, the Makefile wraps common workflows, and `target/` holds build output that must stay untracked.

## Build, Test, and Development Commands
- `cargo build --locked` compiles all binaries locally; `make build` cross-compiles for the release triple.
- `cargo run --bin client_ui` (or `make dev_ui_local`) opens the desktop UI for smoke tests.
- `make release` triggers the Linux and Android packaging scripts in `scripts/`.
- `cargo check --locked` / `make check` runs fast type checks, including the `--no-default-features` pass.

## Coding Style & Naming Conventions
`cargo fmt` (configured in `rustfmt.toml`) enforces 4-space indentation and a 100-character width; run it before committing. Follow Rust norms: snake_case files and functions, PascalCase types, SCREAMING_SNAKE_CASE constants. `cargo clippy --tests -- -D warnings` (`make format`) must pass; prefer fixing lints over adding allows.

## Testing Guidelines
Use `TEST_UPDATER=1 cargo nextest run --retries 2` (or `make test`) to execute the suite and refresh PEM fixtures. `make coverage` runs tarpaulin and emits `coverage.xml`. Test functions stay snake_case and live near the code they cover or under `tests/` for cross-component flows. Add regression tests when touching protocol handling, cryptography, or command execution.

## Commit & Pull Request Guidelines
Commits use short, lowercase imperatives (`add dashboard update`, `trim key before use`) and focus on a single concern. Reference related issues in the body when needed. Pull requests should include a summary, test commands executed, and screenshots for UI tweaks. Highlight updates to release artifacts, systemd units, or sample configs so reviewers can validate deployment impact.

## Security & Configuration Tips
Sample server settings live in `config/config.toml`; never commit real keys and document new requirements. Mirror production adjustments by updating `fail2ban/` and `systemd/` assets. When adding client-server commands, reuse the RSA helpers and update onboarding notes if key paths or permissions shift.
