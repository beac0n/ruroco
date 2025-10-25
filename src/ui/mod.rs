/// contains library content for android apk
#[cfg(target_os = "android")]
pub mod android;
#[cfg(target_os = "android")]
pub mod android_util;

/// used to import everything that is slint related
pub mod rust_slint_bridge;

/// saves commands configured in ui
pub mod saved_command_list;

use crate::ui::rust_slint_bridge::RustSlintBridge;

use std::error::Error;

pub fn run_ui() -> Result<(), Box<dyn Error>> {
    let rust_slint_bridge = RustSlintBridge::create()?;

    rust_slint_bridge.add_on_reset_commands_config();
    rust_slint_bridge.add_on_set_commands_config();
    rust_slint_bridge.add_on_update_application();
    rust_slint_bridge.add_on_add_command();
    rust_slint_bridge.add_on_del_command();
    rust_slint_bridge.add_on_exec_command();
    rust_slint_bridge.add_on_generate_key();
    rust_slint_bridge.run()?;

    Ok(())
}
