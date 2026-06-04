# src/

Source root. Keep this and all nested CLAUDE.md files high-signal (no file dumps, no line numbers).

Layout: `bin/` entry points · `client/` CLI client · `server/` daemon + commander ·
`common/` shared crypto/protocol/logging · `ui/` egui GUI.

End-to-end flow: client hashes a command name (Blake2b-64), builds `ClientData`, encrypts it
(AES-256-GCM) into a 93-byte packet, sends one UDP datagram. The server decrypts, runs replay +
IP + rate checks, then hands a 24-byte `CommanderData` to the separate commander process over a
Unix socket; the commander looks the hash up and runs the configured shell command. Nothing is
ever sent back to the client.
