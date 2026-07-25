use crate::client::util::set_permissions;
use crate::client::wizard::wizard_systemd::{
    COMMANDER_SERVICE_FILE_DATA, COMMANDER_SERVICE_FILE_PATH, COMMANDS_TOML_FILE_DATA,
    COMMANDS_TOML_MODE, COMMANDS_TOML_PATH, CONFIG_DIR_MODE, CONFIG_DIR_PATH,
    CONFIG_TOML_FILE_DATA, CONFIG_TOML_MODE, CONFIG_TOML_PATH, RUROCO_SERVICE,
    RUROCO_SERVICE_FILE_DATA, RUROCO_SERVICE_FILE_PATH, SERVICE_GROUP, SERVICE_USER,
    SOCKET_FILE_DATA, SOCKET_FILE_PATH, UNIT_FILE_MODE,
};
use crate::common::{change_file_ownership, info};
use anyhow::Context;
use std::fs;
use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;
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

        Self::write_data(RUROCO_SERVICE_FILE_PATH, RUROCO_SERVICE_FILE_DATA, UNIT_FILE_MODE)?;
        Self::write_data(COMMANDER_SERVICE_FILE_PATH, COMMANDER_SERVICE_FILE_DATA, UNIT_FILE_MODE)?;
        Self::write_data(SOCKET_FILE_PATH, SOCKET_FILE_DATA, UNIT_FILE_MODE)?;

        Self::init_config_dir()?;
        Self::init_config_file()?;
        Self::init_commands_file()?;
        let has_key = Self::has_key_file(CONFIG_DIR_PATH)?;
        Self::reload_systemd_daemon()?;
        Self::enable_systemd_services()?;
        Self::reset_failed_systemd_services();
        Self::restart_systemd_services(has_key)?;

        info("=========================================");
        info("Ruroco Server Side installed successfully");
        info(format!("Please check the config in {CONFIG_TOML_PATH} and edit it accordingly"));
        if has_key {
            info(format!("{RUROCO_SERVICE} is running the freshly installed binary"));
            info(format!(
                "Restart it yourself after adding or rotating a key: keys are loaded once at \
                 startup, so a new {CONFIG_DIR_PATH}/<name>.key does nothing until \
                 'systemctl restart {RUROCO_SERVICE}'"
            ));
            info("=========================================");
            return Ok(());
        }

        info(format!("{RUROCO_SERVICE} was NOT started: no .key file in {CONFIG_DIR_PATH} yet"));
        info("Generate key file with ruroco-client gen command");
        info(format!(
            "Save key file you just generated in 'config_dir' as defined in {CONFIG_TOML_PATH}"
        ));
        info(format!(
            "Then make it readable for the '{SERVICE_USER}' service user, or the server cannot \
             start: \
             chown root:{SERVICE_GROUP} {CONFIG_DIR_PATH}/<name>.key && \
             chmod 640 {CONFIG_DIR_PATH}/<name>.key"
        ));
        info(format!("Finally start the server: systemctl start {RUROCO_SERVICE}"));
        info("Save key you just generated in your client secure key store");
        info("=========================================");

        Ok(())
    }

    fn has_key_file(dir: &str) -> anyhow::Result<bool> {
        let entries =
            fs::read_dir(dir).with_context(|| format!("Failed to read directory {dir}"))?;
        Ok(entries
            .flatten()
            .map(|e| e.path())
            .any(|p| p.is_file() && p.extension().is_some_and(|e| e == "key")))
    }

    fn create_dir(path: &str, mode: u32) -> anyhow::Result<()> {
        info(format!("Creating directory {path} ..."));
        fs::create_dir_all(path).with_context(|| format!("Failed to create directory {path}"))?;
        set_permissions(path, mode)
    }

    fn init_config_dir() -> anyhow::Result<()> {
        Self::create_dir(CONFIG_DIR_PATH, CONFIG_DIR_MODE)?;
        Self::set_ownership(CONFIG_DIR_PATH, SERVICE_GROUP)
    }

    fn init_config_file() -> anyhow::Result<()> {
        info(format!("Initializing config file {CONFIG_TOML_PATH}"));
        if !Path::new(CONFIG_TOML_PATH).exists() {
            Self::write_data(CONFIG_TOML_PATH, CONFIG_TOML_FILE_DATA, CONFIG_TOML_MODE)?;
        }

        Self::set_ownership(CONFIG_TOML_PATH, SERVICE_GROUP)?;
        set_permissions(CONFIG_TOML_PATH, CONFIG_TOML_MODE)
    }

    fn init_commands_file() -> anyhow::Result<()> {
        info(format!("Initializing commands file {COMMANDS_TOML_PATH}"));
        if !Path::new(COMMANDS_TOML_PATH).exists() {
            Self::write_data(COMMANDS_TOML_PATH, COMMANDS_TOML_FILE_DATA, COMMANDS_TOML_MODE)?;
        }

        Self::set_ownership(COMMANDS_TOML_PATH, "root")?;
        set_permissions(COMMANDS_TOML_PATH, COMMANDS_TOML_MODE)
    }

    fn set_ownership(path: &str, group: &str) -> anyhow::Result<()> {
        change_file_ownership(Path::new(path), "root", group)
    }

    fn write_data(path: &str, data: &[u8], mode: u32) -> anyhow::Result<()> {
        info(format!("Creating {path} ..."));
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(mode)
            .open(path)
            .with_context(|| format!("Failed to create {path}"))?;
        file.write_all(data).with_context(|| format!("Failed to write to {path}"))?;
        set_permissions(path, mode)
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

        Wizard::write_data(path_str, data, 0o600).unwrap();

        let contents = fs::read(&file_path).unwrap();
        assert_eq!(contents, data);
        assert_eq!(fs::metadata(&file_path).unwrap().permissions().mode() & 0o777, 0o600);
    }

    #[test]
    fn test_write_data_never_widens_an_existing_file() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("commands.toml");
        fs::write(&file_path, "old").unwrap();
        fs::set_permissions(&file_path, Permissions::from_mode(0o644)).unwrap();
        let path_str = file_path.to_str().unwrap();

        Wizard::write_data(path_str, b"new", 0o600).unwrap();

        assert_eq!(fs::read(&file_path).unwrap(), b"new");
        assert_eq!(fs::metadata(&file_path).unwrap().permissions().mode() & 0o777, 0o600);
    }

    #[test]
    fn test_write_data_invalid_path() {
        let result = Wizard::write_data("/no/such/dir/file.txt", b"data", 0o600);
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
    fn test_has_key_file_empty_dir() {
        let dir = tempfile::tempdir().unwrap();

        assert!(!Wizard::has_key_file(dir.path().to_str().unwrap()).unwrap());
    }

    #[test]
    fn test_has_key_file_ignores_non_keys() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("config.toml"), "").unwrap();
        fs::write(dir.path().join("keyless"), "").unwrap();
        // a directory named like a key must not count: the server only loads regular files
        fs::create_dir(dir.path().join("nested.key")).unwrap();

        assert!(!Wizard::has_key_file(dir.path().to_str().unwrap()).unwrap());
    }

    #[test]
    fn test_has_key_file_finds_key() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("config.toml"), "").unwrap();
        fs::write(dir.path().join("user.key"), "").unwrap();

        assert!(Wizard::has_key_file(dir.path().to_str().unwrap()).unwrap());
    }

    #[test]
    fn test_has_key_file_missing_dir() {
        let result = Wizard::has_key_file("/no/such/ruroco/dir");

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Failed to read directory"));
    }

    #[test]
    fn test_set_ownership_unknown_group() {
        let dir = tempfile::tempdir().unwrap();
        let path_str = dir.path().to_str().unwrap();

        let result = Wizard::set_ownership(path_str, "no_such_ruroco_group");

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Could not find group"));
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
