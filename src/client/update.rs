use crate::client::util::set_permissions;
use crate::common::{change_file_ownership, get_random_string, info};
use anyhow::{anyhow, bail, Context};
use reqwest::blocking::{get, Client};
use serde::{Deserialize, Serialize};
use std::env::consts::{ARCH, OS};
use std::path::{Path, PathBuf};
use std::{env, fs};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub(crate) struct GithubApiAsset {
    pub(crate) name: String,
    pub(crate) browser_download_url: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub(crate) struct GithubApiData {
    pub(crate) tag_name: String,
    pub(crate) assets: Vec<GithubApiAsset>,
}

const GH_RELEASES_URL: &str = "https://api.github.com/repos/beac0n/ruroco/releases";
const SERVER_BIN_DIR: &str = "/usr/local/bin";
const COMMANDER_BIN_NAME: &str = "ruroco-commander";
const SERVER_BIN_NAME: &str = "ruroco-server";
const CLIENT_BIN_NAME: &str = "ruroco-client";
const CLIENT_UI_BIN_NAME: &str = "ruroco-client-ui";

#[derive(Debug)]
pub(crate) struct Updater {
    force: bool,
    version: Option<String>,
    bin_path: PathBuf,
    server: bool,
}

impl Updater {
    /// Create the updater for updating the client binary to the latest version
    ///
    /// * `force` - force the update even if the client is already up to date
    /// * `version` - the version to update to, if not specified, the latest version will be used
    /// * `bin_path` - the path to the directory where the binary will be saved
    /// * `server` - if true, the server binaries will be downloaded instead of the client binaries
    pub(crate) fn create(
        force: bool,
        version: Option<String>,
        bin_path: Option<PathBuf>,
        server: bool,
    ) -> anyhow::Result<Self> {
        let bin_path = match bin_path.clone() {
            Some(p) if !p.exists() || !p.is_dir() => {
                bail!("{p:?} does not exist or is not a directory");
            }
            Some(p) if !Self::check_if_writeable(&p)? => {
                bail!("can't write to {p:?}");
            }
            Some(p) => p,
            None if server => Self::validate_dir_path(PathBuf::from(SERVER_BIN_DIR))?,
            None => {
                let home_env = env::var("HOME").with_context(|| "Could not get home env")?;
                Self::validate_dir_path(PathBuf::from(home_env).join(".local").join("bin"))?
            }
        };

        Ok(Self {
            force,
            version,
            bin_path,
            server,
        })
    }

    pub(crate) fn update(&self) -> anyhow::Result<()> {
        let current_version = format!("v{}", env!("CARGO_PKG_VERSION"));

        if !self.force && Some(current_version.clone()) == self.version {
            info(&format!("Already using version {current_version}"));
            return Ok(());
        }

        let api_data = Self::get_github_api_data(self.version.as_ref())?;

        if !self.force && current_version.clone() == api_data.tag_name {
            info(&format!("Already using version {current_version}"));
            return Ok(());
        }

        let assets = &api_data.assets;

        if self.server {
            let commander_bin_name = format!("commander-{}-{}-{}", api_data.tag_name, ARCH, OS);
            self.download_and_save_bin(
                self.get_download_url(assets, &commander_bin_name)?,
                COMMANDER_BIN_NAME,
                0o100, // execute for owner
                None,
            )?;

            let server_bin_name = format!("server-{}-{}-{}", api_data.tag_name, ARCH, OS);
            self.download_and_save_bin(
                self.get_download_url(assets, &server_bin_name)?,
                SERVER_BIN_NAME,
                0o500, // read|execute for owner
                Some("ruroco"),
            )?;
        } else {
            let client_bin_name = format!("client-{}-{}-{}", api_data.tag_name, ARCH, OS);
            self.download_and_save_bin(
                self.get_download_url(assets, &client_bin_name)?,
                CLIENT_BIN_NAME,
                0o755, // read|write|execute for owner, read|execute for group and others.
                None,
            )?;

            let client_ui_bin_name = format!("client-ui-{}-{}-{}", api_data.tag_name, ARCH, OS);
            self.download_and_save_bin(
                self.get_download_url(assets, &client_ui_bin_name)?,
                CLIENT_UI_BIN_NAME,
                0o755, // read|write|execute for owner, read|execute for group and others.
                None,
            )?;
        }

        Ok(())
    }

    pub(crate) fn get_github_api_data(
        version_to_download: Option<&String>,
    ) -> anyhow::Result<GithubApiData> {
        let response = Client::builder()
            .user_agent("rust-client")
            .build()
            .with_context(|| "Could not build client")?
            .get(GH_RELEASES_URL)
            .send()
            .with_context(|| "Could not get API response")?;

        let status_code = response.status();
        if !status_code.is_success() {
            let response_text =
                response.text().with_context(|| "Could not get text from response")?;
            bail!("Request failed: {status_code} - {response_text}");
        }

        let response_data: Vec<GithubApiData> =
            response.json().with_context(|| "Could not parse json")?;

        let data = match version_to_download {
            None => response_data.first().cloned(),
            Some(v) => response_data.into_iter().find(|d| d.tag_name == *v),
        };

        match data {
            None => Err(anyhow!("Could not find version {version_to_download:?}")),
            Some(d) => Ok(d),
        }
    }

    fn check_if_writeable(path: &Path) -> anyhow::Result<bool> {
        let tmp_path = path.join(get_random_string(16)?);
        match fs::write(&tmp_path, b"test") {
            Ok(_) => {
                fs::remove_file(&tmp_path).with_context(|| {
                    format!("Could not remove temporary test file {tmp_path:?}")
                })?;
                Ok(true)
            }
            Err(_) => Ok(false),
        }
    }

    fn validate_dir_path(dir_path: PathBuf) -> anyhow::Result<PathBuf> {
        match dir_path {
            p if !p.exists() => {
                fs::create_dir_all(&p).with_context(|| "Could not create .bin directory")?;
                Ok(p)
            }
            p if !p.is_dir() => Err(anyhow!("{p:?} exists but is not a directory")),
            p if !Self::check_if_writeable(&p)? => Err(anyhow!("can't write to {p:?}")),
            p => Ok(p),
        }
    }

    fn get_download_url(
        &self,
        assets: &[GithubApiAsset],
        client_bin_name: &String,
    ) -> anyhow::Result<String> {
        assets
            .iter()
            .find_map(|a| {
                if a.name == *client_bin_name {
                    Some(a.browser_download_url.clone())
                } else {
                    None
                }
            })
            .ok_or_else(|| anyhow!("Could not find {client_bin_name}"))
    }

    fn download_and_save_bin(
        &self,
        bin_url: String,
        bin_name: &str,
        permissions_mode: u32,
        user_and_group: Option<&str>,
    ) -> anyhow::Result<()> {
        //TODO: Verify release signatures or checksums before swapping binaries to prevent
        // MITM/upstream compromise.
        info(&format!("downloading from {bin_url}"));

        let target_bin_path = &self.bin_path.join(bin_name);

        let bin_resp_bytes = get(bin_url)
            .with_context(|| "Could not get binary")?
            .bytes()
            .with_context(|| "Could not get bytes")?;

        let target_bin_path_str = target_bin_path
            .to_str()
            .ok_or_else(|| anyhow!("Could not convert {target_bin_path:?} to str"))?;

        if target_bin_path.exists() {
            fs::rename(target_bin_path_str, format!("{target_bin_path_str}.old"))
                .with_context(|| "Could not rename existing binary")?;
        }

        match fs::write(target_bin_path_str, bin_resp_bytes) {
            Ok(_) => {}
            Err(_) => {
                fs::rename(format!("{target_bin_path_str}.old"), target_bin_path_str)
                    .with_context(|| "Could not recover old binary")?;

                bail!("Could not write new binary to {target_bin_path_str}");
            }
        }

        #[cfg(unix)]
        {
            set_permissions(target_bin_path_str, permissions_mode)?;
            if let Some(ug) = user_and_group {
                change_file_ownership(target_bin_path, ug, ug)?
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::client::update::Updater;
    use crate::common::get_random_string;
    use std::os::unix::fs::PermissionsExt;
    use std::path::PathBuf;
    use std::{env, fs};

    #[test_with::env(TEST_UPDATER)]
    #[test]
    fn test_update() {
        let rand_str = get_random_string(16).unwrap();
        let temp_path = env::temp_dir().join(format!("temp_{rand_str}"));
        fs::create_dir_all(&temp_path).unwrap();

        let result = Updater::create(true, None, Some(temp_path.clone()), false).unwrap().update();

        let entries: Vec<String> = fs::read_dir(temp_path)
            .unwrap()
            .filter_map(|entry| entry.ok())
            .filter_map(|entry| entry.path().to_str().map(String::from))
            .collect();

        assert!(result.is_ok());
        assert_eq!(entries.len(), 2);
    }

    #[test]
    fn test_create_with_nonexistent_bin_path() {
        let result =
            Updater::create(false, None, Some(PathBuf::from("/tmp/no_such_dir_ruroco")), false);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("does not exist or is not a directory"));
    }

    #[test]
    fn test_create_with_file_as_bin_path() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("not_a_dir");
        fs::write(&file_path, "test").unwrap();
        let result = Updater::create(false, None, Some(file_path), false);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("does not exist or is not a directory"));
    }

    #[test]
    fn test_create_with_valid_bin_path() {
        let dir = tempfile::tempdir().unwrap();
        let result = Updater::create(false, None, Some(dir.path().to_path_buf()), false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_create_server_mode() {
        let dir = tempfile::tempdir().unwrap();
        let updater =
            Updater::create(true, Some("v1.0.0".to_string()), Some(dir.path().to_path_buf()), true)
                .unwrap();
        assert!(updater.server);
        assert!(updater.force);
    }

    #[test]
    fn test_check_if_writeable() {
        let dir = tempfile::tempdir().unwrap();
        assert!(Updater::check_if_writeable(dir.path()).unwrap());
    }

    #[test]
    fn test_check_if_writeable_readonly() {
        let dir = tempfile::tempdir().unwrap();
        let readonly_dir = dir.path().join("readonly");
        fs::create_dir_all(&readonly_dir).unwrap();
        fs::set_permissions(&readonly_dir, fs::Permissions::from_mode(0o444)).unwrap();
        assert!(!Updater::check_if_writeable(&readonly_dir).unwrap());
        // Restore permissions for cleanup
        let _ = fs::set_permissions(&readonly_dir, fs::Permissions::from_mode(0o755));
    }

    #[test]
    fn test_validate_dir_path_creates_dir() {
        let dir = tempfile::tempdir().unwrap();
        let new_dir = dir.path().join("new_sub_dir");
        assert!(!new_dir.exists());
        let result = Updater::validate_dir_path(new_dir.clone()).unwrap();
        assert!(new_dir.exists());
        assert_eq!(result, new_dir);
    }

    #[test]
    fn test_validate_dir_path_existing_file() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("a_file");
        fs::write(&file_path, "test").unwrap();
        let result = Updater::validate_dir_path(file_path);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("exists but is not a directory"));
    }

    #[test]
    fn test_get_download_url_found() {
        use crate::client::update::GithubApiAsset;
        let dir = tempfile::tempdir().unwrap();
        let updater = Updater::create(false, None, Some(dir.path().to_path_buf()), false).unwrap();
        let assets = vec![
            GithubApiAsset {
                name: "client-v1.0.0-x86_64-linux".to_string(),
                browser_download_url: "https://example.com/client".to_string(),
            },
            GithubApiAsset {
                name: "server-v1.0.0-x86_64-linux".to_string(),
                browser_download_url: "https://example.com/server".to_string(),
            },
        ];
        let result =
            updater.get_download_url(&assets, &"client-v1.0.0-x86_64-linux".to_string()).unwrap();
        assert_eq!(result, "https://example.com/client");
    }

    #[test]
    fn test_get_download_url_not_found() {
        use crate::client::update::GithubApiAsset;
        let dir = tempfile::tempdir().unwrap();
        let updater = Updater::create(false, None, Some(dir.path().to_path_buf()), false).unwrap();
        let assets = vec![GithubApiAsset {
            name: "other-binary".to_string(),
            browser_download_url: "https://example.com/other".to_string(),
        }];
        let result = updater.get_download_url(&assets, &"nonexistent".to_string());
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Could not find nonexistent"));
    }

    #[test_with::env(TEST_UPDATER)]
    #[test]
    fn test_update_already_current_version() {
        let dir = tempfile::tempdir().unwrap();
        let current_version = format!("v{}", env!("CARGO_PKG_VERSION"));
        let updater =
            Updater::create(false, Some(current_version), Some(dir.path().to_path_buf()), false)
                .unwrap();
        let result = updater.update();
        assert!(result.is_ok());
    }

    #[test_with::env(TEST_UPDATER)]
    #[test]
    fn test_get_github_api_data_latest() {
        let data = Updater::get_github_api_data(None).unwrap();
        assert!(!data.tag_name.is_empty());
        assert!(!data.assets.is_empty());
    }

    #[test_with::env(TEST_UPDATER)]
    #[test]
    fn test_get_github_api_data_specific_version() {
        let data = Updater::get_github_api_data(Some(&"v0.10.0".to_string())).unwrap();
        assert_eq!(data.tag_name, "v0.10.0");
    }

    #[test_with::env(TEST_UPDATER)]
    #[test]
    fn test_get_github_api_data_nonexistent_version() {
        let result = Updater::get_github_api_data(Some(&"v99.99.99".to_string()));
        assert!(result.is_err());
    }

    // Note: test_update_server_mode requires a "ruroco" system user to exist for
    // change_file_ownership, so it can only run in the e2e environment.
}
