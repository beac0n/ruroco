use crate::client::util::set_permissions;
use crate::client::wizard::wizard_systemd::{
    COMMANDER_SERVICE_FILE_DATA, COMMANDER_SERVICE_FILE_PATH, COMMANDS_TOML_FILE_DATA,
    COMMANDS_TOML_PATH, CONFIG_DIR_PATH, CONFIG_TOML_FILE_DATA, CONFIG_TOML_PATH,
    RUROCO_SERVICE_FILE_DATA, RUROCO_SERVICE_FILE_PATH, SOCKET_FILE_DATA, SOCKET_FILE_PATH,
};
use crate::common::info;
use anyhow::Context;
use std::fs;
use std::io::Write;
use std::path::Path;

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

        Self::create_dir(CONFIG_DIR_PATH, 0o755)?;
        Self::init_config_file()?;
        Self::init_commands_file()?;
        Self::reload_systemd_daemon()?;
        Self::enable_systemd_services()?;
        Self::start_systemd_services()?;

        info("=========================================");
        info("Ruroco Server Side installed successfully");
        info(format!("Please check the config in {CONFIG_TOML_PATH} and edit it accordingly"));
        info("Generate key file with ruroco-client gen command");
        info(format!(
            "Save key file you just generated in 'config_dir' as defined in {CONFIG_TOML_PATH}"
        ));
        info("Save key you just generated in your client secure key store");
        info("=========================================");

        Ok(())
    }

    /// Create the config dir if it doesn't exist yet, then force `mode` on it. The mode is set
    /// explicitly instead of left to `create_dir_all`'s umask default: a root shell with
    /// `umask 0077` would otherwise produce a 0700 dir that the `User=ruroco` server can't even
    /// traverse to reach its key file.
    fn create_dir(path: &str, mode: u32) -> anyhow::Result<()> {
        info(format!("Creating directory {path} ..."));
        fs::create_dir_all(path).with_context(|| format!("Failed to create directory {path}"))?;
        set_permissions(path, mode)
    }

    fn init_config_file() -> anyhow::Result<()> {
        info(format!("Initializing config file {CONFIG_TOML_PATH}"));
        if !Path::new(CONFIG_TOML_PATH).exists() {
            Self::write_data(CONFIG_TOML_PATH, CONFIG_TOML_FILE_DATA)?;
        }

        set_permissions(CONFIG_TOML_PATH, 0o600)?; // owner read|write
        Ok(())
    }

    fn init_commands_file() -> anyhow::Result<()> {
        info(format!("Initializing commands file {COMMANDS_TOML_PATH}"));
        if !Path::new(COMMANDS_TOML_PATH).exists() {
            Self::write_data(COMMANDS_TOML_PATH, COMMANDS_TOML_FILE_DATA)?;
        }

        // root-only: read by the commander (root), never by the unprivileged server user
        set_permissions(COMMANDS_TOML_PATH, 0o600)?;
        Ok(())
    }

    fn write_data(path: &str, data: &[u8]) -> anyhow::Result<()> {
        info(format!("Creating {path} ..."));
        let mut file =
            fs::File::create(path).with_context(|| format!("Failed to create {path}"))?;
        file.write_all(data).with_context(|| format!("Failed to write to {path}"))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::Wizard;
    use std::fs;
    use std::fs::Permissions;
    use std::os::unix::fs::PermissionsExt;

    #[test]
    fn test_write_data_success() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("test_file.txt");
        let path_str = file_path.to_str().unwrap();
        let data = b"hello world";

        Wizard::write_data(path_str, data).unwrap();

        let contents = std::fs::read(file_path).unwrap();
        assert_eq!(contents, data);
    }

    #[test]
    fn test_write_data_invalid_path() {
        let result = Wizard::write_data("/no/such/dir/file.txt", b"data");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Failed to create"));
    }

    #[test]
    fn test_create() {
        let wizard = Wizard::create();
        assert_eq!(format!("{wizard:?}"), "Wizard");
    }

    #[test]
    fn test_create_dir_creates_missing_dir_with_mode() {
        let dir = tempfile::tempdir().unwrap();
        let nested = dir.path().join("etc").join("ruroco");
        let path_str = nested.to_str().unwrap();

        Wizard::create_dir(path_str, 0o755).unwrap();

        assert!(nested.is_dir());
        assert_eq!(fs::metadata(&nested).unwrap().permissions().mode() & 0o777, 0o755);
    }

    #[test]
    fn test_create_dir_forces_mode_on_existing_dir() {
        let dir = tempfile::tempdir().unwrap();
        let existing = dir.path().join("ruroco");
        fs::create_dir(&existing).unwrap();
        fs::set_permissions(&existing, Permissions::from_mode(0o700)).unwrap();
        let path_str = existing.to_str().unwrap();

        Wizard::create_dir(path_str, 0o755).unwrap();

        assert!(existing.is_dir());
        assert_eq!(fs::metadata(&existing).unwrap().permissions().mode() & 0o777, 0o755);
    }

    #[test]
    fn test_create_dir_invalid_path() {
        let dir = tempfile::tempdir().unwrap();
        let blocker = dir.path().join("not_a_dir");
        fs::write(&blocker, "").unwrap();
        let child = blocker.join("child");
        let path_str = child.to_str().unwrap();

        let result = Wizard::create_dir(path_str, 0o755);

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Failed to create directory"));
    }
}
