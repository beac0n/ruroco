use crate::client::update::Updater;
use crate::client::wizard::Wizard;
use crate::common::info;
use anyhow::Context;
use std::process::Command;

pub(super) const CONFIG_TOML_PATH: &str = "/etc/ruroco/config.toml";
pub(super) const COMMANDS_TOML_PATH: &str = "/etc/ruroco/commands.toml";
pub(super) const RUROCO_SERVICE_FILE_PATH: &str = "/etc/systemd/system/ruroco.service";
pub(super) const COMMANDER_SERVICE_FILE_PATH: &str = "/etc/systemd/system/ruroco-commander.service";
pub(super) const SOCKET_FILE_PATH: &str = "/etc/systemd/system/ruroco.socket";

pub(super) const CONFIG_TOML_FILE_DATA: &[u8] = include_bytes!("../../../config/config.toml");
pub(super) const COMMANDS_TOML_FILE_DATA: &[u8] = include_bytes!("../../../config/commands.toml");
pub(super) const RUROCO_SERVICE_FILE_DATA: &[u8] =
    include_bytes!("../../../systemd/ruroco.service");
pub(super) const COMMANDER_SERVICE_FILE_DATA: &[u8] =
    include_bytes!("../../../systemd/ruroco-commander.service");
pub(super) const SOCKET_FILE_DATA: &[u8] = include_bytes!("../../../systemd/ruroco.socket");

impl Wizard {
    pub(super) fn update() -> anyhow::Result<()> {
        info("Updating/Installing ruroco server binaries");
        Updater::create(true, None, None, true)?.update()
    }

    pub(super) fn start_systemd_services() -> anyhow::Result<()> {
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

    pub(super) fn enable_systemd_services() -> anyhow::Result<()> {
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

    pub(super) fn reload_systemd_daemon() -> anyhow::Result<()> {
        info("Reloading systemd daemon ...");
        Command::new("systemctl")
            .arg("daemon-reload")
            .status()
            .with_context(|| "Failed to reload systemd")?;
        Ok(())
    }

    pub(super) fn create_ruroco_user() -> anyhow::Result<()> {
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
}
