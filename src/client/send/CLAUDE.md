# src/client/send/

Builds and sends the UDP packet. `core.rs` = `Sender` + packet assembly, `network.rs` = address
resolution + socket send.

Flow (`Sender::create`): read and trim the key from `cmd.key_file` (the only source; `SendCommand`
has no `key` field, so a stray secret can't leak via `ps`, shell history, or an in-memory bypass).
Then (`Sender::send`): resolve `address` to `SocketAddr`s (filtered by
`--ipv4`/`--ipv6`, port kept from resolution), then for each destination in turn: increment the
counter, build `ClientData::create(cmd, !permissive, src_ip, dst_ip, counter)`, serialize to 58
bytes, encrypt to the 94-byte packet (key_id prepended), and send one datagram to that exact
`SocketAddr` (not the original hostname, which the OS could otherwise re-resolve differently from
what the encrypted `dst_ip` says). Sleeps `send_delay_ms` between destinations.

Gotchas: the counter (`<conf_dir>/counter`, seeded to `now_nanos()`) is written to disk on every
send, so the server's replay floor advances monotonically. `permissive` is inverted into the
packet's `strict` bool. IPv6 literal addresses need `[addr]:port`.
