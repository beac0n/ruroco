# src/common/

Code shared by client, server, and ui. Subdirs: `crypto/`, `protocol/`, `android/`.

Cross-cutting helpers worth knowing:

- `logging.rs`: the project's own logger. `info()`/`error()` take `impl Display`, so pass owned
  values: `info(format!(...))` or `info("literal")`, never `info(&format!(...))` (that borrows a
  temporary and reads worse). No external log crate.
- `fs.rs`: `write_atomic()` (temp file + `fsync` + rename, used for counter/blocklist/commands
  list), `resolve_path()`, and ownership helpers (`nix`).
- `mod.rs`: `blake2b_u64()` (command-name hashing, shared by client and server) and
  `normalize_ip()` (collapse IPv6-mapped IPv4 back to v4).
