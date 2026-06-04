# Counter, Lock, Generator, and Util

This chapter covers the four leaf modules of the client core:

- `src/client/counter.rs`: the persisted, monotonic replay counter.
- `src/client/lock.rs`: the PID-based single-instance lock.
- `src/client/gen.rs`: the AES key generator.
- `src/client/util.rs`: filesystem permission helpers.

## `counter.rs`

The counter is the client side of replay protection. It is a `u128` nanosecond
timestamp persisted to `<conf_dir>/counter` as 16 raw big-endian bytes. Every
send increments and rewrites it, so the value the server sees as its replay floor
only ever moves forward.

### `Counter`

```rust
#[derive(Debug)]
pub struct Counter {
    path: PathBuf,
    count: u128,
}
```

Both fields are private. `path` is the on-disk file; `count` is the in-memory
value.

### `Counter::create_and_init`

```rust
pub fn create_and_init(path: PathBuf, initial: u128) -> anyhow::Result<Self>
```

Constructs the counter and loads its starting value:

1. Build `Self { path, count: 0 }`.
2. Try `read()`. If reading the existing file succeeds, `count` is set to the
   stored value (the file wins).
3. If `read()` fails (typically because the file does not exist yet), set
   `count = initial` and `write()` it, propagating any write error.

Callers pass `now_nanos()` as `initial`, so a brand-new counter is seeded to the
current nanosecond timestamp; a pre-existing counter keeps its persisted value.

### `Counter::count`

```rust
pub(crate) fn count(&self) -> u128
```

Returns the current in-memory value. `Sender::get_data_to_encrypt` uses this to
fill the packet's `counter` field.

### `Counter::inc`

```rust
pub(crate) fn inc(&mut self) -> anyhow::Result<()>
```

Increments with `checked_add(1)`. On overflow (the value has reached
`u128::MAX`) it returns an error reading
`counter overflow: value has reached u128::MAX (<MAX>) and cannot be incremented`
rather than wrapping. On success it persists the new value via `write()`.
`Sender::send_data` calls this before assembling each datagram.

### `Counter::reseed`

```rust
pub fn reseed(path: PathBuf, value: u128) -> anyhow::Result<()>
```

Constructs a throwaway `Counter { path, count: value }` and writes it, overwriting
whatever was on disk. This is the `reseed` subcommand's mechanism; `run_client`
calls it with `now_nanos()` to reset the counter to the current time.

### Private persistence helpers

```rust
fn write(&self) -> anyhow::Result<()>
fn read(&mut self) -> anyhow::Result<()>
```

- `write` calls `File::create(path)` (truncating) and `write_all(&count.to_be_bytes())`,
  with contexts `Could not create counter file <path>` and
  `Could not write counter file <path>`.
- `read` opens the file, `read_exact` into a `[0u8; 16]` buffer, then
  `u128::from_be_bytes`. Contexts: `Could not open counter file <path>` and
  `Could not read counter file <path>`.

Gotchas:

- The format is exactly 16 raw bytes (no text, no newline). Any tooling that
  inspects the file must treat it as a big-endian `u128`.
- Because the value is a nanosecond timestamp, gaps between successive counters
  are expected and normal; the counter is not a sequential message index.
- `create_and_init` will not reset an existing file: re-initializing reads the
  stored value and ignores `initial`.

## `lock.rs`

The lock guarantees at most one client run touches the conf dir at a time. It is a
PID file at `<conf_dir>/client.lock` with automatic stale-lock cleanup.

### `ClientLock`

```rust
pub(crate) struct ClientLock {
    path: PathBuf,
    file: Option<File>,
}
```

`path` is the lock file path; `file` is the held handle (wrapped in `Option` so
`Drop` can take and close it before removing the file, which matters on Windows).

### `ClientLock::acquire`

```rust
pub(crate) fn acquire(path: PathBuf) -> anyhow::Result<Self>
```

1. Try `Self::open(&path)`, which uses `OpenOptions::new().create_new(true).write(true)`
   so it fails with `AlreadyExists` if the file is already there.
2. On `AlreadyExists`: read the file, parse its contents as a `u32` PID. If that
   PID `is_pid_running`, bail with `Client already running (lock at <path>)`.
   Otherwise the lock is stale: remove the file and re-`open` it, adding the
   context `Client lock unavailable at <path> after cleanup` on failure.
3. On any other open error: bail with `Client lock unavailable at <path>: <e>`.
4. Write the current process id (`std::process::id()`) into the file (best-effort:
   the result of `writeln!` is ignored) and return the held `ClientLock`.

### `ClientLock::open`

```rust
fn open(path: &PathBuf) -> io::Result<File>
```

The `create_new(true)` open primitive that makes acquisition atomic: it both
creates the file and signals contention via `AlreadyExists`.

### `is_pid_running` (per platform)

```rust
fn is_pid_running(pid: u32) -> bool
```

Implemented behind `#[cfg(target_os = ...)]`:

- **Linux**: checks whether `/proc/<pid>` exists.
- **Android**: always `false` (the app runs at most once, so any existing lock is
  treated as stale).
- **macOS**: runs `ps -p <pid>` and checks for success.
- **Windows**: runs `tasklist /FI "PID eq <pid>"` and checks whether the PID
  appears in the output.
- **Other**: always `true` (conservatively assume the owner is alive, so an
  existing file blocks).

### `Drop for ClientLock`

```rust
impl Drop for ClientLock {
    fn drop(&mut self) {
        let _ = self.file.take();
        let _ = remove_file(&self.path);
    }
}
```

On drop it closes the file handle first (Windows-friendly) and then removes the
lock file. Because `run_client` holds the lock in a `_lock` binding for the whole
call, the file is removed when the function returns, even on the error path.

Gotchas:

- A lock with non-numeric or unparseable contents is treated as stale and cleaned
  up (the PID parse simply yields `None`).
- Acquisition fails with `Client lock unavailable` if the parent directory does
  not exist, since `create_new` cannot create the file.

## `gen.rs`

### `Generator`

```rust
pub struct Generator {}
```

A zero-field handle; it carries no state and exists so key generation has a
consistent `create`/`gen` shape matching the other client subsystems.

### `Generator::create`

```rust
pub fn create() -> anyhow::Result<Self>
```

Returns `Ok(Self {})`. It is fallible only for interface symmetry.

### `Generator::gen`

```rust
pub fn gen(&self) -> anyhow::Result<String>
```

Calls `CryptoHandler::gen_key()?`, prints the key to stdout with `print!`
(no trailing newline), and returns it. The key is a base64 string of 40 raw
bytes: an 8-byte random key id concatenated with a 32-byte (256-bit) random AES
key. Decoded, that is exactly 40 bytes; the inline test asserts
`key_decoded.len() == 40`. This base64 string is what the user passes back via
`send --key`.

## `util.rs`

A single filesystem helper used elsewhere in the client.

### `set_permissions`

```rust
pub(crate) fn set_permissions(path: &str, permissions_mode: u32) -> anyhow::Result<()>
```

Reads the file metadata (context `Could not get <path> meta data`), sets the Unix
mode bits to `permissions_mode` via `PermissionsExt::set_mode`, and applies them
with `fs::set_permissions` (context `Could not set file permissions for <path>`).
It is Unix-specific (`std::os::unix::fs::PermissionsExt`). The inline tests verify
round-tripping `0o644` and `0o600`, and that a nonexistent path returns an error
containing `Could not get`.
