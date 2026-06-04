# Commander

The commander is the privileged half of the receiving side. It owns the Unix domain socket, reads
the 24-byte `CommanderData` the server writes, looks the command up by its Blake2b-64 hash, and
runs the configured shell command with the client IP exported into the environment. This chapter
covers `commander.rs`, `commander_data.rs`, `commander_exec.rs`, and `util.rs`.

## `commander_data.rs`: the 24-byte wire format

This is the exact message sent across the Unix socket from server to commander.

```rust
pub(crate) const CMDR_DATA_SIZE: usize = 24;

pub(crate) struct CommanderData {
    pub(crate) cmd_hash: u64,
    pub(crate) ip: IpAddr,
}
```

### Layout

| Bytes | Field | Encoding |
| --- | --- | --- |
| `[0:8]` | `cmd_hash` | `u64` big-endian (`to_be_bytes`) |
| `[8:24]` | `ip` | 16 bytes, IPv6-mapped (`serialize_ip`) |

### Conversions

```rust
impl From<CommanderData> for [u8; CMDR_DATA_SIZE] {
    fn from(value: CommanderData) -> Self {
        let mut data = [0u8; CMDR_DATA_SIZE];
        data[..8].copy_from_slice(&value.cmd_hash.to_be_bytes());
        data[8..].copy_from_slice(&serialize_ip(&value.ip));
        data
    }
}

impl From<[u8; CMDR_DATA_SIZE]> for CommanderData {
    fn from(data: [u8; CMDR_DATA_SIZE]) -> Self {
        // cmd_hash = u64::from_be_bytes(data[0..8])
        // ip = deserialize_ip(data[8..24])  -> normalize_ip applied
    }
}
```

`serialize_ip` writes IPv4 as its IPv6-mapped form, so the IP is always 16 bytes regardless of
family. `deserialize_ip` reverses this and runs `normalize_ip`, so an IPv4 client IP arrives at the
commander as a plain `IpAddr::V4`. The conversions are infallible: the buffer is a fixed 24 bytes.

## `commander.rs`: the `Commander` struct and accept loop

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
pub(super) fn create_from_path(path: &Path) -> anyhow::Result<Commander>; // read TOML
pub fn create(config: ConfigServer) -> anyhow::Result<Commander>;
```

`create` builds `cmds` via `config.get_hash_to_cmd()` (hashing each command **name** with
`blake2b_u64`), derives the socket path from `config.config_dir`, and copies the
`socket_user`/`socket_group`. The hash map is what makes the commander able to resolve an incoming
`cmd_hash` without ever knowing what the client typed.

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

It binds the listener once, then serves connections forever. A per-connection error (unknown command,
read failure) is logged via `error(...)` and the loop continues; one bad message never takes the
commander down.

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

An unknown `cmd_hash` is an error (logged, connection dropped). `read` fills a fixed 24-byte buffer.

### Command lookup

The lookup is `self.cmds.get(&cmd_hash)`. The hashes were computed at startup from the config's
command names. A hash with no matching name produces `"Unknown command name: {hash}"`. This is the
point where the opaque hash the client sent is finally resolved to a concrete shell string, and it
happens only inside the privileged process.

## `commander_exec.rs`: socket setup and shell execution

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
- Permissions are set to **`0o204`**: owner (the server user) has write, others have read, and no
  execute. The comment in the source reads "only server should be able to write, everyone else can
  read". This is the access-control boundary: only the server may push commands in.
- Ownership is applied via `change_socket_ownership` -> `change_file_ownership(path, socket_user,
  socket_group)` (both trimmed). Defaults are user `ruroco` / group `ruroco`.

### Shell execution with `$RUROCO_IP`

```rust
const ENV_PREFIX: &str = "RUROCO_";

pub(super) fn run_command(&self, command: &str, ip: IpAddr) {
    if Self::sanitize_ip(ip) { return; }           // reject suspicious IP
    Command::new("sh")
        .arg("-c")
        .arg(command)
        .env(format!("{ENV_PREFIX}IP"), ip.to_string()) // RUROCO_IP=<client ip>
        .output();
    // logs stdout/stderr; info on success, error on non-zero exit or spawn failure
}
```

The configured command string is run through `sh -c`, with the environment variable `RUROCO_IP` set
to the client IP, so a command can react to who triggered it (for example
`ufw allow from $RUROCO_IP`). Output is captured: on success both stdout and stderr are logged at
info level, on a non-zero exit they are logged at error level, and a spawn failure is logged as
`"Error executing {command}: {e}"`. A failing command is never fatal to the commander loop.

### IP sanitization

```rust
fn sanitize_ip(ip: IpAddr) -> bool {
    let ip_str = ip.to_string();
    if !ip_str.chars().all(|c| c.is_ascii_hexdigit() || c == '.' || c == ':') {
        error(...); // "refusing to execute with suspicious IP"
        true        // true => abort, do not run
    } else { false }
}
```

Before the IP is placed into `RUROCO_IP` and the command is run, the string form is checked to
contain only hex digits, dots, and colons (the only characters valid in an IPv4 or IPv6 textual
address). Anything else aborts execution. Since the value comes from a parsed `IpAddr` this is
belt-and-braces defense against shell injection through the environment variable.

### `run_commander` entry point

```rust
pub fn run_commander(server: CliServer) -> anyhow::Result<()> {
    Commander::create_from_path(&server.config)?.run()
}
```

This is the `commander` binary's main path: load the TOML config, build the `Commander`, and serve
forever.

## `util.rs`: socket path

```rust
pub fn get_commander_unix_socket_path(config_dir: &Path) -> PathBuf {
    common::resolve_path(config_dir).join("ruroco.socket")
}
```

The single source of truth for the Unix socket location. Both the server (when deciding where to
connect) and the commander (when deciding where to bind) call this with their `config_dir`, so they
always agree: `<resolved config_dir>/ruroco.socket`. `resolve_path` is applied first so a relative
config dir resolves consistently on both sides.

## Gotchas

- The socket mode is `0o204`, not a more common value: server writes, world reads, owner cannot read
  back. Confirm the server process runs as the `socket_user` so it actually holds the write bit.
- The commander must be able to bind in `config_dir`; the directory is created with
  `create_dir_all` if absent.
- `cmd_hash` is the hash of the command **name**, computed identically on the config side
  (`get_hash_to_cmd`) and on the client. A mismatch in the name produces "Unknown command name".
- All execution happens in the commander, never the server, and the commander never touches the
  network: the only input it trusts is the 24-byte message on its own Unix socket.
