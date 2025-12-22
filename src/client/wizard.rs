use crate::client::update::Updater;
use crate::client::util::set_permissions;
use crate::common::info;
use anyhow::Context;
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
pub(crate) struct Wizard {}

impl Wizard {
    pub(crate) fn create() -> Self {
        Self {}
    }

    pub(crate) fn run(&self) -> anyhow::Result<()> {
        Self::create_ruroco_user()?;
        Self::update()?;

        Self::write_data(RUROCO_SERVICE_FILE_PATH, RUROCO_SERVICE_FILE_DATA)?;
        Self::write_data(COMMANDER_SERVICE_FILE_PATH, COMMANDER_SERVICE_FILE_DATA)?;
        Self::write_data(SOCKET_FILE_PATH, SOCKET_FILE_DATA)?;

        Self::init_config_file()?;
        Self::reload_systemd_daemon()?;
        Self::enable_systemd_services()?;
        Self::start_systemd_services()?;

        info("=========================================");
        info("Ruroco Server Side installed successfully");
        info(&format!("Please check the config in {CONFIG_TOML_PATH} and edit it accordingly"));
        info("Generate key file with ruroco-client gen command");
        info(&format!(
            "Save key file you just generated in 'config_dir' as defined in {CONFIG_TOML_PATH}"
        ));
        info("Save key you just generated in your client secure key store");
        info("=========================================");

        Ok(())
    }

    fn update() -> anyhow::Result<()> {
        info("Updating/Installing ruroco server binaries");
        Updater::create(true, None, None, true)?.update()
    }

    fn init_config_file() -> anyhow::Result<()> {
        info(&format!("Initializing config file {CONFIG_TOML_PATH}"));
        if !Path::new(CONFIG_TOML_PATH).exists() {
            Self::write_data(CONFIG_TOML_PATH, CONFIG_TOML_FILE_DATA)?;
        }

        set_permissions(CONFIG_TOML_PATH, 0o600)?; // owner read|write
        Ok(())
    }

    fn start_systemd_services() -> anyhow::Result<()> {
        info("Starting systemd services ...");
        Command::new("systemctl")
            .arg("start")
            .arg("ruroco.service")
            .arg("ruroco-commander.service")
            .arg("ruroco.socket")
            .status()
            .with_context(|| "Failed to start ruroco systemd services")?;
        Ok(())
    }

    fn enable_systemd_services() -> anyhow::Result<()> {
        info("Enabling systemd services ...");
        Command::new("systemctl")
            .arg("enable")
            .arg("ruroco.service")
            .arg("ruroco-commander.service")
            .arg("ruroco.socket")
            .status()
            .with_context(|| "Failed to enable ruroco systemd services")?;
        Ok(())
    }

    fn reload_systemd_daemon() -> anyhow::Result<()> {
        info("Reloading systemd daemon ...");
        Command::new("systemctl")
            .arg("daemon-reload")
            .status()
            .with_context(|| "Failed to reload systemd")?;
        Ok(())
    }

    fn create_ruroco_user() -> anyhow::Result<()> {
        info("Creating user 'ruroco' ...");
        Command::new("useradd")
            .arg("--system")
            .arg("ruroco")
            .arg("--shell")
            .arg("/bin/false")
            .status()
            .with_context(|| "Failed to create ruroco user")?;
        Ok(())
    }

    fn write_data(path: &str, data: &[u8]) -> anyhow::Result<()> {
        info(&format!("Creating {path} ..."));
        let mut file =
            fs::File::create(path).with_context(|| format!("Failed to create {path}"))?;
        file.write_all(data).with_context(|| format!("Failed to write to {path}"))?;
        Ok(())
    }
}
