# src/client/wizard/

Interactive server setup (run as root). `core.rs` = flow, `wizard_systemd.rs` = unit paths + data.

Flow: run a forced self-update of the server binaries, then write three systemd units
(`ruroco.service`, `ruroco-commander.service`, `ruroco.socket`) and `/etc/ruroco/config.toml`
(mode `0o600`), all embedded at compile time via `include_bytes!` from `systemd/` and `config/`.
The config is only written if missing (idempotent). Finishes with daemon-reload + enable + start.

Note: paths under `/etc` are hard-coded constants in `wizard_systemd.rs`; tests exercise the
file-writing helpers against a `tempdir`, not the real `/etc`.
