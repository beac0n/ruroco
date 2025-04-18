use crate::client::update::{Updater, COMMANDER_BIN_NAME, SERVER_BIN_DIR, SERVER_BIN_NAME};
use crate::common::{info, set_permissions};
use crate::config::config_client::get_conf_dir;
use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::Command;

const CONFIG_TOML_PATH: &str = "/etc/ruroco/config.toml";
const RUROCO_SERVICE_FILE_PATH: &str = "/etc/systemd/system/ruroco.service";
const COMMANDER_SERVICE_FILE_PATH: &str = "/etc/systemd/system/ruroco-commander.service";
const SOCKET_FILE_PATH: &str = "/etc/systemd/system/ruroco.socket";

const CONFIG_TOML_FILE_DATA: &[u8] = include_bytes!("../../config/config.toml");
const RUROCO_SERVICE_FILE_DATA: &[u8] = include_bytes!("../../systemd/ruroco.service");
const COMMANDER_SERVICE_FILE_DATA: &[u8] = include_bytes!("../../systemd/ruroco-commander.service");
const SOCKET_FILE_DATA: &[u8] = include_bytes!("../../systemd/ruroco.socket");

#[derive(Debug)]
pub struct Wizard {}

impl Wizard {
    pub fn create() -> Self {
        Self {}
    }

    pub fn run(&self) -> Result<(), String> {
        Self::create_ruroco_user()?;
        Updater::create(true, None, None, true)?.update()?;

        Self::write_data(RUROCO_SERVICE_FILE_PATH, RUROCO_SERVICE_FILE_DATA)?;
        Self::write_data(COMMANDER_SERVICE_FILE_PATH, COMMANDER_SERVICE_FILE_DATA)?;
        Self::write_data(SOCKET_FILE_PATH, SOCKET_FILE_DATA)?;

        Self::init_config_file()?;
        Self::reload_systemd_daemon()?;
        Self::enable_systemd_services()?;
        Self::start_systemd_services()?;

        info("### Ruroco Server Side installed successfully");
        info(&format!("# Please check the config in {CONFIG_TOML_PATH} and edit it accordingly"));
        info("# Generate public and private pem files with ruroco-client gen command");
        info(&format!(
            "# Save public pem file you just generated in 'config_dir' as defined in {CONFIG_TOML_PATH}"
        ));
        info(&format!(
            "# Save public and private pem file you just generated on your client in {:?}",
            get_conf_dir()
        ));

        Ok(())
    }

    fn init_config_file() -> Result<(), String> {
        if !Path::new(CONFIG_TOML_PATH).exists() {
            Self::write_data(CONFIG_TOML_PATH, CONFIG_TOML_FILE_DATA)?;
        }

        set_permissions(CONFIG_TOML_PATH, 0o600)?; // owner read|write
        Ok(())
    }

    fn start_systemd_services() -> Result<(), String> {
        Command::new("systemctl")
            .arg("start")
            .arg("ruroco.service")
            .arg("ruroco-commander.service")
            .arg("ruroco.socket")
            .status()
            .map_err(|e| format!("Failed to start ruroco systemd services: {e}"))?;
        Ok(())
    }

    fn enable_systemd_services() -> Result<(), String> {
        Command::new("systemctl")
            .arg("enable")
            .arg("ruroco.service")
            .arg("ruroco-commander.service")
            .arg("ruroco.socket")
            .status()
            .map_err(|e| format!("Failed to enable ruroco systemd services: {e}"))?;
        Ok(())
    }

    fn reload_systemd_daemon() -> Result<(), String> {
        Command::new("systemctl")
            .arg("daemon-reload")
            .status()
            .map_err(|e| format!("Failed to reload systemd: {e}"))?;
        Ok(())
    }

    fn create_ruroco_user() -> Result<(), String> {
        Command::new("useradd")
            .arg("--system")
            .arg("ruroco")
            .arg("--shell")
            .arg("/bin/false")
            .status()
            .map_err(|e| format!("Failed to create ruroco user: {e}"))?;
        Ok(())
    }

    fn write_data(path: &str, data: &[u8]) -> Result<(), String> {
        let mut file =
            fs::File::create(path).map_err(|e| format!("Failed to create {path}: {e}"))?;
        file.write_all(data).map_err(|e| format!("Failed to write to {path}: {e}"))?;
        Ok(())
    }
}
