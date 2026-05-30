# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.14.2] - 2026-05-30

### Added

- Server: improved logging in handler for better observability

### Fixed

- Tests: re-introduce update tests that were previously removed

## [0.14.1] - 2026-05-30

### Fixed

- Tests: fix update test suite

## [0.14.0] - 2026-05-30

### Added

- Security: release binary signing with Ed25519 keys
- Client: verify Ed25519 signature of downloaded release binaries before installing
- Build: add signing step to release workflow; include public key in repository

### Changed

- Dependencies: cargo update

## [0.13.1] - 2026-05-28

### Added

- Client: `reseed` subcommand resets the replay-protection counter to the current nanosecond timestamp
- UI: **Reseed Counter** button on the dashboard (below the key section)
- README: `### reseed` documentation and troubleshooting section for shared-key counter desync

## [0.13.0] - 2026-05-26

### Added

- Android: persistent AES key storage using shared preferences
- Client: add `-d` shortcut flag

### Changed

- UI: migrate from Slint to egui (major rewrite of GUI layer)
- UI: extract widget helpers (equal-button row, clipboard) into widgets module
- UI: read commands directly from commands list; remove cached_commands
- UI: stop round-trpping CommandData through CLI strings
- Server: update blocklist before executing command for improved safety
- Build: gate server-only features behind `with-server` feature flag
- Build: coverage now produces branch coverage and HTML output

### Fixed

- Security: blocklist is now updated before command execution, preventing potential replay in edge cases
- UI: fix Android interaction issues and safe-area rendering
- UI: set font size globally

### Removed

- UI: remove dead `CommandData::color` field
- Slint UI files and Slint/Rust bridge code

## [0.12.4] - 2026-05-15

### Added

- Server: rate limiting

### Changed

- Crypto: simplified AES key generation by removing PBKDF2 and salt handling
- Dependencies: replace `reqwest` with `ureq`, use `nix` crate instead of shelling out to `id`
- Server: blocklist now uses binary atomic writes; removed `blocklist_serialization` module
- Protocol: consolidate client/server data and parser modules
- Common: add `write_atomic` / `write_atomic_text` helpers for safe atomic file writes
- Client: extract `Counter` struct into `Sender`; replace sequential counter with nanosecond timestamp

### Fixed

- Security: sanitize `$RUROCO_IP` env var passed to shell commands
- Security: normalize IPv6-mapped IPv4 addresses during deserialization for consistent comparison
- Client: add sleep delay between packets sent to multiple IPs to avoid race conditions
- Crypto: prevent counter overflow at boundary conditions
- Server: ensure AES key is not kept in memory when not needed

### Removed

- Dead code and unused modules across protocol and crypto layers

## [0.12.3] - 2026-04-06

### Fixed

- Android: fix opening browser window
- Dependencies: rollback reqwest (0.13 not compatible with Android)

## [0.12.2] - 2026-06-05

### Fixed

- Android: fix safe-area-insets


## [0.12.1] - 2026-04-05

### Fixed

- Android: fix android_util.rs
- Docs: fix typos

## [0.12.0] - 2026-03-31

### Changed

- Build: update build make goal
- Build: add cargo features for building components separately
- Dependencies: bump (clap, reqwest, openssl, and others)

### Fixed

- Refactor: consolidate cfg guards at module boundaries and eliminate dead code warnings in subset builds
- Build: fix release_linux target in Makefile and revert binary filenames to match upstream
- Client: send command to all IP addresses that could be found, not just IPv6

### Added

- Tests: add more unit tests
- Docs: update README.md with secret-tool usage for key storage

## [0.11.0] - 2025-12-28

**BREAKING: 0.11.0 is a breaking release. Existing configs are not compatible and must be updated.**

### Added

- Crypto: AES-based encryption path and key generation option
- Client: ClientLock for managing concurrent access
- Server: graceful shutdown via signal handling
- Common: chrono-based date/time utilities
- Android: update functionality in command handling
- Tests: tempfile-backed fixtures for integration data

### Changed

- Crypto: command payload format and size limits for AES
- Client/Server: command handling refactors and clearer error messages
- UI: dashboard and update button behavior tweaks
- Build/Test: dependency bumps (clap, reqwest, toml, openssl, test-with)

### Fixed

- Crypto: key_id persistence and ciphertext validation
- Client/Server: improved IP handling and data parsing robustness
- Build/Test: formatting, lint cleanup, and minor test fixes

### Removed

- Build: upx usage
- Crypto: RSA-only flow and sntpc/ntp support paths

## [0.10.8] - 2025-10-05

### Fixed

- Build: upx usage
- Build: nix usage

## [0.10.7] - 2025-10-05

### Fixed

- Formatting
- Build: nix usage

## [0.10.6] - 2025-10-04

### Updated

- Dependencies: bump

## [0.10.5] - 2025-09-19

### Updated

- Dependencies: bump

### Fixed

- Formatting

## [0.10.4] - 2025-07-06

### Updated

- Dependencies: bump

### Fixed

- Build: upx usage

## [0.10.3] - 2025-06-11

### Updated

- Dependencies: bump

## [0.10.2] - 2025-06-11

### Changed

Ui: Make commands config box editable

## [0.10.1] - 2025-05-19

### Changed

- Ui: make commands config box read-only and reduce font size

## [0.10.0] - 2025-05-18

### Added

Ui:

- Add button to paste commands via text field
- Add reset commands in text field

### Changed

Android: changed package name to `org.beac0n.ruroco`

## [0.9.6] - 2025-05-18

### Fixed

- Android: package name

## [0.9.5] - 2025-05-18

### Fixed

- CI: workflow stability

## [0.9.4] - 2025-05-18

### Changed

- Build: run nix for builds except end-to-end tests and release; CI fixes

## [0.9.3] - 2025-05-18

### Changed

- Build: target Ubuntu 22.04 for broader glibc compatibility

## [0.9.2] - 2025-05-18

### Changed

- Build: restrict Android builds to nix to avoid glibc mismatches

## [0.9.1] - 2025-05-18

### Fixed

- Lints

## [0.9.0] - 2025-05-18

### Updated

Ui: Add button to copy commands from text field

## [0.8.4] - 2025-05-17

### Updated

Misc: Diverse build setup and documentation changes

## [0.8.3] - 2025-05-03

### Changed

- Build: add nix commands and use nix for Android packaging
- Docs: update README; refactors

## [0.8.2] - 2025-04-20

### Fixed

- Android: icon adjustments
- Logging updates

## [0.8.1] - 2025-04-20

### Fixed

- Lints

## [0.8.0] - 2025-04-20

### Added

Server: Wizard command to automatically install server setup

## [0.7.7] - 2025-04-18

### Fixed

- Client: Update logic
- Android: Icon

## [0.7.6] - 2025-04-14

### Fixed

- Android: icon tweaks; import cleanup

## [0.7.5] - 2025-04-14

### Changed

- Networking: prefer IPv6 when both address families are available

## [0.7.4] - 2025-04-14

### Changed

- Update flow: reduce sleep duration

## [0.7.3] - 2025-04-13

### Fixed

- Update logic

## [0.7.2] - 2025-04-13

### Fixed

- Icon sizing

## [0.7.1] - 2025-04-13

### Changed

- Networking: send over IPv4 and IPv6 when flags are omitted; fix binary path

## [0.7.0] - 2025-04-13

### Updated

- Android: icon refresh

## [0.6.9] - 2025-04-13

### Fixed

- Release publishing workflow

## [0.6.8] - 2025-04-13

### Fixed

- Release action reliability

## [0.6.7] - 2025-04-13

### Fixed

- CI: disable update tests

## [0.6.6] - 2025-04-13

### Changed

- Coverage: re-enable reporting; skip update tests in CI

## [0.6.5] - 2025-04-13

### Added

- Tests: run nextest in CI

### Updated

- CI: rust workflow tweaks

## [0.6.4] - 2025-04-13

### Added

- UI: add update button

## [0.6.3] - 2025-04-12

### Fixed

- build

## [0.6.2] - 2025-04-12

### Added

- Client: Self update functionality
- Android: Update icon

## [0.6.1] - 2024-11-24

### Added

- Android: Add icon and label

## [0.6.0] - 2024-11-02

## Changed

Refactored User Interface

## [0.5.13] - 2024-11-01

### Fixed

- Release: pipeline stability

## [0.5.12] - 2024-11-01

### Fixed

- Release: pipeline stability

## [0.5.11] - 2024-11-01

### Fixed

- Release: pipeline stability

## [0.5.10] - 2024-11-01

### Fixed

- Release: pipeline stability

## [0.5.9] - 2024-11-01

### Fixed

- Release: pipeline stability

## [0.5.8] - 2024-11-01

### Fixed

- Release: pipeline stability

## [0.5.7] - 2024-11-01

### Fixed

- Release: pipeline stability

## [0.5.6] - 2024-11-01

### Fixed

- Release: pipeline stability

## [0.5.5] - 2024-11-01

### Fixed

- Release: pipeline stability

## [0.5.4] - 2024-11-01

### Fixed

- Release: pipeline stability

## [0.5.3] - 2024-11-01

### Fixed

- Release: pipeline stability

## [0.5.2] - 2024-11-01

### Fixed

- Android: release packaging

## [0.5.1] - 2024-11-01

### Changed

- Build: Makefile and release flow fixes
- Docs: README updates

## [0.5.0] - 2024-11-01

## Added

- Release build of Android `.apk`
- Release build of client UI binary
- Save and load commands added in UI to disk
- Auto Generate PEM files on android

## Changed

- Fixed Android Internet Permissions

## [0.4.0] - 2024-10-05

### Added

- Add time lookup via ntp for client and server
- Add IPv6 support
- Add support for multiple PEM files on server

### Changed

- Replace --strict flag with --permissive flag

## [0.3.0] - 2024-09-25

### Added

- Add --strict and --ip flags to implement overriding the IP address that is being used for the commander
- Add `ip` field to config, to make sure that the UDP packet that was sent is actually designated for the server

### Changed

- Update invalid error log and fix ip checking and fix tests

## [0.2.6] - 2024-08-24

### Added

- Add features = ["vendored"] to openssl dependency

## [0.2.5] - 2024-08-22

### Removed

- Remove all dependencies related to logging and replace it with simple println!

## [0.2.4] - 2024-08-15

### Added

- Add more tests and refactor

### Fixed

- Fix get_id_by_name_and_flag to not run if name is empty

## [0.2.3] - 2024-08-11

### Changed

- Update CI configs

### Fixed

- Fix users dependency security advisory

## [0.2.2] - 2024-08-04

### Added

- Add Cargo.lock to git, since this a binary and not a library
- Add Cargo.lock to files that are committed with a new version when version.sh is executed

## [0.2.1] - 2024-08-04

### Added

- Add auto update CHANGELOG.md to version.sh when creating a new version

### Fixed

- Fix version.sh not writing correct version into Cargo.toml

## [0.2.0] - 2024-08-04

### Added

- implement passing IP address as env var (`RUROCO_IP`) to commands executed by commander

### Changed

- code refactoring
- add auto formatting and linting
- add auto create releases
- increase test speed
- fix coverage warnings
- add/update docs
- add code coverage

## [0.1.2] - 2024-06-16

### Fixed

- Fix server crashing after first UDP packet received

## [0.1.1] - 2024-06-16

### Fixed

- Fix client command binding to 127.0.0.1 instead of binding to 0.0.0.0 when sending UDP packet to host

## [0.1.0] - 2024-06-09

### Added

- Initial Release

[0.14.2]: https://github.com/beac0n/ruroco/compare/v0.14.1..v0.14.2

[0.14.1]: https://github.com/beac0n/ruroco/compare/v0.14.0..v0.14.1

[0.14.0]: https://github.com/beac0n/ruroco/compare/v0.13.1..v0.14.0

[0.13.1]: https://github.com/beac0n/ruroco/compare/v0.13.0..v0.13.1

[0.13.0]: https://github.com/beac0n/ruroco/compare/v0.12.4..v0.13.0

[0.12.4]: https://github.com/beac0n/ruroco/compare/v0.12.3..v0.12.4

[0.12.3]: https://github.com/beac0n/ruroco/compare/v0.12.2..v0.12.3

[0.12.2]: https://github.com/beac0n/ruroco/compare/v0.12.1..v0.12.2

[0.12.1]: https://github.com/beac0n/ruroco/compare/v0.12.0..v0.12.1

[0.12.0]: https://github.com/beac0n/ruroco/compare/v0.11.0..v0.12.0

[0.11.0]: https://github.com/beac0n/ruroco/compare/v0.10.8..v0.11.0

[0.10.8]: https://github.com/beac0n/ruroco/compare/v0.10.7..v0.10.8

[0.10.7]: https://github.com/beac0n/ruroco/compare/v0.10.6..v0.10.7

[0.10.6]: https://github.com/beac0n/ruroco/compare/v0.10.5..v0.10.6

[0.10.5]: https://github.com/beac0n/ruroco/compare/v0.10.4..v0.10.5

[0.10.4]: https://github.com/beac0n/ruroco/compare/v0.10.3..v0.10.4

[0.10.3]: https://github.com/beac0n/ruroco/compare/v0.10.2..v0.10.3

[0.10.2]: https://github.com/beac0n/ruroco/compare/v0.10.1..v0.10.2

[0.10.1]: https://github.com/beac0n/ruroco/compare/v0.10.0..v0.10.1

[0.10.0]: https://github.com/beac0n/ruroco/compare/v0.9.6..v0.10.0

[0.9.6]: https://github.com/beac0n/ruroco/compare/v0.9.5..v0.9.6

[0.9.5]: https://github.com/beac0n/ruroco/compare/v0.9.4..v0.9.5

[0.9.4]: https://github.com/beac0n/ruroco/compare/v0.9.3..v0.9.4

[0.9.3]: https://github.com/beac0n/ruroco/compare/v0.9.2..v0.9.3

[0.9.2]: https://github.com/beac0n/ruroco/compare/v0.9.1..v0.9.2

[0.9.1]: https://github.com/beac0n/ruroco/compare/v0.9.0..v0.9.1

[0.9.0]: https://github.com/beac0n/ruroco/compare/v0.8.4..v0.9.0

[0.8.4]: https://github.com/beac0n/ruroco/compare/v0.8.3..v0.8.4

[0.8.3]: https://github.com/beac0n/ruroco/compare/v0.8.2..v0.8.3

[0.8.2]: https://github.com/beac0n/ruroco/compare/v0.8.1..v0.8.2

[0.8.1]: https://github.com/beac0n/ruroco/compare/v0.8.0..v0.8.1

[0.8.0]: https://github.com/beac0n/ruroco/compare/v0.7.7..v0.8.0

[0.7.7]: https://github.com/beac0n/ruroco/compare/v0.7.6..v0.7.7

[0.7.6]: https://github.com/beac0n/ruroco/compare/v0.7.5..v0.7.6

[0.7.5]: https://github.com/beac0n/ruroco/compare/v0.7.4..v0.7.5

[0.7.4]: https://github.com/beac0n/ruroco/compare/v0.7.3..v0.7.4

[0.7.3]: https://github.com/beac0n/ruroco/compare/v0.7.2..v0.7.3

[0.7.2]: https://github.com/beac0n/ruroco/compare/v0.7.1..v0.7.2

[0.7.1]: https://github.com/beac0n/ruroco/compare/v0.7.0..v0.7.1

[0.7.0]: https://github.com/beac0n/ruroco/compare/v0.6.9..v0.7.0

[0.6.9]: https://github.com/beac0n/ruroco/compare/v0.6.8..v0.6.9

[0.6.8]: https://github.com/beac0n/ruroco/compare/v0.6.7..v0.6.8

[0.6.7]: https://github.com/beac0n/ruroco/compare/v0.6.6..v0.6.7

[0.6.6]: https://github.com/beac0n/ruroco/compare/v0.6.5..v0.6.6

[0.6.5]: https://github.com/beac0n/ruroco/compare/v0.6.4..v0.6.5

[0.6.4]: https://github.com/beac0n/ruroco/compare/v0.6.3..v0.6.4

[0.6.3]: https://github.com/beac0n/ruroco/compare/v0.6.2..v0.6.3

[0.6.2]: https://github.com/beac0n/ruroco/compare/v0.6.1..v0.6.2

[0.6.1]: https://github.com/beac0n/ruroco/compare/v0.6.0..v0.6.1

[0.6.0]: https://github.com/beac0n/ruroco/compare/v0.5.13..v0.6.0

[0.5.13]: https://github.com/beac0n/ruroco/compare/v0.5.12..v0.5.13

[0.5.12]: https://github.com/beac0n/ruroco/compare/v0.5.11..v0.5.12

[0.5.11]: https://github.com/beac0n/ruroco/compare/v0.5.10..v0.5.11

[0.5.10]: https://github.com/beac0n/ruroco/compare/v0.5.9..v0.5.10

[0.5.9]: https://github.com/beac0n/ruroco/compare/v0.5.8..v0.5.9

[0.5.8]: https://github.com/beac0n/ruroco/compare/v0.5.7..v0.5.8

[0.5.7]: https://github.com/beac0n/ruroco/compare/v0.5.6..v0.5.7

[0.5.6]: https://github.com/beac0n/ruroco/compare/v0.5.5..v0.5.6

[0.5.5]: https://github.com/beac0n/ruroco/compare/v0.5.4..v0.5.5

[0.5.4]: https://github.com/beac0n/ruroco/compare/v0.5.3..v0.5.4

[0.5.3]: https://github.com/beac0n/ruroco/compare/v0.5.2..v0.5.3

[0.5.2]: https://github.com/beac0n/ruroco/compare/v0.5.1..v0.5.2

[0.5.1]: https://github.com/beac0n/ruroco/compare/v0.5.0..v0.5.1

[0.5.0]: https://github.com/beac0n/ruroco/compare/v0.4.0..v0.5.0

[0.4.0]: https://github.com/beac0n/ruroco/compare/v0.3.0..v0.4.0

[0.3.0]: https://github.com/beac0n/ruroco/compare/v0.2.6..v0.3.0

[0.2.6]: https://github.com/beac0n/ruroco/compare/v0.2.5..v0.2.6

[0.2.5]: https://github.com/beac0n/ruroco/compare/v0.2.4..v0.2.5

[0.2.4]: https://github.com/beac0n/ruroco/compare/v0.2.3..v0.2.4

[0.2.3]: https://github.com/beac0n/ruroco/compare/v0.2.2..v0.2.3

[0.2.2]: https://github.com/beac0n/ruroco/compare/v0.2.1..v0.2.2

[0.2.1]: https://github.com/beac0n/ruroco/compare/v0.2.0..v0.2.1

[0.2.0]: https://github.com/beac0n/ruroco/compare/v0.1.2..v0.2.0

[0.1.2]: https://github.com/beac0n/ruroco/compare/v0.1.1..v0.1.2

[0.1.1]: https://github.com/beac0n/ruroco/compare/v0.1.0..v0.1.1

[0.1.0]: https://github.com/beac0n/ruroco/compare/430f13210909893d2c80d2f94244e4c737a960b2..v0.1.0
