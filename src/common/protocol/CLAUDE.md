# src/common/protocol/

Wire protocol. **Do not change sizes without understanding the full impact.** `parser.rs` =
encode/decode (prepends/strips the key_id, calls crypto), `serialization.rs` = IP encoding,
`client_data.rs` = the plaintext struct, `constants.rs` = sizes.

Sizes (`constants.rs`):
- `MSG_SIZE` = 93 = `KEY_ID_SIZE`(8) + `CIPHERTEXT_SIZE`(85)
- `CIPHERTEXT_SIZE`(85) = IV(12) + GCM tag(16) + `PLAINTEXT_SIZE`(57)

`ClientData` plaintext layout (57 bytes, big-endian): cmd_hash `u64` [0:8], counter `u128` [8:24],
strict `bool` [24], src_ip [25:41], dst_ip [41:57]. IPs are always 16 bytes (`serialize_ip` maps
IPv4 to IPv6-mapped; a src_ip of all-zeros decodes to `None`). `is_source_ip_invalid` only rejects
when `strict` is set and a sent src_ip mismatches the real packet source.
