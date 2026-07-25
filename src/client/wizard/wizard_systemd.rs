use crate::client::update::Updater;
use crate::client::wizard::Wizard;
use crate::common::info;
use anyhow::{ensure, Context};
use std::process::Command;

pub(super) const SERVICE_USER: &str = "ruroco";
pub(super) const SERVICE_GROUP: &str = SERVICE_USER;

pub(super) const RUROCO_SERVICE: &str = "ruroco.service";
pub(super) const COMMANDER_SERVICE: &str = "ruroco-commander.service";
pub(super) const RUROCO_SOCKET: &str = "ruroco.socket";

const KEYLESS_UNITS: [&str; 2] = [COMMANDER_SERVICE, RUROCO_SOCKET];
const ALL_UNITS: [&str; 3] = [COMMANDER_SERVICE, RUROCO_SOCKET, RUROCO_SERVICE];

pub(super) const CONFIG_DIR_MODE: u32 = 0o750;
pub(super) const CONFIG_TOML_MODE: u32 = 0o640;
pub(super) const COMMANDS_TOML_MODE: u32 = 0o600;
pub(super) const UNIT_FILE_MODE: u32 = 0o644;

pub(super) const CONFIG_DIR_PATH: &str = "/etc/ruroco";
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

    pub(super) fn restart_systemd_services(with_server: bool) -> anyhow::Result<()> {
        let units: &[&str] = if with_server {
            &ALL_UNITS
        } else {
            &KEYLESS_UNITS
        };
        info(format!("Restarting systemd units {units:?} ..."));
        Self::systemctl("restart", units)
    }

    pub(super) fn enable_systemd_services() -> anyhow::Result<()> {
        info("Enabling systemd services ...");
        Self::systemctl("enable", &ALL_UNITS)
    }

    pub(super) fn reset_failed_systemd_services() {
        if let Err(e) = Self::systemctl("reset-failed", &ALL_UNITS) {
            info(format!("Could not reset failed state (ignored): {e:#}"));
        }
    }

    fn systemctl(verb: &str, units: &[&str]) -> anyhow::Result<()> {
        let status = Command::new("systemctl")
            .arg(verb)
            .args(units)
            .status()
            .with_context(|| format!("Failed to run 'systemctl {verb}'"))?;
        ensure!(status.success(), "'systemctl {verb}' failed with {status}");
        Ok(())
    }

    pub(super) fn reload_systemd_daemon() -> anyhow::Result<()> {
        info("Reloading systemd daemon ...");
        Self::systemctl("daemon-reload", &[])
    }

    pub(super) fn create_ruroco_user() -> anyhow::Result<()> {
        info("Creating user 'ruroco' ...");
        let status = Command::new("useradd")
            .arg("--system")
            .arg("--user-group")
            .arg("--shell")
            .arg("/bin/false")
            .arg(SERVICE_USER)
            .status()
            .with_context(|| "Failed to create ruroco user")?;
        // useradd exit code 9 means the user already exists: treat as success so wizard re-runs
        // (reinstall/upgrade) stay idempotent
        if status.code() == Some(9) {
            info("user ruroco already exists, skipping creation");
            return Ok(());
        }
        ensure!(status.success(), "'useradd ruroco' failed with {status}");
        Ok(())
    }
}
