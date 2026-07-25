# src/client/wizard/

Interactive server setup (run as root). `core.rs` = flow, `wizard_systemd.rs` = unit paths + data.

Flow: create the `ruroco` user (`useradd --system --user-group`, so a matching group exists for the chowns below and
for the unit's `Group=`), run a forced self-update of the server binaries, write three systemd units
(`ruroco.service`, `ruroco-commander.service`, `ruroco.socket`), then create `/etc/ruroco` and the two config files
(embedded at compile time via `include_bytes!` from `systemd/` and `config/`). Each config file is only written if
missing (idempotent), but ownership and mode are re-applied on every run. Finishes with daemon-reload, enable,
reset-failed, **restart**.

`restart`, not `start`: the wizard replaces the server binaries (`Updater`) and rewrites the unit files, and `start`
on an already-running unit is a no-op, so a wizard re-run would install new binaries while the old processes keep
running and still report success. `restart` covers the fresh-install case too.

**The server is only started when a `*.key` already exists** (`has_key_file`, mirroring the server's own
`get_key_paths` filter). Without a key the server exits at startup, and `Restart=always` turns that into a crash loop
that trips systemd's start limit in about a second, leaving a failed unit that refuses to start even after the admin
adds the key. So a keyless run starts only the commander and socket and tells the admin to `systemctl start
ruroco.service` once the key is in place. `reset-failed` runs first to clear exactly that leftover state from an older
install.

Ownership scheme, and why the server can actually read its config:

| path | owner | mode | rationale |
| --- | --- | --- | --- |
| `/etc/ruroco` | `root:ruroco` | `0750` | server must traverse it to reach its `.key`; nothing else may list it |
| `config.toml` | `root:ruroco` | `0640` | read by commander (root) *and* server (`User=ruroco`), writable by neither |
| `commands.toml` | `root:root` | `0600` | commander-only; the server must never enumerate the commands it triggers |

Modes are always set explicitly rather than left to the umask: a root shell with `umask 0077` would otherwise leave
`/etc/ruroco` at `0700`, untraversable for the `ruroco` user. Root keeps ownership of everything so a compromised
server cannot rewrite its own config or drop in a key, which is the DAC-level counterpart to the read-only
`/etc/ruroco` that `ruroco.service`'s `ProtectSystem=strict` already enforces. `set_ownership` names `root` as the
owner explicitly rather than relying on the wizard's euid, so it also repairs a path a hand-edit or an older wizard
left owned by someone else.

The `.key` file is *not* installed by the wizard (the admin generates it with `ruroco-client gen` and copies it in), so
the wizard's closing instructions spell out the `chown root:ruroco` + `chmod 640` it needs. Without that the server
crash-loops on EACCES.

Note: paths under `/etc` are hard-coded constants in `wizard_systemd.rs`; tests exercise the file-writing helpers
against a `tempdir`, not the real `/etc`. The chowns need root, so they are not unit-tested.
