# Config and Keys

This chapter covers configuration loading (`config.rs`) and key file discovery / crypto handler
construction (`keys.rs`). Both are implemented as methods on `ConfigServer`, shared by the server
and the commander.

## `config.rs`

### CLI argument

```rust
#[derive(Parser, Debug)]
pub struct CliServer {
    #[arg(short, long, default_value = "/etc/ruroco/config.toml")]
    pub(crate) config: PathBuf,
}
```

Both `run_server` and `run_commander` take a `CliServer` and read the TOML config from this path.

### `ConfigServer`

```rust
#[derive(Debug, Deserialize, PartialEq)]
pub struct ConfigServer {
    pub commands: HashMap<String, String>, // command name -> shell string
    #[serde(deserialize_with = "deserialize_ips")]
    pub ips: Vec<IpAddr>,
    #[serde(default = "default_config_path")]   // /etc/ruroco
    pub config_dir: PathBuf,
    #[serde(default = "default_socket_user")]   // "ruroco"
    pub socket_user: String,
    #[serde(default = "default_socket_group")]  // "ruroco"
    pub socket_group: String,
    #[serde(default = "default_max_requests_per_second")] // 2
    pub max_requests_per_second: u32,
}
```

Field meanings:

- `commands`: maps a human command **name** to the **shell string** to run. The client never sees
  this; it only sends the Blake2b-64 hash of the name. Only the commander resolves it.
- `ips`: the set of destination IPs this server answers for. A packet's `dst_ip` must be in this
  list (handler step 2). Defaults to `["127.0.0.1"]`.
- `config_dir`: directory holding the `*.key` files, the `blocklist.msgpck`, and the
  `ruroco.socket`. Defaults to `/etc/ruroco` when loaded from TOML, or the current working
  directory in the `Default` impl.
- `socket_user` / `socket_group`: ownership applied to the Unix socket by the commander.
- `max_requests_per_second`: per-IP rate limit, default 2.

### IP normalization on load

```rust
fn deserialize_ips<'de, D>(d: D) -> Result<Vec<IpAddr>, D::Error> {
    let v: Vec<String> = Vec::<String>::deserialize(d)?;
    v.into_iter()
        .map(|s| {
            let ip: IpAddr = s.parse().map_err(serde::de::Error::custom)?;
            Ok(crate::common::normalize_ip(ip))
        })
        .collect()
}
```

Every configured IP is parsed and run through `normalize_ip`, which collapses an IPv6-mapped IPv4
address back to plain IPv4. So `"::ffff:127.0.0.1"` in the config deserializes to `127.0.0.1`, and
matches a packet whose `dst_ip` arrived as either form. An unparseable string is a deserialization
error.

### Command lookup by `blake2b_u64`

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

This is how the commander turns the name-keyed config into a hash-keyed lookup table. The incoming
`CommanderData.cmd_hash` is matched against these `u64` keys. The hash is computed over the command
**name** (the map key), not the shell string.

### Other methods and defaults

```rust
pub(crate) fn deserialize(data: &str) -> anyhow::Result<ConfigServer>; // toml::from_str
impl Default for ConfigServer; // commands empty, ips ["127.0.0.1"], config_dir = cwd, user/group "", rps = 2
fn default_config_path() -> PathBuf;          // /etc/ruroco
fn default_socket_user() -> String;           // "ruroco"
fn default_socket_group() -> String;          // "ruroco"
fn default_max_requests_per_second() -> u32;  // 2
```

`Default` is used heavily in tests with struct update syntax:

```rust
ConfigServer { config_dir, ..Default::default() }
```

### Gotchas

- The `Default` impl differs from the `serde` defaults: `Default` sets `socket_user`/`socket_group`
  to empty strings and `config_dir` to the current directory, whereas a TOML file with those keys
  omitted gets `"ruroco"`/`"ruroco"` and `/etc/ruroco` from the `#[serde(default = ...)]` functions.
- `commands` and `ips` have no serde default, so a config TOML must provide both (`[commands]` may
  be empty, but the key must be present).

## `keys.rs`

### Responsibilities

Discovers every `*.key` file in `config_dir`, reads them, and builds one `CryptoHandler` per key,
indexed by the 8-byte key id. Supporting multiple keys lets several independent clients (each with
its own key) talk to one server. Also constructs the blocklist and resolves the Unix socket path.

### Methods

```rust
pub(crate) fn create_blocklist(&self) -> anyhow::Result<Blocklist>;
pub(crate) fn create_crypto_handlers(&self)
    -> anyhow::Result<HashMap<[u8; KEY_ID_SIZE], CryptoHandler>>;
pub(crate) fn get_commander_unix_socket_path(&self) -> PathBuf;
pub(crate) fn resolve_config_dir(&self) -> PathBuf;
pub(crate) fn get_key_paths(&self) -> anyhow::Result<Vec<PathBuf>>;
```

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
