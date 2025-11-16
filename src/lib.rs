#![allow(clippy::too_long_first_doc_paragraph)]
//! This file exposes all the modules that are needed by the client, server and commander

/// provides functionality to sending data to the server and for generating key file
pub mod client;
/// common functionality used by client, server and commander
pub mod common;

/// responsible for receiving data from the client and sending that data to the commander
pub mod server;
/// includes everything needed to run the user interface
pub mod ui;
