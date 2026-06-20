# common/protocol/

The wire protocol implementation. Four files: `constants.rs` (the fixed sizes), `client_data.rs`
(the plaintext struct and its (de)serialization), `parser.rs` (framing: prepend/strip the key_id
and call crypto), and `serialization.rs` (IP to 16 bytes and back). The conceptual layout is in
[Wire Protocol](../architecture/protocol.md); this is the file-by-file reference.

> Do not change these sizes without understanding the full impact. They are matched on both sides
> and assumed throughout the crypto and validation code.

## constants.rs

```rust
pub(crate) const PLAINTEXT_SIZE: usize  = 58; // serialized ClientData
pub(crate) const CIPHERTEXT_SIZE: usize = 86; // IV(12) + tag(16) + ciphertext(58)
pub(crate) const KEY_ID_SIZE: usize     = 8;  // cleartext key selector
pub(crate) const MSG_SIZE: usize        = KEY_ID_SIZE + CIPHERTEXT_SIZE; // = 94, the datagram
```

`mod.rs` re-exports all four for use across the crate.

## client_data.rs

Defines the plaintext payload and the only struct that crosses the encryption boundary.

```rust
#[derive(Debug, Deserialize, Serialize, PartialEq)]
pub(crate) struct ClientData {
    pub(crate) cmd_hash: u64,
    pub(crate) counter:  u128,
    pub(crate) strict:   bool,
    pub(crate) src_ip:   Option<IpAddr>,
    pub(crate) dst_ip:   IpAddr,
}
```

### Client side (with-client)

- **`create(command, strict, src_ip, dst_ip, counter) -> Result<ClientData>`**: hashes `command`
  with `blake2b_u64` into `cmd_hash` and stores the rest verbatim.
- **`serialize(&self) -> Result<[u8; 58]>`**: writes the fixed big-endian layout into a
  58-byte array:

  | Field | Offset | Encoding |
  | --- | --- | --- |
  | `version` | `[0]` | `PROTOCOL_VERSION` byte (currently `1`) |
  | `cmd_hash` | `[1:9]` | `u64` big-endian |
  | `counter` | `[9:25]` | `u128` big-endian |
  | `strict` | `[25]` | `1` or `0` |
  | `src_ip` | `[26:42]` | `serialize_ip`, or all-zeros if `None` |
  | `dst_ip` | `[42:58]` | `serialize_ip` |

### Server side (with-server)

- **`deserialize(data: [u8; 58]) -> ClientData`**: reads the same layout back. The `version` byte
  at `[0]` is checked against `PROTOCOL_VERSION` (after the GCM tag has verified). A `src_ip` field of
  all-zeros decodes to `None` (the "no claimed source IP" sentinel); any other value decodes via
  `deserialize_ip`.
- **`is_source_ip_invalid(&self, source_ip: IpAddr) -> bool`**: returns `true` only when
  `self.strict` is set **and** `self.src_ip` is `Some` **and** the stored value differs from the
  datagram's real `source_ip`. In all other cases it returns `false` (the check passes). This is
  the entire strict-mode source-IP enforcement.

### Tests

`client_data.rs` ships three test modules: size tests proving `serialize` always yields exactly
58 bytes for both extreme (`u128::MAX` counter, IPv6) and minimal (all zeros, IPv4) values, and a
cross-feature round-trip test asserting `create -> serialize -> deserialize` reproduces the
original struct including the Blake2b hash of the command name.

## parser.rs

`DataParser` handles framing: turning the encrypted blob into the 94-byte datagram and back. It
owns a `CryptoHandler` on the client.

```rust
pub(crate) struct DataParser {
    #[cfg(feature = "with-client")]
    pub(crate) crypto_handler: CryptoHandler,
}
```

### encode (with-client)

```rust
pub(crate) fn create(key_string: &str) -> Result<Self>
pub(crate) fn encode(&self, data: &[u8; 58]) -> Result<[u8; 94]>
```

`create` builds the inner `CryptoHandler` from the key string. `encode` encrypts the 58-byte
plaintext into the 86-byte blob, then prepends the handler's 8-byte `id`, producing the final
94-byte message.

### decode (with-server)

```rust
pub(crate) fn decode(data: &[u8; 94])
    -> Result<(&[u8; 8], &[u8; 86])>
```

A static method (no handler needed): splits the datagram into the `key_id` (`[0:8]`) and the
ciphertext blob (`[8:94]`), returning references into the original buffer. The server then uses the
`key_id` to pick the right `CryptoHandler` and decrypt the blob. Decode is purely structural; it
does no crypto and cannot fail on content, only on a wrong-sized buffer.

## serialization.rs

IP-to-bytes conversion. `IP_SIZE = 16`.

```rust
pub(crate) fn serialize_ip(ip: &IpAddr) -> [u8; 16]
#[cfg(feature = "with-server")]
pub(crate) fn deserialize_ip(data: [u8; 16]) -> IpAddr
```

- **`serialize_ip`**: IPv4 addresses are converted to their IPv6-mapped form
  (`to_ipv6_mapped().octets()`); IPv6 addresses are taken as-is. Either way the result is 16 bytes,
  which is why both IP fields in `ClientData` are fixed 16-byte slots.
- **`deserialize_ip`** (server only): reconstructs an `Ipv6Addr` from the 16 bytes and runs it
  through `normalize_ip`, so an IPv6-mapped IPv4 comes back out as a clean `IpAddr::V4`.

This pairing is why the protocol can carry IPv4 and IPv6 uniformly in the same fixed layout, and
why the server always compares and exposes normalized addresses.

## Gotchas

- The protocol carries a **version byte** at plaintext offset `[0]` (`PROTOCOL_VERSION`, currently
  `1`). It lives inside the authenticated plaintext and is checked only after the GCM tag verifies,
  so it cannot be tampered with on the wire. Compatibility otherwise relies on never changing the
  sizes or field order. The constants file is the contract.
- `decode` returns borrowed slices into the input datagram; the server must keep that buffer alive
  while decrypting.
- An all-zero `src_ip` is meaningful: it is the wire encoding of `None`, not of `0.0.0.0`. The
  client never claims `0.0.0.0` as a real source.
