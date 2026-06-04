# src/client/update/

Self-update from GitHub releases. `github.rs` = release/version lookup + download,
`filesystem.rs` = verify + replace.

Flow (`Updater::update`): compare current vs latest release, skip unless newer or `force`; download
the per-arch/OS binary and its signature; **verify Ed25519** against the embedded public key
(`keys/ruroco-release-ed25519.pub.pem`, private key is a CI secret) before touching disk; move the
existing binary to `.old` and write the new one, restoring `.old` on failure; then set perms
(client `0o755`, server binaries `0o500`, chowned to the `ruroco` user).

Tests: `Updater` exposes overridable `public_key_pem` and `releases_url` fields so tests inject a
generated keypair and a local HTTP server instead of hitting GitHub.
