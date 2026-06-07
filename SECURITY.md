# Security Policy

## Reporting a Vulnerability

Please use the [Security Advisories](https://github.com/beac0n/ruroco/security/advisories/new) feature to report
vulnerabilities.

## Cryptographic design and an accepted risk

Packets are authenticated and encrypted with a shared-secret AES-256-GCM-SIV key (key id + key), not with an asymmetric
signature scheme. This is a deliberate choice:

- One AEAD gives both confidentiality and authenticity in an 85-byte payload (93-byte packet). It hides the command
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

A fuller threat model (full in-scope attack list, key lifecycle, supported-versions table) is planned.
