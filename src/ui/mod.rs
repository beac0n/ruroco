/// contains library content for android apk
#[cfg(target_os = "android")]
mod android;
#[cfg(target_os = "android")]
mod android_update;
#[cfg(target_os = "android")]
mod android_util;

/// used to import everything that is slint related
mod rust_slint_bridge;

mod colors;
mod command_data;
mod rust_slint_bridge_ctx;
/// saves commands configured in ui
mod saved_command_list;

use crate::ui::rust_slint_bridge::RustSlintBridge;

use crate::client::config;
use crate::client::lock::ClientLock;
use std::error::Error;

pub fn run_ui() -> Result<(), Box<dyn Error>> {
    let conf_dir = config::get_conf_dir()?;
    let _lock = ClientLock::acquire(conf_dir.join("client.lock"))?;
    RustSlintBridge::create(&conf_dir)?.run()?;
    Ok(())
}
