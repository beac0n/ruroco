# src/client/config/

Clap CLI schema + config-dir resolution.

- `CliClient` (clap `Parser`) holds the `CommandsClient` subcommand enum (Gen, Send, Update,
  Wizard, Reseed). `SendCommand` (in `commands.rs`) is the main one: `address`, `key` (base64),
  `command` (default `"default"`), `ipv4`/`ipv6` family filters, `permissive`, `send_delay_ms`.
- `get_conf_dir()`: `RUROCO_CONF_DIR` env, else `$HOME/.config/ruroco` (Linux); Android uses its
  own app dir. Creates the dir if missing.

To add a CLI subcommand: add a variant to `CommandsClient`, handle it in `run_client`, and update
`README.md` (commands table + `### <command>` section). To add a `Send` flag: add the field with
its `#[arg(...)]`, update the `Default` impl, and thread it through `Sender`.
