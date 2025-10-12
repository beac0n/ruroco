/// contains library content for android apk
#[cfg(target_os = "android")]
pub mod android;
#[cfg(target_os = "android")]
pub mod android_util;

/// used to import everything that is slint related
pub mod rust_slint_bridge;

/// saves commands configured in ui
pub mod saved_command_list;

use crate::client::gen::Generator;
use crate::config::config_client::default_key_path;
use crate::ui::rust_slint_bridge::RustSlintBridge;

use std::error::Error;
use std::thread;

pub fn run_ui() -> Result<(), Box<dyn Error>> {
    let key_path = default_key_path();

    let rust_slint_bridge = RustSlintBridge::create(&key_path)?;

    rust_slint_bridge.add_on_reset_commands_config();
    rust_slint_bridge.add_on_set_commands_config();
    rust_slint_bridge.add_on_update_application();
    rust_slint_bridge.add_on_add_command();
    rust_slint_bridge.add_on_del_command();
    rust_slint_bridge.add_on_exec_command();

    let executor = rust_slint_bridge.create_executor();
    if key_path.exists() {
        executor.enable_key_gen_popup();
        thread::spawn(move || {
            // TODO: handle errors gracefully
            Generator::create(&key_path).unwrap().gen().unwrap();
            let executor_copy = executor.clone();
            slint::invoke_from_event_loop(move || executor_copy.set_public_key().unwrap()).unwrap();
            let executor_copy = executor.clone();
            slint::invoke_from_event_loop(move || executor_copy.disable_key_gen_popup()).unwrap();
        });
    } else {
        executor.set_public_key()?;
    }

    rust_slint_bridge.run()?;

    Ok(())
}
