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

The counter is advanced **once per knock, in `send_to_destinations`**, not once per destination, so
every datagram of one send carries the same counter. That is what makes a dual-stack send execute
exactly once: a hostname with both an A and an AAAA record produces two datagrams, and if both
reach the server the first is accepted and the second hits the replay check (`counter >= floor`)
and is dropped. With a per-destination counter the second packet was strictly newer, so it
validated too and the commander ran the command twice with a different `$RUROCO_IP` - which
silently breaks any non-idempotent command (a toggle, a restart). Do not move `inc()` back into
`send_data`. The trade-off is one expected replay rejection logged per dual-stack knock; it carries
no source IP, so the shipped fail2ban filter cannot ban on it.

Gotchas: the counter (`<conf_dir>/counter`, seeded to `now_nanos()`) is written to disk on every
send, so the server's replay floor advances monotonically. `permissive` is inverted into the
packet's `strict` bool. IPv6 literal addresses need `[addr]:port`.
