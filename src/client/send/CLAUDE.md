# src/client/send/

Builds and sends the UDP packet. `core.rs` = `Sender` + packet assembly, `network.rs` = address
resolution + socket send.

Flow (`Sender::create`): read the key from `cmd.key_file` into a `Zeroizing<String>` (the only
source; `SendCommand` has no `key` field, so a stray secret can't leak via `ps`, shell history, or
an in-memory bypass), matching the server side (`server::keys`) so the key string is wiped on drop
rather than left in freed memory.
Then (`Sender::send`): resolve `address` to `SocketAddr`s (filtered by
`--ipv4`/`--ipv6`, port kept from resolution), then hand them to `send_to_destinations`, which
tries every destination in turn (incrementing the counter, building
`ClientData::create(cmd, !permissive, src_ip, dst_ip, counter)`, serializing to 58 bytes,
encrypting to the 94-byte packet, and sending one datagram to that exact `SocketAddr` - not the
original hostname, which the OS could otherwise re-resolve differently from what the encrypted
`dst_ip` says) rather than stopping at the first failure: a hostname resolving to both an IPv4 and
IPv6 address should still reach the server over whichever family actually works. Only fails
(aggregating every per-destination error) if none of them worked. Sleeps `send_delay_ms` between
destinations.

Gotchas: the counter (`<conf_dir>/counter`, seeded to `now_nanos()`) is written to disk on every
send, so the server's replay floor advances monotonically. `permissive` is inverted into the
packet's `strict` bool. IPv6 literal addresses need `[addr]:port`.
