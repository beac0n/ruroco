# Introduction

**Ruroco** (Run Remote Command) lets you execute a pre-configured command on a remote
server by sending a single encrypted UDP packet. The server never answers, so from the
outside the relevant port looks closed: there is nothing to port-scan, nothing to
fingerprint, and nothing to brute-force.

Ruroco **triggers a pre-configured action** on the server (open a firewall rule, restart a
service, run a script). It is not a tunnel or a VPN: there is no session and no traffic is
carried, so it grants a *capability to act* rather than *network access*. See
[Overview and Core Idea](./architecture/overview.md#ruroco-is-not-a-vpn) for how it compares to
a VPN like WireGuard.

A common use is **Single Packet Authorization (SPA)**: keep a sensitive port firewalled shut at
all times, and use ruroco to briefly open it only for the IP that asked, only when you ask.

## What makes ruroco different

- **One-way and silent.** The client sends one 93-byte UDP datagram. The server never
  sends a response of any kind. An attacker probing the port learns nothing.
- **The client cannot choose arbitrary commands.** Commands are defined on the server. The
  client only sends a Blake2b-64 hash of a command *name*. It literally does not transmit
  the command string, so a captured packet never reveals what runs.
- **Privilege separation by design.** The internet-facing process (`server`) runs
  unprivileged and can only receive, decrypt, validate, and forward. A second process
  (`commander`) runs with the rights needed to execute commands and is reachable only over
  a local Unix socket, never from the network.
- **Replay-protected.** Every packet carries a strictly increasing counter (a nanosecond
  timestamp). The server records the highest counter seen per key and rejects anything at
  or below it.

## The four binaries

| Binary | Runs on | Role | Build feature |
| --- | --- | --- | --- |
| `ruroco-client` | your machine | builds, encrypts and sends the UDP packet | `with-client` |
| `ruroco-client-ui` | your machine / Android | a GUI over the client (egui) | `with-gui` |
| `ruroco-server` | remote host (exposed) | receives, decrypts, validates, forwards | `with-server` |
| `ruroco-commander` | remote host (not exposed) | looks up and runs the command | `with-commander` |

## How to read this documentation

This book is organized **top-down**, like a tree.

1. **Top-Level Architecture** sits at the root: the core idea, how the four big modules
   interact, the end-to-end flow, the wire protocol, the cryptography, the security model,
   and how the project is built and deployed.
2. **Common Layer**, **Client and UI**, **Server**, and **Commander** are the branches: each
   documents how a subsystem works as a whole, then drills into its files.
3. The **leaves** are the individual `.rs` files. Every source file is documented with its
   real types, signatures, responsibilities, and gotchas.

If you want the big picture, read the Top-Level Architecture section straight through. If you
are working on a specific file, jump to the branch that contains it and scroll to its leaf
section.

> The diagrams in this book are rendered with [mermaid](https://mermaid.js.org/) via the
> `mdbook-mermaid` preprocessor. Build the book with `mdbook build` from the `docs/`
> directory and open `docs/book/index.html`.
