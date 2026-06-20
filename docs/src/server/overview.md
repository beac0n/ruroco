# Server and Commander Overview

Ruroco splits the receiving side of the system into two cooperating processes for privilege
separation:

- **Server** (`run_server`): an unprivileged daemon that owns the UDP socket. It receives the
  94-byte datagram, decrypts it, enforces rate limiting, deserializes the plaintext, and runs all
  validation (replay, destination IP, strict source IP). It never executes anything itself.
- **Commander** (`run_commander`): a privileged (typically root) process that owns the Unix domain
  socket. It receives a 24-byte `CommanderData` message from the server, looks the command up by
  its Blake2b-64 hash, and runs the configured shell command.

The two processes communicate over a single Unix domain socket (`ruroco.socket`). This is the only
boundary between them. The server can write to the socket, the commander reads from it. The server
never opens a privileged operation, and the commander never touches the network.

## The never-replies invariant

The protocol is strictly one-way. The server reads UDP datagrams but never sends a UDP response.
There is no acknowledgement, no error reply, and no status returned to the client. A client that
sends a packet learns nothing about whether it was accepted, rejected, rate limited, or replayed.
All outcomes (success and every failure) are logged locally on the server and surfaced as
`anyhow::Result` errors inside the receive loop, never transmitted back over the wire.

## Key invariants

- Server and Commander are separate processes (privilege separation via the Unix socket).
- The client never knows actual commands: it only sends a Blake2b-64 hash of the command name.
  The mapping from hash to shell string lives only in the commander's config.
- The counter is a u128 **nanosecond timestamp**, not a sequential value. Gaps between accepted
  counters are normal and expected.
- All IPs are stored and compared internally as IPv6-mapped (16 bytes); IPv4 addresses round-trip
  through `to_ipv6_mapped` on the wire and are collapsed back via `normalize_ip` on receipt.
- `CommanderData` on the Unix socket is exactly 24 bytes: `cmd_hash` (`u64`, big-endian) in
  bytes `[0:8]` and the IP (16 bytes, IPv6-mapped) in bytes `[8:24]`.

## Main types

```mermaid
classDiagram
    direction TB
    class Server {
        -ConfigServer config
        -HashMap~[u8;8],CryptoHandler~ crypto_handlers
        -UdpSocket socket
        -[u8;94] client_recv_data
        -PathBuf socket_path
        -Blocklist blocklist
        -RateLimiter rate_limiter
        +create(ConfigServer, Option~String~) Server
        +run() Result
        -run_loop_iteration(...) Result
        -check_rate_limit(IpAddr) Result
        -decrypt() Result
        -validate_and_send_command(...) Result
        -send_command(CommanderData)
        -write_to_socket(CommanderData) Result
    }
    class ConfigServer {
        +Vec~IpAddr~ ips
        +PathBuf config_dir
        +String socket_user
        +String socket_group
        +u32 max_requests_per_second
        +u64 max_clock_skew_seconds
        +create_crypto_handlers() Result
        +create_blocklist() Result
        +create_server_udp_socket(Option~String~) Result
        +get_commander_unix_socket_path() PathBuf
    }
    class ConfigCommander {
        +PathBuf config_dir
        +String socket_user
        +String socket_group
    }
    class ConfigCommands {
        +HashMap~String,String~ commands
        +get_hash_to_cmd() Result
    }
    class CliServer {
        +PathBuf config
    }
    class Blocklist {
        -HashMap~[u8;8],u128~ map
        -PathBuf path
        +create(Path) Result
        +is_counter_replayed([u8;8], u128) bool
        +seed_if_absent([u8;8], u128)
        +get_counter([u8;8]) Option
        +add([u8;8], u128)
        +save() Result
    }
    class RateLimiter {
        -HashMap~IpAddr,(Instant,u32)~ map
        +new() RateLimiter
        +check(IpAddr, u32) Result
    }
    class Commander {
        +PathBuf socket_path
        +HashMap~u64,String~ cmds
        +String socket_user
        +String socket_group
        +create(ConfigCommander, ConfigCommands) Result
        +run() Result
        -run_cycle(UnixStream) Result
        -run_command(str, IpAddr)
    }
    class CommanderData {
        +u64 cmd_hash
        +IpAddr ip
    }
    class CliCommander {
        +PathBuf config
        +PathBuf commands
    }

    Server --> ConfigServer
    Server --> Blocklist
    Server --> RateLimiter
    Server ..> CommanderData : sends 24 bytes
    Commander --> CommanderData : receives 24 bytes
    Commander --> ConfigCommander
    Commander --> ConfigCommands
    CliServer ..> Server : run_server
    CliCommander ..> Commander : run_commander
```

`ConfigServer` / `CliServer` live in `server::config`; `Commander`, `ConfigCommander`,
`ConfigCommands`, and `CliCommander` live in the top-level `commander` module; the IPC type
`CommanderData` is the one shared piece, in `common::ipc`. `config.toml` is one file read by both
processes through their own views (`ConfigServer` vs `ConfigCommander`). The commander builds under
`with-commander` (no OpenSSL); `with-server` is a superset of it.

## Full valid request flow

```mermaid
sequenceDiagram
    participant C as Client
    participant S as Server (unprivileged)
    participant B as Blocklist
    participant U as Unix socket
    participant K as Commander (root)
    participant SH as sh -c

    C->>S: UDP datagram (94 bytes)
    Note over S: recv_from into client_recv_data
    S->>S: count == MSG_SIZE (94)?
    S->>S: normalize_ip(src.ip())
    S->>S: rate_limiter.check(src_ip, max)
    S->>S: DataParser::decode -> (key_id, ciphertext)
    S->>S: crypto_handlers[key_id].decrypt -> plaintext (58 bytes)
    S->>S: ClientData::deserialize(plaintext)
    S->>B: is_counter_replayed(key_id, counter)?
    B-->>S: false (not a replay)
    S->>S: config.ips contains dst_ip?
    S->>S: is_source_ip_invalid(src_ip)?
    S->>B: add(key_id, counter) + save()
    S->>U: write 24-byte CommanderData (cmd_hash + ip)
    U->>K: deliver 24 bytes
    K->>K: cmds[cmd_hash] -> shell string
    K->>SH: sh -c "<command>" with RUROCO_IP=<ip>
    Note over S,C: Server never replies to the client
```

## Validation decision tree

```mermaid
flowchart TD
    A[UDP datagram received] --> B{count == 94?}
    B -- no --> X1[Error: Invalid read count, drop]
    B -- yes --> C{rate_limiter.check OK?}
    C -- no --> X2[Error: Rate limit exceeded, drop]
    C -- yes --> D{key_id known and decrypt OK?}
    D -- no --> X3[Error: no key / decrypt fail, drop]
    D -- yes --> E[ClientData::deserialize]
    E --> F{counter replayed?<br/>stored >= counter}
    F -- yes --> X4[Error: Invalid counter on blocklist, drop]
    F -- no --> G{dst_ip in config.ips?}
    G -- no --> X5[Error: Invalid host IP, drop]
    G -- yes --> H{strict and src_ip mismatch?}
    H -- yes --> X6[Error: Invalid source IP, drop]
    H -- no --> I[update blocklist + save]
    I --> J[send 24-byte CommanderData to Unix socket]
    J --> K[Commander runs shell command]
```

All `X*` outcomes are returned as `anyhow::Error` from `run_loop_iteration`, logged via `error(...)`,
and the loop continues. Nothing is sent back to the client in any case.

## Where to read next

- [Socket and signal handling](./socket-signal.md)
- [Handler and validation](./handler.md)
- [Blocklist and rate limiter](./blocklist-ratelimiter.md)
- [Config and keys](./config-keys.md)
- [IPC contract (ipc.rs)](../common/ipc.md)
- [Commander](../commander.md)
