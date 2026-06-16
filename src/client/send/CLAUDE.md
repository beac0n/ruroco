# src/client/send/

Builds and sends the UDP packet. `core.rs` = `Sender` + packet assembly, `network.rs` = address
resolution + socket send.

Flow (`Sender::send`): resolve `address` to IPs (filtered by `--ipv4`/`--ipv6`), then for each
destination IP in turn: increment the counter, build `ClientData::create(cmd, !permissive, src_ip,
dst_ip, counter)`, serialize to 58 bytes, encrypt to the 94-byte packet (key_id prepended), and
send one datagram. Sleeps `send_delay_ms` between IPs.

Gotchas: the counter (`<conf_dir>/counter`, seeded to `now_nanos()`) is written to disk on every
send, so the server's replay floor advances monotonically. `permissive` is inverted into the
packet's `strict` bool. IPv6 literal addresses need `[addr]:port`.
