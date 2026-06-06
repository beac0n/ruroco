# src/client/wizard/

Interactive server setup (run as root). `core.rs` = flow, `wizard_systemd.rs` = unit paths + data.

Flow: run a forced self-update of the server binaries, then write three systemd units (`ruroco.service`,
`ruroco-commander.service`, `ruroco.socket`), `/etc/ruroco/config.toml`, and `/etc/ruroco/commands.toml` (both mode
`0o600`), all embedded at compile time via `include_bytes!` from `systemd/` and `config/`. `commands.toml` is the
commander-only command set and stays `root`-owned (never chowned to the `ruroco` user). Each config file is only written
if missing (idempotent). Finishes with daemon-reload + enable + start.

Note: paths under `/etc` are hard-coded constants in `wizard_systemd.rs`; tests exercise the file-writing helpers
against a `tempdir`, not the real `/etc`.
