# src/common/crypto/

AES-256-GCM via OpenSSL. `handler.rs` = `CryptoHandler` (key parsing/lifecycle), `handler_ops.rs`
= encrypt/decrypt, `mod.rs` = `verify_ed25519` and `blake2b_u64`.

- Key string is base64 of id(8B) + key(32B). `CryptoHandler` is `ZeroizeOnDrop` and its `Debug`
  redacts the key; `gen_key()` uses OpenSSL `rand_bytes`.
- `encrypt` generates a fresh random IV per call and outputs IV(12) || tag(16) || ciphertext(57)
  = 85 bytes; `decrypt` splits in that order and fails closed if the GCM tag check fails.
- `verify_ed25519(pubkey_pem, msg, sig)` is client-only, used by self-update to verify binaries.
