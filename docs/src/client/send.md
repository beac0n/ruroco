# Client Send Subsystem

The send subsystem turns a parsed `SendCommand` into one or more encrypted UDP
datagrams on the wire. It is split across three files:

- `src/client/send/mod.rs`: the module facade.
- `src/client/send/core.rs`: the `Sender` struct, construction, port
  normalization, the `send` loop, and plaintext assembly.
- `src/client/send/network.rs`: destination-IP resolution and the per-datagram
  socket send.

## `send/mod.rs`

```rust
pub mod core;
mod network;

pub use core::Sender;
```

`core` is public so the rest of the crate can reach `Sender` internals through
`super`; `network` is private and only adds `impl Sender` blocks. The single
re-export `pub use core::Sender` is what the rest of the client imports.

## `send/core.rs`

### `Sender`

```rust
#[derive(Debug)]
pub struct Sender {
    pub(super) cmd: SendCommand,
    pub(super) data_parser: DataParser,
    pub(super) counter: Counter,
}
```

- `cmd`: the parsed `SendCommand` (with `address` already port-normalized).
- `data_parser`: a `DataParser` built from `cmd.key`. It owns the `CryptoHandler`
  that holds the 32-byte AES key and the 8-byte key id, and performs encryption
  plus the key-id prepend.
- `counter`: the replay `Counter`, loaded from or seeded into `<conf_dir>/counter`.

All three fields are `pub(super)` so `network.rs` (same module) can read them.

### `Sender::create`

```rust
pub fn create(mut cmd: SendCommand) -> anyhow::Result<Self>
```

Steps:

1. Normalize the destination: `cmd.address = Self::ensure_port(cmd.address, 80)`.
2. Compute the counter path with `Self::get_counter_path()?` and log
   `Loading counter from <path> ...`.
3. Build the `DataParser` with `DataParser::create(&cmd.key)?`. This is where an
   invalid key fails early (for example `Key too short` for an 8-byte input).
4. Build the counter with `Counter::create_and_init(counter_path, now_nanos()?)?`,
   which reads the existing file or seeds it to the current nanosecond timestamp.

### `Sender::ensure_port`

```rust
fn ensure_port(address: String, default_port: u16) -> String
```

Normalizes the destination string so it always carries a port:

- If `address` starts with `[` (an IPv6 literal): keep it as-is when it already
  contains `]:` (a port is present), otherwise append `:<default_port>`. So
  `[::1]` becomes `[::1]:34020`, while `[::1]:1234` is unchanged.
- Else if `address` contains `:` (an IPv4 with port like `1.2.3.4:5678`, or a
  bare IPv6): keep it as-is.
- Else (a hostname or a bare IPv4): append `:<default_port>`, so `127.0.0.1`
  becomes `127.0.0.1:34020`.

The default port passed in `create` is `34020` (`common::DEFAULT_PORT`), matching the server's
default listen port.

### `Sender::get_counter_path`

```rust
pub fn get_counter_path() -> anyhow::Result<PathBuf>
```

Returns `resolve_path(&get_conf_dir()?).join("counter")`. It resolves the conf
dir, canonicalizes it via `resolve_path`, and appends `counter`. This is also the
path `run_client` passes to `Counter::reseed` for the `reseed` subcommand.

### `Sender::send`

```rust
pub fn send(&mut self) -> anyhow::Result<()>
```

The send loop:

1. Log `Connecting to udp://<address>, using <openssl version> ...`.
2. Resolve destinations with `self.get_destination_ips()?` (see below). This
   returns the validated, family-filtered list of `IpAddr`.
3. Log the discovered IPs.
4. Iterate the IPs with their index `i`. For every IP after the first
   (`i > 0`), if `send_delay_ms > 0`, sleep `Duration::from_millis(send_delay_ms)`
   before sending. Then call `self.send_data(*destination_ip)?`.

So with both an IPv4 and an IPv6 destination, two datagrams are sent with a delay
between them; with one destination, no delay is applied.

### `Sender::get_data_to_encrypt`

```rust
pub(super) fn get_data_to_encrypt(
    &self,
    destination_ip: IpAddr,
) -> anyhow::Result<[u8; PLAINTEXT_SIZE]>
```

Builds the 58-byte plaintext for one destination:

```rust
ClientData::create(
    &self.cmd.command,                          // hashed to cmd_hash (Blake2b-64)
    !self.cmd.permissive,                        // permissive -> strict inversion
    self.cmd.ip.clone().and_then(|d| d.parse().ok()), // optional source IP
    destination_ip,                              // this destination
    self.counter.count(),                        // current counter value
)?
.serialize()
```

Two important transforms happen here:

- **permissive -> strict inversion.** The user-facing flag is `permissive`; the
  wire field is `strict`. `strict = !permissive`. When `permissive` is `false`
  (the default), `strict` is `true` and the server enforces that the real source
  IP matches the claimed `--ip`.
- **Best-effort source IP.** `self.cmd.ip` is parsed with `.parse().ok()`; an
  unparsable string becomes `None` (no source IP), which serializes as 16 zero
  bytes.

The `ClientData` plaintext layout (from `ClientData::serialize`) is exactly
`PLAINTEXT_SIZE = 58` bytes:

| Offset | Size | Field |
| --- | --- | --- |
| 0..1 | 1 | `version` (`PROTOCOL_VERSION` byte, currently `1`) |
| 1..9 | 8 | `cmd_hash` (Blake2b-64 of the command name, big-endian) |
| 9..25 | 16 | `counter` (`u128`, big-endian) |
| 25..26 | 1 | `strict` (`0` or `1`) |
| 26..42 | 16 | `src_ip` (IPv6-mapped, all zero if `None`) |
| 42..58 | 16 | `dst_ip` (IPv6-mapped) |

## `send/network.rs`

This file adds two `impl Sender` methods plus a small context helper.

### `Sender::get_destination_ips`

```rust
pub(super) fn get_destination_ips(&self) -> anyhow::Result<Vec<IpAddr>>
```

1. Resolve `cmd.address` with `to_socket_addrs()`. On failure the error carries
   the context `Could not resolve hostname for <address>`.
2. Split the resolved `SocketAddr`s into IPv4 and IPv6 lists.
3. Let `use_ip_undef = (cmd.ipv4 == cmd.ipv6)`, i.e. the family is "undefined"
   when both flags are set or both are unset.
4. Select results by matching on `(first IPv4, first IPv6)`:

| Condition | Result |
| --- | --- |
| Both families present and `use_ip_undef` | `[ipv4, ipv6]` (both) |
| Only IPv4 present and `use_ip_undef` | `[ipv4]` |
| Only IPv6 present and `use_ip_undef` | `[ipv6]` |
| `cmd.ipv6` set and an IPv6 exists | `[ipv6]` |
| `cmd.ipv4` set and an IPv4 exists | `[ipv4]` |
| `cmd.ipv6` set but no IPv6 | error `Could not find any IPv6 address for <address>` |
| `cmd.ipv4` set but no IPv4 | error `Could not find any IPv4 address for <address>` |
| nothing resolved | error `Could not find any IPv4 or IPv6 address for <address>` |

The "undefined" rows take the first address of each available family, so a dual-
stack hostname yields two datagrams.

### `Sender::send_data`

```rust
pub(super) fn send_data(&mut self, ip: IpAddr) -> anyhow::Result<()>
```

The single-datagram path:

1. `self.counter.inc()?`: increment (overflow-checked) and persist the counter to
   disk *before* building the packet. This advances the server's replay floor
   monotonically and means every datagram, even the second one in a dual-stack
   send, carries a strictly larger counter.
2. Pick the bind address by family: `0.0.0.0:0` for IPv4, `[::]:0` for IPv6.
3. Log `Connecting to <ip>...`.
4. `self.get_data_to_encrypt(ip)?` builds the 58-byte plaintext.
5. `self.data_parser.encode(&data_to_encrypt)?` produces the 94-byte datagram.
6. Bind a `UdpSocket` to the bind address, `connect` to `cmd.address`, and `send`
   the bytes. Each of the three socket calls adds the context
   `Could not connect/send data to <address>` via `Self::socket_ctx`.
7. Log `Sent command <command> from <bind_address> to udp://<address>`.

Note that the datagram is `connect`ed to `cmd.address` (the original, possibly
hostname-or-literal string), while the family of the resolved `ip` only decides
the local bind address.

### `Sender::socket_ctx`

```rust
pub(super) fn socket_ctx<E: std::fmt::Debug>(val: E) -> String
```

Returns `format!("Could not connect/send data to {val:?}")`, the shared context
string for the three socket operations.

## Packet assembly: from 58 bytes to the 94-byte datagram

The encryption and framing happen in the `common` crate, driven by the client's
`DataParser`.

- **Encrypt** (`CryptoHandler::encrypt`): AES-256-GCM-SIV with a freshly randomized
  12-byte IV. The output `CIPHERTEXT_SIZE = 86` bytes is laid out as
  `[IV (12)] [GCM tag (16)] [ciphertext (58)]`. The ciphertext is the same length
  as the plaintext (GCM is a stream cipher), so `12 + 16 + 58 = 86`.
- **Frame / key_id prepend** (`DataParser::encode`): prepend the 8-byte key id in
  front of the 86-byte ciphertext block, giving `MSG_SIZE = 94` bytes:
  `[key_id (8)] [IV (12)] [tag (16)] [ciphertext (58)]`. The key id lets the
  server pick the right shared key before attempting decryption.

So the full datagram geometry is:

```
94 bytes total
= 8  key_id
+ 86 ciphertext block
     = 12 IV
     + 16 GCM tag
     + 58 encrypted ClientData
```

## Send sequence diagram

```mermaid
sequenceDiagram
    participant R as run_client
    participant S as Sender
    participant N as get_destination_ips
    participant C as Counter
    participant D as DataParser / CryptoHandler
    participant K as UdpSocket

    R->>S: Sender::create(send_command)
    S->>C: Counter::create_and_init(path, now_nanos)
    R->>S: send()
    S->>N: resolve cmd.address, filter by ipv4/ipv6
    N-->>S: Vec<IpAddr> (validated)
    loop for each destination IP (delay send_delay_ms after the first)
        S->>C: inc() then persist counter (u128 big-endian)
        S->>S: get_data_to_encrypt(ip)
        Note over S: ClientData::create(cmd, !permissive, src_ip, dst_ip, counter)<br/>serialize -> 58 bytes
        S->>D: encode(58-byte plaintext)
        Note over D: AES-256-GCM-SIV encrypt -> 86-byte (IV+tag+ct)<br/>prepend 8-byte key_id -> 94-byte datagram
        D-->>S: [u8; 94]
        S->>K: bind 0.0.0.0:0 or [::]:0, connect(address), send(datagram)
        Note over K: one UDP datagram, no response read
    end
    S-->>R: Ok
```
