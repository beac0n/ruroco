# Summary

[Introduction](./introduction.md)

# Top-Level Architecture

- [Overview and Core Idea](./architecture/overview.md)
- [Top-Level Modules](./architecture/modules.md)
- [End-to-End Flow](./architecture/end-to-end-flow.md)
- [Wire Protocol](./architecture/protocol.md)
- [Cryptography](./architecture/cryptography.md)
- [Security Model](./architecture/security.md)
- [Build, Features and Deployment](./architecture/build-and-deploy.md)

# Common Layer

- [Overview](./common/overview.md)
- [crypto/](./common/crypto.md)
- [protocol/](./common/protocol.md)
- [fs.rs and logging.rs](./common/fs-logging.md)

# Client and UI

- [Client Overview](./client/overview.md)
- [config/](./client/config.md)
- [send/](./client/send.md)
- [counter, lock, gen, util](./client/counter-lock-gen-util.md)
- [update/](./client/update.md)
- [wizard/](./client/wizard.md)
- [UI Overview](./ui/overview.md)
- [app/ (RurocoApp and state)](./ui/app.md)
- [tabs/](./ui/tabs.md)
- [UI support (widgets, colors, command list)](./ui/support.md)
- [Android integration](./ui/android.md)

# Server and Commander

- [Server and Commander Overview](./server/overview.md)
- [socket.rs and signal.rs](./server/socket-signal.md)
- [handler.rs](./server/handler.md)
- [blocklist.rs and rate_limiter.rs](./server/blocklist-ratelimiter.md)
- [config.rs and keys.rs](./server/config-keys.md)
- [commander (data, exec, util)](./server/commander.md)
