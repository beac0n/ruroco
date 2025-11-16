use crate::client::util::set_permissions;
use crate::common::{change_file_ownership, get_random_string, info};
use reqwest::blocking::{get, Client};
use serde::{Deserialize, Serialize};
use std::env::consts::{ARCH, OS};
use std::path::{Path, PathBuf};
use std::{env, fs};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GithubApiAsset {
    pub name: String,
    pub browser_download_url: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GithubApiData {
    pub tag_name: String,
    pub assets: Vec<GithubApiAsset>,
}

const GH_RELEASES_URL: &str = "https://api.github.com/repos/beac0n/ruroco/releases";
pub const SERVER_BIN_DIR: &str = "/usr/local/bin";
pub const COMMANDER_BIN_NAME: &str = "ruroco-commander";
pub const SERVER_BIN_NAME: &str = "ruroco-server";
pub const CLIENT_BIN_NAME: &str = "ruroco-client";
pub const CLIENT_UI_BIN_NAME: &str = "ruroco-client-ui";

#[derive(Debug)]
pub struct Updater {
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
    pub fn create(
        force: bool,
        version: Option<String>,
        bin_path: Option<PathBuf>,
        server: bool,
    ) -> Result<Self, String> {
        let bin_path = match bin_path.clone() {
            Some(p) if !p.exists() || !p.is_dir() => {
                return Err(format!("{p:?} does not exist or is not a directory"))
            }
            Some(p) if !Self::check_if_writeable(&p)? => {
                return Err(format!("can't write to {p:?}"));
            }
            Some(p) => p,
            None if server => Self::validate_dir_path(PathBuf::from(SERVER_BIN_DIR))?,
            None => {
                let home_env =
                    env::var("HOME").map_err(|e| format!("Could not get home env: {e}"))?;
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

    pub fn update(&self) -> Result<(), String> {
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

    pub fn get_github_api_data(
        version_to_download: Option<&String>,
    ) -> Result<GithubApiData, String> {
        let response = Client::builder()
            .user_agent("rust-client")
            .build()
            .map_err(|e| format!("Could not build client: {e}"))?
            .get(GH_RELEASES_URL)
            .send()
            .map_err(|e| format!("Could not get API response: {e}"))?;

        let status_code = response.status();
        if !status_code.is_success() {
            let response_text =
                response.text().map_err(|e| format!("Could not get text from response: {e}"))?;
            return Err(format!("Request failed: {status_code} - {response_text}"));
        }

        let response_data: Vec<GithubApiData> =
            response.json().map_err(|e| format!("Could not parse json: {e}"))?;

        let data = match version_to_download {
            None => response_data.first().cloned(),
            Some(v) => response_data.into_iter().find(|d| d.tag_name == *v),
        };

        match data {
            None => Err(format!("Could not find version {version_to_download:?}")),
            Some(d) => Ok(d),
        }
    }

    fn check_if_writeable(path: &Path) -> Result<bool, String> {
        let tmp_path = path.join(get_random_string(16)?);
        match fs::write(&tmp_path, b"test") {
            Ok(_) => {
                fs::remove_file(&tmp_path).map_err(|e| {
                    format!("Could not remove temporary test file {tmp_path:?}: {e}")
                })?;
                Ok(true)
            }
            Err(_) => Ok(false),
        }
    }

    fn validate_dir_path(dir_path: PathBuf) -> Result<PathBuf, String> {
        match dir_path {
            p if !p.exists() => {
                fs::create_dir_all(&p)
                    .map_err(|e| format!("Could not create .bin directory: {e:?}"))?;
                Ok(p)
            }
            p if !p.is_dir() => Err(format!("{p:?} exists but is not a directory")),
            p if !Self::check_if_writeable(&p)? => Err(format!("can't write to {p:?}")),
            p => Ok(p),
        }
    }

    fn get_download_url(
        &self,
        assets: &[GithubApiAsset],
        client_bin_name: &String,
    ) -> Result<String, String> {
        assets
            .iter()
            .find_map(|a| {
                if a.name == *client_bin_name {
                    Some(a.browser_download_url.clone())
                } else {
                    None
                }
            })
            .ok_or(format!("Could not find {client_bin_name}"))
    }

    fn download_and_save_bin(
        &self,
        bin_url: String,
        bin_name: &str,
        permissions_mode: u32,
        user_and_group: Option<&str>,
    ) -> Result<(), String> {
        //TODO: Verify release signatures or checksums before swapping binaries to prevent
        // MITM/upstream compromise.
        info(&format!("downloading from {bin_url}"));

        let target_bin_path = &self.bin_path.join(bin_name);

        let bin_resp_bytes = get(bin_url)
            .map_err(|e| format!("Could not get binary: {e}"))?
            .bytes()
            .map_err(|e| format!("Could not get bytes: {e}"))?;

        let target_bin_path_str = target_bin_path
            .to_str()
            .ok_or(format!("Could not convert {target_bin_path:?} to str"))?;

        if target_bin_path.exists() {
            fs::rename(target_bin_path_str, format!("{target_bin_path_str}.old"))
                .map_err(|e| format!("Could not rename existing binary: {e}"))?;
        }

        match fs::write(target_bin_path_str, bin_resp_bytes) {
            Ok(_) => {}
            Err(_) => {
                fs::rename(format!("{target_bin_path_str}.old"), target_bin_path_str)
                    .map_err(|e| format!("Could not recover old binary: {e}"))?;

                return Err(format!("Could not write new binary to {target_bin_path_str}"));
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

        dbg!(&result);
        assert!(result.is_ok());
        assert_eq!(entries.len(), 2);
    }
}
