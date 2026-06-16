# src/common/protocol/

Wire protocol. **Do not change sizes without understanding the full impact.** `parser.rs` =
encode/decode (prepends/strips the key_id, calls crypto), `serialization.rs` = IP encoding,
`client_data.rs` = the plaintext struct, `constants.rs` = sizes.

Sizes (`constants.rs`):
- `MSG_SIZE` = 94 = `KEY_ID_SIZE`(8) + `CIPHERTEXT_SIZE`(86)
- `CIPHERTEXT_SIZE`(86) = IV(12) + GCM tag(16) + `PLAINTEXT_SIZE`(58)

`ClientData` plaintext layout (58 bytes, big-endian): version `u8` [0], cmd_hash `u64` [1:9],
counter `u128` [9:25], strict `bool` [25], src_ip [26:42], dst_ip [42:58]. The version byte is
`PROTOCOL_VERSION` (currently 1); it lives inside the authenticated plaintext, so `deserialize`
rejects any unknown version after the GCM tag check. Bump `PROTOCOL_VERSION` on any incompatible
plaintext/framing change. IPs are always 16 bytes (`serialize_ip` maps IPv4 to IPv6-mapped; a
src_ip of all-zeros decodes to `None`). `is_source_ip_invalid` only rejects when `strict` is set
and a sent src_ip mismatches the real packet source.
