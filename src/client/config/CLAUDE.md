# src/client/config/

Clap CLI schema + config-dir resolution.

- `CliClient` (clap `Parser`) holds the `CommandsClient` subcommand enum (Gen, Send, Update,
  Wizard, Reseed). `SendCommand` (in `commands.rs`) is the main one: `address`, `key_file` (path to
  a file holding the base64 key), `command` (default `"default"`), `ipv4`/`ipv6` family filters,
  `permissive`, `send_delay_ms`. There is no `key` field: `key_file` is the only way in, for the
  CLI and for every other caller (the GUI writes the in-memory key to a temp file before calling
  `Sender::create`) - never pass the key itself on the command line or hold it in `SendCommand`.
  `Sender::create` reads and trims `key_file` before building the `DataParser`.
- `get_conf_dir()`: `RUROCO_CONF_DIR` env, else `$HOME/.config/ruroco` (Linux); Android uses its
  own app dir. Creates the dir if missing.

To add a CLI subcommand: add a variant to `CommandsClient`, handle it in `run_client`, and update
`README.md` (commands table + `### <command>` section). To add a `Send` flag: add the field with
its `#[arg(...)]`, update the `Default` impl, and thread it through `Sender`.
