use crate::client::gen;
use crate::config_client::{default_private_pem_path, default_public_pem_path, DEFAULT_KEY_SIZE};
use crate::slint_bridge;
use crate::slint_bridge::SlintBridge;
use slint::ComponentHandle;
use slint_bridge::App;
use std::error::Error;
use std::path::PathBuf;

pub fn run_ui() -> Result<(), Box<dyn Error>> {
    let public_pem_path = default_public_pem_path();
    let private_pem_path = default_private_pem_path();
    generate_pem_files(&public_pem_path, &private_pem_path)?;

    let app = App::new()?;

    let slint_bridge = SlintBridge::create(&app, private_pem_path, public_pem_path)?;

    slint_bridge.add_on_add_command(&app);
    slint_bridge.add_on_del_command(&app);
    slint_bridge.add_on_exec_command(&app);

    app.run()?;

    Ok(())
}

fn generate_pem_files(public_pem_path: &PathBuf, private_pem_path: &PathBuf) -> Result<(), String> {
    match (private_pem_path.clone(), public_pem_path.clone()) {
        (pr, pu) if !pr.exists() && !pu.exists() => {
            gen(pr, pu, DEFAULT_KEY_SIZE as u32)?;
            Ok(())
        }
        (pr, pu) if pr.exists() && pu.exists() => Ok(()),
        (_, _) => {
            Err("Invalid public/private pem files state - there should be both or neither"
                .to_string())
        }
    }
}
