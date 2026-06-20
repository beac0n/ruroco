# Commander

The commander is the privileged half of the receiving side: a separate process and binary from the
server, typically run as root. It owns the Unix domain socket, reads the 24-byte `CommanderData` the
server writes, looks the command up by its Blake2b-64 hash, and runs the configured shell command
with the client IP exported into the environment.

It lives in the top-level `src/commander/` module and builds under the `with-commander` feature,
which links **no** OpenSSL and none of the server's UDP/decrypt code (`with-server` is a superset of
`with-commander`, since the server produces the IPC type the commander consumes). The module is
three files plus the shared types it imports from `common`:

- `mod.rs`: the `Commander` struct and accept loop.
- `exec.rs`: socket setup, shell execution, and the `run_commander` entry point.
- `config.rs`: `ConfigCommander` (the commander's view of `config.toml`), `ConfigCommands` (the
  `commands.toml` schema), and `CliCommander`.

The only thing shared with the server is the IPC contract in `common::ipc`: the wire format
(`CommanderData`, `CMDR_DATA_SIZE`) and the socket path (`get_commander_unix_socket_path`). See
[ipc.rs](./common/ipc.md). The server's own config lives in `server::config::ConfigServer`.

## `config.rs`: `ConfigCommander`, `ConfigCommands`, `CliCommander`

The commander reads **two** files: the shared `config.toml` (for `config_dir` and the socket
ownership) and its own `commands.toml`. Both paths are configurable (`--config` / `--commands`) so
the command set can be relocated independently of the server config.

```rust
#[derive(Parser, Debug)]
pub struct CliCommander {
    #[arg(short, long, default_value = "/etc/ruroco/config.toml")]
    pub(crate) config: PathBuf,
    #[arg(long, default_value = "/etc/ruroco/commands.toml")]
    pub(crate) commands: PathBuf,
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct ConfigCommander {
    #[serde(default = "default_config_path")]   // /etc/ruroco
    pub config_dir: PathBuf,
    #[serde(default)]                           // None -> falls back to config_dir
    pub socket_dir: Option<PathBuf>,
    #[serde(default = "default_socket_user")]   // "ruroco"
    pub socket_user: String,
    #[serde(default = "default_socket_group")]  // "ruroco"
    pub socket_group: String,
    #[serde(default)]                           // false: reject non-routable client IPs
    pub allow_non_routable_ips: bool,
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct ConfigCommands {
    pub commands: HashMap<String, String>, // command name -> shell string
}
```

`ConfigCommander` is the commander's view of `config.toml`: it declares only the fields it uses
(`config_dir` for the socket path, `socket_user`/`socket_group` for socket ownership). All its fields
are optional, so the server-only fields in the same file (`ips`, rate limit, clock skew) are simply
ignored. `config_dir` is the one value that must agree with `ConfigServer` (see
[ipc.rs](./common/ipc.md)).

The command set is kept in its own file, `commands.toml`, separate from `config.toml`, so the
network-facing server process never loads it. It is installed `root`-owned `0600`.

```rust
pub(crate) fn get_hash_to_cmd(&self) -> anyhow::Result<HashMap<u64, String>> {
    self.commands
        .iter()
        .map(|(k, v)| {
            let hash = blake2b_u64(k)?; // hash of the command NAME
            Ok((hash, v.clone()))
        })
        .collect()
}
```

`get_hash_to_cmd` turns the name-keyed config into a hash-keyed lookup table. The incoming
`CommanderData.cmd_hash` is matched against these `u64` keys. The hash is computed over the command
**name** (the map key), not the shell string, identically to how the client computes it, so the
client never has to transmit the command itself.

## `mod.rs`: the `Commander` struct and accept loop

```rust
#[derive(Debug, PartialEq)]
pub struct Commander {
    pub(super) socket_path: PathBuf,
    pub(super) cmds: HashMap<u64, String>, // cmd_hash -> shell string
    pub(super) socket_user: String,
    pub(super) socket_group: String,
}
```

### Construction

```rust
pub(super) fn create_from_paths(config_path: &Path, commands_path: &Path) -> anyhow::Result<Commander>;
pub fn create(config: ConfigCommander, commands: ConfigCommands) -> anyhow::Result<Commander>;
```

`create` builds `cmds` via `commands.get_hash_to_cmd()`, derives the socket path from
`get_commander_unix_socket_path(&config.config_dir)`, and copies the `socket_user`/`socket_group`
from the `ConfigCommander`. `create_from_paths` loads both TOML files
(`ConfigCommander::create_from_path` and `ConfigCommands::create_from_path`) and forwards to
`create`.

### Accept loop

```rust
pub fn run(&self) -> anyhow::Result<()> {
    for stream in self.create_listener()?.incoming() {
        match stream {
            Ok(mut stream) => if let Err(e) = self.run_cycle(&mut stream) { error(e) },
            Err(e) => error(format!("Connection for {:?} failed: {e}", &self.socket_path)),
        }
    }
    Ok(())
}
```

It binds the listener once, then serves connections forever. A per-connection error (unknown
command, read failure) is logged via `error(...)` and the loop continues; one bad message never
takes the commander down.

### Per-connection cycle

```rust
fn run_cycle(&self, stream: &mut UnixStream) -> anyhow::Result<()> {
    let msg = Commander::read(stream)?;            // [u8; 24]
    let cmdr_data: CommanderData = msg.into();
    let cmd = self.cmds.get(&cmdr_data.cmd_hash)
        .ok_or_else(|| anyhow!("Unknown command name: {cmd_hash}"))?;
    info(format!("Running command ({cmd_hash}) {cmd}"));
    self.run_command(cmd, cmdr_data.ip);
    Ok(())
}

fn read(stream: &mut UnixStream) -> anyhow::Result<[u8; CMDR_DATA_SIZE]>;
```

`read` fills a fixed 24-byte buffer. The lookup `self.cmds.get(&cmd_hash)` is the point where the
opaque hash the client sent is finally resolved to a concrete shell string, and it happens only
inside the privileged process. A hash with no matching name produces `"Unknown command name:
{hash}"` (logged, connection dropped).

## `exec.rs`: socket setup and shell execution

### Socket creation, permissions, ownership

```rust
pub(super) fn create_listener(&self) -> anyhow::Result<UnixListener> {
    let socket_dir = self.socket_path.parent()
        .ok_or_else(|| ... "Could not get parent dir ...")?;
    fs::create_dir_all(socket_dir)?;
    let _ = fs::remove_file(&self.socket_path);    // clear stale socket
    let mode = 0o204;                              // write-only for server, read for others
    let listener = UnixListener::bind(&self.socket_path)?;
    fs::set_permissions(&self.socket_path, Permissions::from_mode(mode))?;
    self.change_socket_ownership()?;
    Ok(listener)
}
```

- The parent directory is created if missing.
- Any stale socket file at the path is removed before binding (binding fails if the path exists).
- Permissions are set to **`0o204`**: owner (the server user) has write, others have read, no
  execute. This is the access-control boundary: only the server may push commands in.
- Ownership is applied via `change_socket_ownership` -> `change_file_ownership(path, socket_user,
  socket_group)` (both trimmed). Defaults are user `ruroco` / group `ruroco`.

### Shell execution with `$RUROCO_IP`

```rust
const ENV_PREFIX: &str = "RUROCO_";

pub(super) fn run_command(&self, command: &str, ip: IpAddr) {
    if !self.allow_non_routable_ips && !Self::is_ip_allowed(ip) { return; } // reject non-routable
    Command::new("sh")
        .arg("-c")
        .arg(command)
        .env(format!("{ENV_PREFIX}IP"), ip.to_string()) // RUROCO_IP=<client ip>
        .output();
    // logs stdout/stderr; info on success, error on non-zero exit or spawn failure
}
```

The configured command string is run through `sh -c`, with `RUROCO_IP` set to the client IP, so a
command can react to who triggered it (for example `ufw allow from $RUROCO_IP`). Output is captured:
on success both stdout and stderr are logged at info level, on a non-zero exit at error level, and a
spawn failure is logged as `"Error executing {command} for {ip}: {e}"` (the client IP is included in
every execution log line for an audit trail). A failing command is never fatal to the commander loop.

### IP filtering

```rust
fn is_ip_allowed(ip: IpAddr) -> bool {
    let reject = ip.is_unspecified() || ip.is_loopback() || ip.is_multicast()
        || match ip {
            IpAddr::V4(v4) => v4.is_broadcast() || v4.is_private()
                || v4.is_link_local() || v4.is_documentation(),
            IpAddr::V6(v6) => v6.is_unique_local() || v6.is_unicast_link_local(),
        };
    if reject { error(...); } // "refusing to execute with non-routable IP"
    !reject
}
```

The IP placed into `RUROCO_IP` is meant to be an outside unicast peer (for example for
`ufw allow from $RUROCO_IP`), so by default only globally-routable addresses run the command:
unspecified, loopback, multicast, broadcast, private/RFC1918, link-local, and documentation
addresses are rejected. This stops a client from naming `127.0.0.1` or an internal address to
whitelist a host it does not own. Setting `allow_non_routable_ips = true` in `config.toml` bypasses
the filter (mainly for local testing, where the only available source address is loopback).

### `run_commander` entry point

```rust
pub fn run_commander(commander: CliCommander) -> anyhow::Result<()> {
    Commander::create_from_paths(&commander.config, &commander.commands)?.run()
}
```

This is the `commander` binary's main path: load both TOML files, build the `Commander`, and serve
forever.

## Gotchas

- The socket mode is `0o204`, not a more common value: server writes, world reads, owner cannot read
  back. Confirm the server process runs as the `socket_user` so it actually holds the write bit.
- The commander must be able to bind in `config_dir`; the directory is created with
  `create_dir_all` if absent.
- `cmd_hash` is the hash of the command **name**, computed identically on the config side
  (`get_hash_to_cmd`) and on the client. A mismatch in the name produces "Unknown command name".
- All execution happens in the commander, never the server; and the commander never touches the
  network or links OpenSSL: the only input it trusts is the 24-byte message on its own Unix socket.
