# Handler and Validation

`handler.rs` implements the validation and dispatch half of the `Server`. It is reached from the
receive loop after the datagram has been received, size-checked, rate limited, and decrypted. Its
job is to deserialize the plaintext into `ClientData`, run the three validation checks, persist the
counter, and hand a `CommanderData` to the commander over the Unix socket.

All functions here are `impl Server` methods with `pub(super)` visibility (callable from the parent
`server` module but not outside the crate's server tree).

## Entry point: `validate_and_send_command`

```rust
pub(super) fn validate_and_send_command(
    &mut self,
    key_id: [u8; KEY_ID_SIZE],   // KEY_ID_SIZE == 8
    plaintext_data: [u8; PLAINTEXT_SIZE], // PLAINTEXT_SIZE == 57
    src_ip: IpAddr,
) -> anyhow::Result<()>
```

It first deserializes the plaintext:

```rust
ClientData::deserialize(plaintext_data)
```

`ClientData::deserialize` validates the protocol version byte (the first byte of the authenticated
plaintext) and then reads fixed offsets out of the 58-byte buffer; an unknown version is rejected.
The resulting struct is then matched against guard clauses, evaluated top to bottom. The first guard
that matches produces an error and the packet is dropped; if none match, the success arm runs.

### Step 1: replay check

```rust
client_data if self.blocklist.is_counter_replayed(key_id, client_data.counter) => ...
```

If `is_counter_replayed` returns `true`, the packet is rejected with:

```
Invalid counter for key {key_id:X?} - {counter} is on blocklist, expected > {server_counter:?}
```

`is_counter_replayed` uses a `>=` comparison against the stored per-`key_id` counter, so a counter
equal to or below the last accepted value is a replay. The counter is a u128 nanosecond timestamp,
so a freshly generated packet is normally strictly greater than the stored floor. See
[Blocklist and rate limiter](./blocklist-ratelimiter.md).

### Step 2: destination IP check

```rust
client_data if !self.config.ips.contains(&client_data.dst_ip) => ...
```

The `dst_ip` the client encoded into the packet must be one of the server's configured `ips`. If it
is not, the packet is rejected with:

```
Invalid host IP for key {key_id:X?} - expected {ips:?} to contain {destination_ip}
```

Both sides are compared as `IpAddr`. Config IPs are normalized at load time and `dst_ip` is
normalized during deserialization, so an IPv4 address and its IPv6-mapped form compare equal. This
check binds a captured packet to a specific destination host: replaying it against a different
server IP fails.

### Step 3: strict source IP check

```rust
client_data if client_data.is_source_ip_invalid(src_ip) => ...
```

`is_source_ip_invalid` is defined as:

```rust
pub(crate) fn is_source_ip_invalid(&self, source_ip: IpAddr) -> bool {
    self.strict && self.src_ip.is_some_and(|ip_sent| ip_sent != source_ip)
}
```

It only rejects when **both** conditions hold: the client set the `strict` flag, and it included a
`src_ip` that does not match the real UDP source address (`src_ip` here is the `normalize_ip`'d
sender from the receive loop). If `strict` is false, or no `src_ip` was sent, this check passes.
Rejection message:

```
Invalid source IP for key {key_id:X?} - expected {client_src_ip_str}, actual {src_ip}
```

where `client_src_ip_str` is the sent src_ip or the literal `"none"`.

### Success arm

When no guard matched, the server logs an info line and dispatches:

```rust
info("Valid data for key {key_id:X?} - trying cmd {cmd} and counter {client_counter}|{server_counter:?} with {ip}");
self.update_block_list(key_id, client_data.counter);
self.send_command(CommanderData { cmd_hash: cmd, ip });
Ok(())
```

Note the order: the blocklist is updated **before** the command is sent. The IP forwarded to the
commander is `client_data.src_ip.unwrap_or(src_ip)`: the client-declared source IP if present,
otherwise the real packet source.

## `update_block_list`

```rust
pub(super) fn update_block_list(&mut self, key_id: [u8; KEY_ID_SIZE], counter: u128)
```

Calls `blocklist.add(key_id, counter)` then `blocklist.save()`. A save failure is logged via
`error(...)` but does not abort the request: the in-memory counter is still advanced, so the replay
check is correct for the rest of the process lifetime even if persistence failed.

## `send_command`

```rust
pub(super) fn send_command(&self, data: CommanderData)
```

Wraps `write_to_socket`. On success it logs `"Successfully sent data to commander"`. On failure it
logs an `error(...)` including the socket path but **swallows the error** (returns nothing). A
missing or unreachable commander socket therefore does not crash the server loop; it is logged and
the next datagram is processed.

## `write_to_socket`

```rust
pub(super) fn write_to_socket(&self, data: CommanderData) -> anyhow::Result<()>
```

Connects to the Unix socket at `self.socket_path`, converts the `CommanderData` into its 24-byte
array (`[u8; CMDR_DATA_SIZE]` via `From`), writes all bytes with `write_all`, then `flush`es.
Failures are wrapped with context: `"Could not connect to socket {path}"`,
`"Could not write {bytes} to socket {path}"`, or `"Could not flush stream for {path}"`.

## What causes a packet to be dropped

Summarising the failure modes that prevent a command from running (each returns an `Err` that is
logged and never sent to the client):

| Cause | Where | Message fragment |
| --- | --- | --- |
| Wrong datagram length | receive loop | `Invalid read count` |
| Rate limit exceeded | `check_rate_limit` | `Rate limit exceeded` |
| Unknown key id / decrypt failure | `decrypt` | `Could not find key for id` |
| Replayed/old counter | `validate_and_send_command` | `is on blocklist` |
| Destination IP not configured | `validate_and_send_command` | `Invalid host IP` |
| Strict source IP mismatch | `validate_and_send_command` | `Invalid source IP` |
| Commander socket unreachable | `send_command` (logged only) | `Could not send data to commander` |

The last row is special: validation already passed and the blocklist was updated, so the counter is
consumed even though the command did not reach the commander.
