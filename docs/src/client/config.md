# Client Configuration and CLI Schema

This chapter documents the two files that define the client's command-line
interface and its configuration-directory resolution:

- `src/client/config/mod.rs`: the top-level clap parser `CliClient`, the
  `CommandsClient` subcommand enum, and `get_conf_dir`.
- `src/client/config/commands.rs`: the per-subcommand argument structs
  (`GenCommand`, `ReseedCommand`, `SendCommand`, `UpdateCommand`, `WizardCommand`)
  and the `Default` impl for `SendCommand`.

## `config/mod.rs`

### Constant

```rust
pub(crate) const DEFAULT_COMMAND: &str = "default";
```

This is the fallback command name used when `--command` is not supplied to
`send`, and the value `SendCommand::default()` uses for its `command` field.

### `CliClient`

```rust
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct CliClient {
    #[command(subcommand)]
    pub(crate) command: CommandsClient,
}
```

`CliClient` is the root clap `Parser`. It carries the chosen subcommand in
`command`. `#[command(version, ...)]` wires up `--version` and `--help`. The
field is `pub(crate)`: external crates construct a `CliClient` only by parsing
(for example `CliClient::parse_from(...)` or `CliClient::try_parse_from(...)`).

### `CommandsClient`

```rust
#[derive(Debug, Subcommand)]
pub(crate) enum CommandsClient {
    /// Generate a shared AES key (base64 with embedded key id).
    Gen(GenCommand),
    /// Send a command to a specific address.
    Send(SendCommand),
    /// Update the client binary
    Update(UpdateCommand),
    /// Run the wizard to set up the server side.
    Wizard(WizardCommand),
    /// Reseed the replay-protection counter to the current timestamp.
    Reseed(ReseedCommand),
}
```

The doc comment on each variant becomes the subcommand's help text. The variant
names map to lowercase subcommand names (`gen`, `send`, `update`, `wizard`,
`reseed`). `run_client` matches exhaustively on this enum (see `overview.md`).

### `get_conf_dir`

```rust
pub(crate) fn get_conf_dir() -> anyhow::Result<PathBuf>
```

This is a thin platform dispatcher:

- On Linux it calls `get_conf_dir_linux()`.
- On Android it calls `get_conf_dir_android()`, which delegates to
  `AndroidUtil::create()?.get_conf_dir()`.
- On every other platform it returns `Err(anyhow!("unsupported platform"))`.

#### Linux resolution logic

```rust
#[cfg(target_os = "linux")]
fn get_conf_dir_linux() -> anyhow::Result<PathBuf>
```

The directory is chosen in strict priority order:

1. If the `RUROCO_CONF_DIR` environment variable is set, use it verbatim as the
   path. This is the hook tests use to isolate state.
2. Otherwise, if `HOME` is set, use `$HOME/.config/ruroco`.
3. Otherwise, fall back to the current working directory
   (`env::current_dir()`), adding the context `Could not determine config dir`
   on failure.

After selecting the path, the function calls
`fs::create_dir_all(&path)` and adds the context `Could not create config dir`
if that fails. It then returns the path. Two consequences worth noting:

- Calling `get_conf_dir()` has the side effect of creating the directory tree.
- If the chosen path cannot be created (for example because a parent component is
  a regular file), the function returns an error containing
  `Could not create config dir`.

### Tests in `config/mod.rs`

The inline test module verifies: `--help` produces clap's `DisplayHelp` error;
the env-var, `$HOME`, and no-`HOME` resolution branches; the create-failure path
(pointing `RUROCO_CONF_DIR` at a path under `/etc/hostname`, which is a file);
and `SendCommand::default()` field values.

## `config/commands.rs`

This file defines one struct per subcommand. Empty structs exist so that clap can
still attach `--help` to subcommands that take no arguments.

### `GenCommand`

```rust
#[derive(Parser, Debug)]
pub(crate) struct GenCommand {}
```

No arguments. Selecting `gen` runs the key generator.

### `ReseedCommand`

```rust
#[derive(Parser, Debug)]
pub(crate) struct ReseedCommand {}
```

No arguments. Selecting `reseed` rewrites the counter file to `now_nanos()`.

### `SendCommand`

This is the main command struct and the only one that is `pub` (re-exported as
`crate::client::config::SendCommand`), because `Sender` and external callers
construct it directly.

```rust
#[derive(Parser, Debug)]
pub struct SendCommand {
    #[arg(short, long)]
    pub address: String,
    #[arg(short, long)]
    pub key: String,
    #[arg(short, long, default_value = DEFAULT_COMMAND)]
    pub command: String,
    #[arg(short = 'e', long)]
    pub permissive: bool,
    #[arg(short, long)]
    pub ip: Option<String>,
    #[arg(short = '4', long)]
    pub ipv4: bool,
    #[arg(short = '6', long)]
    pub ipv6: bool,
    #[arg(short = 'd', long, default_value = "50")]
    pub send_delay_ms: u64,
}
```

Field-by-field:

| Field | Flags | Type | Default | Meaning |
| --- | --- | --- | --- | --- |
| `address` | `-a`, `--address` | `String` | (required) | Destination to send the command to. A hostname, IPv4, or IPv6 literal. A missing port is filled in by `Sender::ensure_port` (default port 80). |
| `key` | `-k`, `--key` | `String` | (required) | Base64 key with embedded key id, the output of `gen` or the UI. Decoded into an 8-byte id plus a 32-byte AES key. |
| `command` | `-c`, `--command` | `String` | `"default"` | The command *name* to invoke. Only its Blake2b-64 hash is sent. |
| `permissive` | `-e`, `--permissive` | `bool` | `false` | Allow permissive IP validation: the server-side source IP need not match the provided `--ip`. Inverted into the packet's `strict` flag (`strict = !permissive`). |
| `ip` | `-i`, `--ip` | `Option<String>` | `None` | Optional source IP (or CIDR-like literal) from which the command is claimed to be sent. Parsed with `.parse()`; an unparseable value silently becomes "no source IP". |
| `ipv4` | `-4`, `--ipv4` | `bool` | `false` | Restrict the destination to IPv4 addresses. |
| `ipv6` | `-6`, `--ipv6` | `bool` | `false` | Restrict the destination to IPv6 addresses. |
| `send_delay_ms` | `-d`, `--send-delay-ms` | `u64` | `50` | Milliseconds to sleep between datagrams when more than one destination IP is used (for example sending to both an IPv4 and an IPv6 address). |

Notes:

- The `--ip` help text suggests using `-6ei "dead:beef:dead:beef::/64"` to allow a
  whole IPv6 network, and gives a one-liner to derive that automatically from
  `api64.ipify.org`.
- `ipv4` and `ipv6` together (or neither) mean "no family restriction"; the
  resolver treats `ipv4 == ipv6` as the undefined case. See `send.md` for the
  exact family-filter table.

### `SendCommand` Default impl

```rust
impl Default for SendCommand {
    fn default() -> SendCommand {
        SendCommand {
            address: "127.0.0.1:1234".to_string(),
            key: "FFFFFFFF...DEADBEEF...".to_string(), // 80-char base64 placeholder
            command: DEFAULT_COMMAND.to_string(),
            permissive: false,
            ip: None,
            ipv4: false,
            ipv6: false,
            send_delay_ms: 50,
        }
    }
}
```

The default `key` is a fixed 80-character base64 placeholder (an all-`F` key id
followed by repeated `DEADBEEF`). The `Default` impl exists primarily so tests can
build a `SendCommand` with struct-update syntax (`SendCommand { key, ..Default::default() }`).
Whenever a field is added to `SendCommand`, this impl must be updated too.

### `UpdateCommand`

```rust
#[derive(Parser, Debug)]
pub(crate) struct UpdateCommand {
    #[arg(short, long)]
    pub(crate) force: bool,
    #[arg(short, long)]
    pub(crate) version: Option<String>,
    #[arg(short, long)]
    pub(crate) bin_path: Option<PathBuf>,
    #[arg(short, long)]
    pub(crate) server: bool,
}
```

| Field | Flags | Type | Meaning |
| --- | --- | --- | --- |
| `force` | `-f`, `--force` | `bool` | Force the update even when versions match. |
| `version` | `-v`, `--version` | `Option<String>` | Target version (for example `v0.14.2`). |
| `bin_path` | `-b`, `--bin-path` | `Option<PathBuf>` | Directory where binaries are written. |
| `server` | `-s`, `--server` | `bool` | Update the server-side binary instead of the client. |

These fields are unpacked by `run_client` and passed to `Updater::create`.

### `WizardCommand`

```rust
#[derive(Parser, Debug)]
pub(crate) struct WizardCommand {
    #[arg(short, long)]
    pub(crate) force: bool,
}
```

A single `-f`/`--force` flag. The wizard subsystem consumes it.
