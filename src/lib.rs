#![allow(clippy::too_long_first_doc_paragraph)]
// Unsafe is limited to two audited FFI/syscall spots (systemd socket activation, signal handler
// registration) and the Android JNI bridge, each carrying its own `#[allow(unsafe_code)]` with a
// SAFETY justification. Everything else must stay safe Rust.
#![deny(unsafe_code)]
// No panics in production code (see CLAUDE.md); `clippy.toml`'s allow-*-in-tests options exempt
// #[cfg(test)] code from unwrap_used/expect_used.
#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unreachable
)]

/// provides functionality to sending data to the server and for generating key file
#[cfg(feature = "with-client")]
pub mod client;
/// the privileged executor (root): owns the Unix socket and runs the configured commands
#[cfg(feature = "with-commander")]
pub mod commander;
/// common functionality used by client, server and commander
pub mod common;

/// public entry points for the libFuzzer targets in `fuzz/`; only built with the `fuzzing` feature
#[cfg(feature = "fuzzing")]
pub mod fuzz_api;
/// responsible for receiving data from the client and sending that data to the commander
#[cfg(feature = "with-server")]
pub mod server;
/// includes everything needed to run the user interface
#[cfg(feature = "with-gui")]
pub mod ui;
