/// used to import everything that is slint related
pub mod rust_slint_bridge;

#[cfg(target_os = "android")]
/// contains library content for android apk
pub mod android;
/// saves commands configured in ui
pub mod saved_command_list;
use crate::client::gen::gen;
use crate::config::config_client::{
    default_private_pem_path, default_public_pem_path, DEFAULT_KEY_SIZE,
};
use crate::ui::rust_slint_bridge::RustSlintBridge;

use std::error::Error;
use std::path::Path;
use std::thread;

pub fn run_ui() -> Result<(), Box<dyn Error>> {
    let public_pem_path = default_public_pem_path();
    let private_pem_path = default_private_pem_path();

    let rust_slint_bridge = RustSlintBridge::create(&public_pem_path, &private_pem_path)?;

    rust_slint_bridge.add_on_add_command();
    rust_slint_bridge.add_on_del_command();
    rust_slint_bridge.add_on_exec_command();

    let executor = rust_slint_bridge.create_executor();
    if check_pem_files(&public_pem_path, &private_pem_path)? {
        executor.enable_key_gen_popup();
        thread::spawn(move || {
            // TODO: handle errors gracefully
            gen(&private_pem_path, &public_pem_path, DEFAULT_KEY_SIZE as u32).unwrap();
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

fn check_pem_files(public_pem_path: &Path, private_pem_path: &Path) -> Result<bool, String> {
    match (private_pem_path, public_pem_path) {
        (pr, pu) if !pr.exists() && !pu.exists() => Ok(true), // both files do not exist -> generate them
        (pr, pu) if pr.exists() && pu.exists() => Ok(false),  // both files exist -> nothing to do
        (_, _) => {
            // one file exists and the other does not -> invalid state
            let msg = "Invalid public/private pem files state - there should be both or neither";
            Err(msg.to_string())
        }
    }
}
