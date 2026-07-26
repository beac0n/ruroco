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
and verified does `swap_all` run `save_bin` for each: copy the existing binary to `.old` (rollback
snapshot), then atomically replace the target via `write_atomic_with_mode` (temp file + rename,
exec bits set before the swap), so the target always holds a complete binary (client `0o755`,
server binaries `0o500`, chowned to the `ruroco` user). Splitting verify from swap this way means a
missing or invalid asset for one target can never leave another target already swapped to the new
version while it stays on the old one. If `save_bin` fails partway through `swap_all` (e.g. the
chown to the `ruroco` user fails after the write succeeded), every target applied so far -
including the failing one itself, since its write may have already landed - is rolled back via
`rollback_bin`: restore from `.old` if it existed before, otherwise remove the partially-applied
file, so disk state never ends up as a mix of old and new binaries. `rollback_bin` re-applies the
target's intended ownership after the restore, because `fs::copy` gave the `.old` the *copying*
process's owner (root) rather than the original's: a plain rename back would leave `ruroco-server`
root-owned `0o500` and unexecutable by `User=ruroco`, i.e. the rollback itself breaks the service
it was meant to save. Once every target has swapped successfully, `swap_all` drops the `.old`
files (`remove_backup`, never fatal). Leaving them behind is not just clutter: a later update whose
target had no backup of its own would find the stale one and "roll back" to a binary from some
earlier version. On a successful `--server`
update, `update()` logs a reminder to restart `ruroco-server`/`ruroco-commander`; nothing restarts
them automatically.

Every HTTP call goes through `http_agent()`, never a bare `ureq::get` or a default agent. ureq 2.x
defaults `timeout_connect` to 30s but leaves read/write **unbounded**, so a server that accepts the
connection and then sends nothing hangs `update` forever. The agent sets connect 10s and read/write
30s. Those are per socket operation, not for the whole transfer, so a slow but progressing download
of a multi-MB binary still completes while a stalled one gives up. `build_http_agent` takes the
durations so tests can use a short one; `http_agent()` is the production wrapper.

`github.rs`'s release lookup requests `per_page=100` (GitHub defaults to 30) and reports how many
releases it searched when a `--version` isn't found. Downloads are capped at `MAX_DOWNLOAD_BYTES`
(100 MB) in `filesystem.rs` to bound memory against a misbehaving server.

Tests: `Updater` exposes overridable `public_key_pem` and `releases_url` fields so tests inject a
generated keypair and a local HTTP server instead of hitting GitHub. Network-dependent tests are
gated behind `#[test_with::env(TEST_ONLINE)]`. The test module lives in the sibling `mod_tests.rs`
(loaded via `#[path]`), not inline in `mod.rs`.
