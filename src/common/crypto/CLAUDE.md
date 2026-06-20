# src/common/crypto/

AES-256-GCM-SIV (RFC 8452) via OpenSSL, fetched from the default provider (needs OpenSSL >= 3.2).
`handler.rs` = `CryptoHandler` (key parsing/lifecycle), `handler_ops.rs` = encrypt/decrypt (via
`cipher::Cipher::fetch` + `cipher_ctx::CipherCtx`), `mod.rs` = `verify_ed25519` and `blake2b_u64`.

- GCM-SIV is nonce-misuse-resistant: a repeated nonce only leaks whether two plaintexts were equal
  (and equal packets are rejected as replays), never the key-recovery/forgery of plain GCM.
- Key string is base64 of id(8B) + key(32B). `CryptoHandler` is `ZeroizeOnDrop` and its `Debug`
  redacts the key; `gen_key()` uses OpenSSL `rand_bytes`.
- `encrypt` generates a fresh random IV per call and outputs IV(12) || tag(16) || ciphertext(58)
  = 86 bytes; `decrypt` splits in that order and fails closed if the tag check fails. The replay
  counter lives inside the authenticated plaintext, so the random IV stays metadata-free on the wire.
- `verify_ed25519(pubkey_pem, msg, sig)` is client-only, used by self-update to verify binaries.
