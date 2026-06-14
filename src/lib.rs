#![allow(clippy::too_long_first_doc_paragraph)]
//! This file exposes all the modules that are needed by the client, server and commander

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
