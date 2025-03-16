#![allow(clippy::too_long_first_doc_paragraph)]
//! This file exposes all the modules that are needed by the client, server and commander

#[cfg(target_os = "android")]
/// contains library content for android apk
pub mod android;
/// persists the blocked list of deadlines
pub mod blocklist;
/// provides functionality to sending data to the server and for generating PEM files
pub mod client;
/// responsible for executing the commands that are defined in the config file
pub mod commander;
/// common functionality used by client, server and commander
pub mod common;
/// data structures for using CLI arguments for the client binary
pub mod config_client;
/// data structures for loading configuration files and using CLI arguments for server services
pub mod config_server;
pub mod data;
/// structs to deserialize json responses from github api
pub mod github_api_definition;
/// used to import everything that is slint related
pub mod rust_slint_bridge;
/// saves commands configured in ui
pub mod saved_command_list;
/// responsible for receiving data from the client and sending that data to the commander
pub mod server;
/// includes everything needed to run the user interface
pub mod ui;
