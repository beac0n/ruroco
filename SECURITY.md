# Security Policy

## Reporting a Vulnerability

Please use the [Security Advisories](https://github.com/beac0n/ruroco/security/advisories/new) feature to report
vulnerabilities.

## Supported versions

ruroco follows SemVer. Security fixes land on the latest minor release; older minors do not receive backports.

| Version  | Supported          | Notes                                                              |
| -------- | ------------------ | ------------------------------------------------------------------ |
| 0.14.x   | :white_check_mark: | Current line. AES-256-GCM-SIV packets, Ed25519-signed releases.    |
| < 0.14.0 | :x:                | Pre-signing releases; updates cannot be verified. Upgrade.         |

When a fix ships, update with `ruroco-client update` (client) and `ruroco-client update --server` (server binaries);
both verify the Ed25519 signature before replacing anything on disk.

## Format versioning and stability

The UDP wire packet carries an explicit version marker, `PROTOCOL_VERSION`, as the first byte of
the AES-256-GCM-SIV plaintext. It is authenticated (so it cannot be tampered with on the wire) and
checked before any field is interpreted: a packet of an unknown version is rejected fail-closed,
never reinterpreted. The byte is added now, pre-1.0, because it cannot be introduced later without
an undetectable break, and it is what lets a future server distinguish and support multiple packet
versions during a migration. Bump `PROTOCOL_VERSION` on any incompatible plaintext/framing change.

The on-disk formats are local state, not cross-version wire contracts, and need no version field:

- `blocklist.msgpck` is msgpack of the `Blocklist` struct, so any incompatible schema change makes
  deserialization fail (surfaced as "Could not create blocklist from vec"): it already fails closed.
- The client counter file is a single raw big-endian `u128`; a fixed-width integer has no internal
  layout that can change incompatibly, and it is recoverable at any time with `ruroco-client reseed`.

## Threat model

ruroco's job is narrow: deliver one authenticated, encrypted UDP packet that names (by hash) a pre-configured action,
and run that action on the server. There is no session, no response, and no network access granted to the client. The
model below states what we defend against, what we explicitly do not, and how keys are managed over their lifetime.

### System boundaries

- **Client** (untrusted position on the internet): holds the shared key, sends 94-byte packets. Never learns the
  command strings; only sends a Blake2b-64 hash of a command name.
- **Server** (network-facing, unprivileged): receives packets, decrypts, validates, forwards a command hash to the
  commander over a local Unix socket. Never replies on the network. Holds the shared key(s).
- **Commander** (root, not network-facing): owns `commands.toml`, maps the hash to a command string, runs it. Trusts
  the local Unix socket.

The trust boundary that matters most is between server and commander: hostile-input parsing and crypto stay in the
unprivileged server; only a fixed-size message of `(command hash, IP)` crosses the socket into the root process.

### In-scope attacks (defended)

- **Replay.** Every accepted packet's counter (a u128 nanosecond timestamp) is recorded per key id in a persisted
  blocklist (`blocklist.msgpck`). A packet is accepted only if its counter is strictly greater than the last one seen
  for that key. Identical packets (retransmits, captures, adversarial replays) are rejected. See
  `src/server/blocklist.rs`.
- **Forward-dated / clock-jump packets.** A counter further in the future than `now + max_clock_skew_seconds` is
  rejected and does *not* advance the blocklist, so a single bogus far-future packet cannot lock out a key.
- **Packet forgery without the key.** Packets are sealed with AES-256-GCM-SIV (RFC 8452). Without the 256-bit key an
  attacker cannot produce a packet that authenticates, nor read the command hash / counter of a captured one.
- **Source-IP spoofing.** A command may pin the requesting IP (`$RUROCO_IP`). The server checks the packet's claimed
  source IP against the real UDP source unless `--permissive` is set. With `--permissive` the operator is expected to
  supply a verified external IP (e.g. from an IP-echo service). All IPs are normalized to IPv6-mapped (16 bytes). The
  packet also pins the destination IP, which must be in the server's configured `ips`.
- **Flooding / DoS (amplification and state exhaustion).** The server never responds, so it cannot be used as a UDP
  reflector/amplifier. A per-source-IP rate limiter (`src/server/rate_limiter.rs`) caps requests per second and
  lazily evicts stale entries so a flood of spoofed unique source IPs cannot grow server memory without bound.
- **Port scanning / fingerprinting.** No response is ever sent to any packet, valid or not, so scanning reveals nothing.
- **Supply-chain (release tampering).** Release binaries are signed with Ed25519 in CI. The client embeds the public
  key at build time and verifies the `.sig` before writing any update to disk; a missing or bad signature aborts the
  update and leaves the existing binary untouched. Verification covers `v0.14.0` and later.
- **Command enumeration from a compromised server.** Commands live in `commands.toml`, read only by the root commander
  (installed `root`-owned, mode `0600`). A compromised server process cannot read the command set and, because commands
  are referenced by hash, can only trigger commands whose hashes it observes in live traffic, never dormant ones.

### Out-of-scope / accepted risks

- **Symmetric key on the network-facing server.** The unprivileged server holds the same secret needed to *create*
  valid packets, so compromising the server process yields packet-forgery capability against that one deployment. We
  accept this rather than move to an asymmetric authenticator (see "Cryptographic design" below for the full
  rationale).
- **Nonce / message-volume limit.** A fresh random 96-bit IV is generated per packet. Because AES-256-GCM-SIV is
  nonce-misuse-resistant, a repeated IV only ever reveals whether two plaintexts were identical (which the replay
  counter already rejects), not key material. There is therefore no practical birthday-bound message ceiling per key.
- **Trust of the local Unix socket.** The commander trusts whatever arrives on its socket (`0o204`: server writes,
  commander reads). Any local process able to write that socket as the server user can request command execution. This
  is by design: privilege separation protects the root command set and the host, not against a fully compromised host.
- **Compromised client.** A stolen client key lets an attacker fire the whitelisted actions for that deployment. It
  does not grant a network position or reveal the command strings. Limit blast radius by scoping commands narrowly and
  using one key per client (see below).
- **Local clock dependency.** Replay protection is timestamp-based. A client whose clock is rolled back below the
  server's last-seen counter will have packets rejected until reseeded (`ruroco-client reseed`).
- **Traffic analysis / metadata.** Packet size is fixed and contents are encrypted, but an observer can still see that
  a 94-byte UDP packet was sent to the server, and when.

### Key lifecycle

- **Generation.** `ruroco-client gen` (or the UI) produces a base64 string of `8-byte key id || 32-byte key`, drawn
  from OpenSSL's CSPRNG. Keys are held in `Zeroizing`/`ZeroizeOnDrop` buffers and never logged (only the key id is).
- **Distribution.** Copy the generated `.key` file to the server config dir (default `/etc/ruroco/`, the server loads
  every `*.key` there) and keep the matching string on the client, ideally in a password manager or the keyring via
  `secret-tool`. Treat the key as a shared secret in transit: move it over an already-secure channel.
- **One key per client.** Each client must have its own key. Sharing a key across clients desynchronizes the per-key
  counter and causes the server to reject the lagging client (see README troubleshooting). Per-client keys also bound
  the blast radius of a single compromise and allow targeted revocation.
- **Rotation.** Generate a new key, copy it to the server config dir alongside the old one, switch the client to the
  new key, then delete the retired `.key` file from the server and the client store. The server loads all `*.key`
  files, so rotation is a drop-in/drop-out with no downtime.
- **Compromise / revocation.** To revoke a key, delete its `.key` file from the server config dir and restart the
  server; packets bearing that key id will then fail to decrypt and be dropped. Rotate any client that shared the key.
- **Counter recovery.** If a client's counter file is lost/corrupted or the clock jumped, `ruroco-client reseed`
  resets the local counter to the current nanosecond timestamp so packets are accepted again, without changing the key.

## Cryptographic design and an accepted risk

Packets are authenticated and encrypted with a shared-secret AES-256-GCM-SIV key (key id + key), not with an asymmetric
signature scheme. This is a deliberate choice:

- One AEAD gives both confidentiality and authenticity in an 86-byte payload (94-byte packet). It hides the command
  hash and replay counter from on-path observers while authenticating them.
- AES-256-GCM-SIV (RFC 8452) is nonce-misuse-resistant: a repeated 96-bit IV is not catastrophic (it only reveals
  whether two plaintexts were identical, which the replay counter already rejects), so the fresh random IV per packet
  carries no birthday-bound message ceiling in practice. A signature-only asymmetric scheme would
  authenticate but leak the plaintext, and restoring confidentiality with a sealed-box (ephemeral-key) construction
  would enlarge the packet. Speed is not the main reason (signature verification is microseconds); packet size,
  confidentiality, and key-management simplicity are.

**Accepted risk:** the unprivileged, network-facing server holds the same secret needed to *create* valid packets, so
compromising the server process yields packet-forgery capability. We accept this instead of moving to an asymmetric
authenticator, for these reasons:

- Keys are per-deployment and unique, so a stolen key forges only against that one deployment.
- The server does not hold the command set. Commands live in a separate `commands.toml` read only by the root
  commander (installed `root`-owned `0600`). A compromised server cannot enumerate the configured commands, and since
  commands are referenced by hash it can only trigger commands whose hashes it observes in live traffic, not dormant
  ones.
- In this split-process design the benefit of asymmetric crypto would be limited anyway: the root commander trusts the
  local Unix socket, so a compromised server can already drive it directly. The only real improvement would be making
  the commander verify packets itself, which we reject because it would move hostile-input parsing and crypto into the
  privileged root process.
- Replay is bounded by the per-key blocklist: a forged packet still needs a counter that has not been seen.
