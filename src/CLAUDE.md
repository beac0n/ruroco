# src/

Source root. Keep this and all nested CLAUDE.md files high-signal (no file dumps, no line numbers).

Layout: `bin/` entry points · `client/` CLI client · `server/` network-facing daemon ·
`commander/` privileged root executor · `common/` shared crypto/protocol/logging/ipc ·
`ui/` egui GUI. Server and commander are separate processes/binaries (privilege separation); their
only shared contract (`CommanderData` + socket path) lives in `common::ipc`. Each role owns its
config (`server::config::ConfigServer`, `commander::config::ConfigCommander`/`ConfigCommands`);
they just read the same `config.toml`/`commands.toml` files.

End-to-end flow: client hashes a command name (Blake2b-64), builds `ClientData`, encrypts it
(AES-256-GCM-SIV) into a 93-byte packet, sends one UDP datagram. The server decrypts, runs replay +
IP + rate checks, then hands a 24-byte `CommanderData` to the separate commander process over a
Unix socket; the commander looks the hash up and runs the configured shell command. Nothing is
ever sent back to the client.
