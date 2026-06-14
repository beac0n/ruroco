# Config and Keys

This chapter covers the server's configuration (`config.rs`) and key file discovery / crypto handler
construction (`keys.rs`).

## `config.rs`: `ConfigServer` and `CliServer`

The server's view of `config.toml`. The commander reads the *same* file through its own
`ConfigCommander` view (see [Commander](../commander.md)); only `config_dir` is shared between the
two, and it must agree so both resolve the same `ruroco.socket` (see [ipc.rs](../common/ipc.md)).
The command set is a separate file (`commands.toml`), never loaded here.

```rust
#[derive(Parser, Debug)]
pub struct CliServer {
    #[arg(short, long, default_value = "/etc/ruroco/config.toml")]
    pub(crate) config: PathBuf,
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct ConfigServer {
    #[serde(deserialize_with = "deserialize_ips")]
    pub ips: Vec<IpAddr>,
    #[serde(default = "default_config_path")]            // /etc/ruroco
    pub config_dir: PathBuf,
    #[serde(default = "default_max_requests_per_second")] // 2
    pub max_requests_per_second: u32,
    #[serde(default = "default_max_clock_skew_seconds")]  // 3600
    pub max_clock_skew_seconds: u64,
}
```

- `ips`: the destination IPs this server answers for; a packet's `dst_ip` must be in this list
  (handler step 2). Defaults to `["127.0.0.1"]`. Each entry is run through `normalize_ip` on load
  (via `deserialize_ips`), so `"::ffff:127.0.0.1"` is stored as `127.0.0.1`.
- `config_dir`: directory holding the `*.key` files, `blocklist.msgpck`, and `ruroco.socket`.
  Defaults to `/etc/ruroco` from TOML, or the current working directory in `Default`.
- `max_requests_per_second`: per-IP rate limit, default 2.
- `max_clock_skew_seconds`: how far ahead of server-local time an accepted counter may be, default
  3600. See [handler.rs](./handler.md).

Note there is **no** `socket_user` / `socket_group` here: those are commander-only (the commander
chowns the socket), so they live in `ConfigCommander`. `ConfigServer` simply ignores them when they
appear in `config.toml`.

The inherent methods that act on `config_dir` (key discovery, crypto handlers, the UDP socket, the
blocklist) are separate `impl ConfigServer` blocks in `keys.rs` and `socket.rs`, covered below and
in [socket.rs and signal.rs](./socket-signal.md).

## `keys.rs`

### Responsibilities

Discovers every `*.key` file in `config_dir`, reads them, and builds one `CryptoHandler` per key,
indexed by the 8-byte key id. Supporting multiple keys lets several independent clients (each with
its own key) talk to one server. Also constructs the blocklist and resolves the Unix socket path
(via `common::ipc::get_commander_unix_socket_path`).

### Methods

```rust
pub(crate) fn create_blocklist(&self) -> anyhow::Result<Blocklist>;
pub(crate) fn create_crypto_handlers(&self)
    -> anyhow::Result<HashMap<[u8; KEY_ID_SIZE], CryptoHandler>>;
pub(crate) fn get_commander_unix_socket_path(&self) -> PathBuf; // convenience over common::ipc
pub(crate) fn resolve_config_dir(&self) -> PathBuf;
pub(crate) fn get_key_paths(&self) -> anyhow::Result<Vec<PathBuf>>;
```

These are server-only, so they do not compile into the commander build (which loads `ConfigServer`
for its fields only). `create_server_udp_socket` is the matching server-only method in
[socket.rs](./socket-signal.md).

### `*.key` discovery

```rust
pub(crate) fn get_key_paths(&self) -> anyhow::Result<Vec<PathBuf>> {
    let config_dir = self.resolve_config_dir();
    // read_dir, keep entries that are files with extension == "key"
    match key_files.len() {
        0 => Err(anyhow!("Could not find any .key files in {config_dir:?}")),
        _ => Ok(key_files),
    }
}
```

It filters `config_dir` for regular files whose extension is exactly `key`. A directory with no
`.key` files is an error (the server cannot start with no keys). A directory that cannot be read is
also an error: `"Error reading directory {dir}: {e}"`.

### Multiple keys, indexed by key id

```rust
pub(crate) fn create_crypto_handlers(&self) -> anyhow::Result<HashMap<[u8; KEY_ID_SIZE], CryptoHandler>> {
    let key_paths = self.get_key_paths()?;
    let content_to_path = Self::get_content_to_path(&key_paths)?; // HashMap<content, path>
    if key_paths.len() != content_to_path.len() {
        bail!("Duplicate key files detected; refusing to start");
    }
    // for each key: CryptoHandler::create(content), index by handler.id
}
```

Each key file is read to a `String`, a `CryptoHandler` is created from it, and the handlers are
collected into `HashMap<[u8; 8], CryptoHandler>` keyed by `handler.id` (the 8-byte key id that also
prefixes every datagram on the wire). The server uses this map in `decrypt` to pick the right
handler for an incoming packet's key id.

### Duplicate detection

`get_content_to_path` builds a `HashMap` keyed by file **content**, so two files with identical key
material collapse to one entry. If that shrinks the count relative to the number of key paths, the
server refuses to start with `"Duplicate key files detected; refusing to start"`. This guards
against copy-paste mistakes that would otherwise produce two key ids for the same secret.

### Gotchas

- The map is keyed by key **id** (`handler.id`), not by filename. The filename only matters for the
  `.key` extension filter.
- Two distinct files with the same content is a hard startup error, not a warning.
- `resolve_config_dir` runs `config_dir` through `resolve_path` before any filesystem access, so all
  of these methods (keys, blocklist, socket path) agree on the same resolved directory.
