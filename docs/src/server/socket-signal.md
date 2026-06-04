# Socket Binding and Signal Handling

This chapter covers two small but important pieces of the server lifecycle: how the UDP socket is
acquired (`socket.rs`) and how the process shuts down cleanly (`signal.rs`).

## `socket.rs`

### Responsibilities

Decides which UDP socket the server will listen on. It supports three sources, in priority order:
an explicit address argument, the `RUROCO_LISTEN_ADDRESS` environment variable, systemd socket
activation, and finally a hardcoded fallback bind to `[::]`.

### Default port

```rust
pub(crate) const DEFAULT_PORT: u16 = 34020;
```

Used only by the fallback bind. The value is derived from the alphabet indices of the letters in
"ruroco" (r=18, u=21, o=15, c=3) multiplied together and doubled: `18 * 21 * 15 * 3 * 2 = 34020`.

### Signature

```rust
impl ConfigServer {
    pub(crate) fn create_server_udp_socket(
        &self,
        address: Option<String>,
    ) -> anyhow::Result<UdpSocket>
}
```

### Resolution order

The function matches on the tuple
`(LISTEN_PID, LISTEN_FDS, RUROCO_LISTEN_ADDRESS, address)` and picks the first arm that applies:

1. **Explicit `address` argument** (`Some(address)`): `UdpSocket::bind(address)`. This is what the
   tests use to bind ephemeral ports such as `127.0.0.1:0`.
2. **`RUROCO_LISTEN_ADDRESS` env var** set and no argument: `UdpSocket::bind(address)`.
3. **systemd socket activation**: when `LISTEN_PID` equals the current process id (as a string) and
   `LISTEN_FDS == "1"`, the socket is adopted from raw file descriptor `3`:
   ```rust
   let fd: RawFd = 3;
   let sock = unsafe { UdpSocket::from_raw_fd(fd) };
   ```
   systemd guarantees FD 3 is the first passed socket. Ownership of the fd transfers to the returned
   `UdpSocket`; this is the only `unsafe` block in the server path, and it is justified by the two
   environment checks above. This lets ruroco start on demand from a `.socket` unit without the
   daemon ever binding a port itself.
4. **Misconfigured activation guards**:
   - `LISTEN_FDS != "1"` returns `Err("LISTEN_FDS was set to {n}, expected 1")`.
   - `LISTEN_PID` not matching the current PID returns
     `Err("LISTEN_PID ({pid}) does not match current PID")`.
5. **Fallback**: bind `[::]:34020`. Binding the unspecified IPv6 address `[::]` accepts both IPv6
   and IPv6-mapped IPv4 traffic on dual-stack hosts.

### Gotchas

- The argument always wins over the environment variable, which always wins over socket activation,
  which wins over the fallback.
- Socket activation is selected purely from environment variables; it does not validate that FD 3 is
  actually a UDP socket. The safety contract relies on systemd setting it up correctly.
- The fallback uses `[::]`, not `0.0.0.0`. If you need IPv4-only behaviour, supply an explicit
  address.

## `signal.rs`

### Responsibilities

Installs POSIX signal handlers for `SIGTERM` and `SIGINT` that flip a global atomic flag. The main
loop polls this flag once per iteration so the server can stop between datagrams without being
killed mid-processing.

### State and signatures

```rust
static SHUTDOWN_REQUESTED: AtomicBool = AtomicBool::new(false);

pub(crate) fn shutdown_requested() -> bool;
pub(crate) fn install_signal_handlers();
```

`install_signal_handlers` calls the libc `signal` function for signal numbers `15` (SIGTERM) and
`2` (SIGINT), both pointing at one handler:

```rust
extern "C" fn handle_signal(_sig: c_int) {
    SHUTDOWN_REQUESTED.store(true, Ordering::SeqCst);
}
```

`shutdown_requested()` reads the atomic with `Ordering::SeqCst`.

### How the loop uses it

`Server::run` sets a 1-second read timeout on the socket, installs the handlers, then loops:

```rust
loop {
    if shutdown_requested() {
        info("Shutdown requested, stopping server loop");
        break;
    }
    let data = self.socket.recv_from(&mut self.client_recv_data);
    // WouldBlock / TimedOut -> continue
    // otherwise -> run_loop_iteration
}
```

The 1-second read timeout is what makes clean shutdown responsive: even when no datagrams arrive,
`recv_from` returns `WouldBlock`/`TimedOut` every second, the loop continues, and the
`shutdown_requested()` check runs again. Without the timeout the process would block in `recv_from`
and could not notice the flag until the next packet arrived.

### Gotchas

- The handler does the absolute minimum allowed in async-signal context: a single atomic store.
- The flag is process-global, so all tests reset it explicitly before asserting.
- Shutdown is cooperative: a datagram already being processed in `run_loop_iteration` finishes
  first; the flag is only checked at the top of the next iteration.
