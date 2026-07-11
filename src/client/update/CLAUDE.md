# src/client/update/

Self-update from GitHub releases. `github.rs` = release/version lookup + download,
`filesystem.rs` = verify + replace.

Flow (`Updater::update`): compare current vs latest release, skip unless newer or `force`. Unless
an explicit `--version` was given, refuses to install a tag older than the running version
(`is_downgrade`, a pure semver-triple comparison) - an explicit `--version` always does exactly
what was asked, including a deliberate rollback. For every target (list of (asset prefix, target
name, mode, chown user) driven by `binary_targets`, looping over commander+server or
client+client-ui): download the binary and its signature and **verify Ed25519** against the
embedded public key (`keys/ruroco-release-ed25519.pub.pem`, private key is a CI secret) -
`download_and_verify_bin` does this without touching disk. Only once every target has downloaded
and verified does the second loop run `save_bin` for each: copy the existing binary to `.old`
(rollback snapshot), then atomically replace the target via `write_atomic_with_mode` (temp file +
rename, exec bits set before the swap), so the target always holds a complete binary (client
`0o755`, server binaries `0o500`, chowned to the `ruroco` user). Splitting verify from swap this
way means a missing or invalid asset for one target can never leave another target already
swapped to the new version while it stays on the old one.

`github.rs`'s release lookup requests `per_page=100` (GitHub defaults to 30) and reports how many
releases it searched when a `--version` isn't found. Downloads are capped at `MAX_DOWNLOAD_BYTES`
(100 MB) in `filesystem.rs` to bound memory against a misbehaving server.

Tests: `Updater` exposes overridable `public_key_pem` and `releases_url` fields so tests inject a
generated keypair and a local HTTP server instead of hitting GitHub. Network-dependent tests are
gated behind `#[test_with::env(TEST_ONLINE)]`. The test module lives in the sibling `mod_tests.rs`
(loaded via `#[path]`), not inline in `mod.rs`.
