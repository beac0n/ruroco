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
    use crate::client::update::{GithubApiAsset, Updater};
    use crate::common::get_random_string;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::os::unix::fs::PermissionsExt;
    use std::path::{Path, PathBuf};
    use std::thread::JoinHandle;
    use std::{env, fs};

    fn create_updater(dir: &Path) -> Updater {
        Updater::create(false, None, Some(dir.to_path_buf()), false).unwrap()
    }

    fn create_readonly_dir(parent: &Path) -> PathBuf {
        let dir = parent.join("readonly");
        fs::create_dir_all(&dir).unwrap();
        fs::set_permissions(&dir, fs::Permissions::from_mode(0o444)).unwrap();
        dir
    }

    /// Spawns a local HTTP server that serves `payload` once, returns (port, join handle).
    fn serve_payload(payload: &'static [u8]) -> (u16, JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let handle = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut buf = [0u8; 1024];
            let _ = stream.read(&mut buf);
            let resp = format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n", payload.len());
            stream.write_all(resp.as_bytes()).unwrap();
            stream.write_all(payload).unwrap();
        });
        (port, handle)
    }

    fn make_asset(name: &str, url: &str) -> GithubApiAsset {
        GithubApiAsset {
            name: name.to_string(),
            browser_download_url: url.to_string(),
        }
    }

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
        assert!(result.unwrap_err().to_string().contains("does not exist or is not a directory"));
    }

    #[test]
    fn test_create_with_file_as_bin_path() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("not_a_dir");
        fs::write(&file_path, "test").unwrap();
        let result = Updater::create(false, None, Some(file_path), false);
        assert!(result.unwrap_err().to_string().contains("does not exist or is not a directory"));
    }

    #[test]
    fn test_create_with_valid_bin_path() {
        let dir = tempfile::tempdir().unwrap();
        assert!(Updater::create(false, None, Some(dir.path().to_path_buf()), false).is_ok());
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
        let readonly_dir = create_readonly_dir(dir.path());
        assert!(!Updater::check_if_writeable(&readonly_dir).unwrap());
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
        assert!(Updater::validate_dir_path(file_path)
            .unwrap_err()
            .to_string()
            .contains("exists but is not a directory"));
    }

    #[test]
    fn test_get_download_url_found() {
        let dir = tempfile::tempdir().unwrap();
        let updater = create_updater(dir.path());
        let assets = vec![
            make_asset("client-v1.0.0-x86_64-linux", "https://example.com/client"),
            make_asset("server-v1.0.0-x86_64-linux", "https://example.com/server"),
        ];
        let result =
            updater.get_download_url(&assets, &"client-v1.0.0-x86_64-linux".to_string()).unwrap();
        assert_eq!(result, "https://example.com/client");
    }

    #[test]
    fn test_get_download_url_not_found() {
        let dir = tempfile::tempdir().unwrap();
        let updater = create_updater(dir.path());
        let assets = vec![make_asset("other-binary", "https://example.com/other")];
        assert!(updater
            .get_download_url(&assets, &"nonexistent".to_string())
            .unwrap_err()
            .to_string()
            .contains("Could not find nonexistent"));
    }

    #[test_with::env(TEST_UPDATER)]
    #[test]
    fn test_update_already_current_version() {
        let dir = tempfile::tempdir().unwrap();
        let current_version = format!("v{}", env!("CARGO_PKG_VERSION"));
        let updater =
            Updater::create(false, Some(current_version), Some(dir.path().to_path_buf()), false)
                .unwrap();
        assert!(updater.update().is_ok());
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
        assert_eq!(Updater::get_github_api_data(Some(&"v0.10.0".to_string())).unwrap().tag_name, "v0.10.0");
    }

    #[test_with::env(TEST_UPDATER)]
    #[test]
    fn test_get_github_api_data_nonexistent_version() {
        assert!(Updater::get_github_api_data(Some(&"v99.99.99".to_string())).is_err());
    }

    #[test]
    fn test_create_with_readonly_bin_path() {
        let dir = tempfile::tempdir().unwrap();
        let readonly_dir = create_readonly_dir(dir.path());
        let result = Updater::create(false, None, Some(readonly_dir.clone()), false);
        assert!(result.unwrap_err().to_string().contains("can't write to"));
        let _ = fs::set_permissions(&readonly_dir, fs::Permissions::from_mode(0o755));
    }

    #[test]
    fn test_validate_dir_path_readonly() {
        let dir = tempfile::tempdir().unwrap();
        let readonly_dir = create_readonly_dir(dir.path());
        let result = Updater::validate_dir_path(readonly_dir.clone());
        assert!(result.unwrap_err().to_string().contains("can't write to"));
        let _ = fs::set_permissions(&readonly_dir, fs::Permissions::from_mode(0o755));
    }

    #[test]
    fn test_download_and_save_bin_creates_file() {
        let (port, handle) = serve_payload(b"fake-binary-content");
        let dir = tempfile::tempdir().unwrap();
        let updater = create_updater(dir.path());
        let result =
            updater.download_and_save_bin(format!("http://127.0.0.1:{port}/bin"), "tb", 0o755, None);
        handle.join().unwrap();
        assert!(result.is_ok(), "download_and_save_bin failed: {result:?}");

        let target = dir.path().join("tb");
        assert_eq!(fs::read(&target).unwrap(), b"fake-binary-content");
        assert_eq!(fs::metadata(&target).unwrap().permissions().mode() & 0o777, 0o755);
    }

    #[test]
    fn test_download_and_save_bin_renames_existing_to_old() {
        let (port, handle) = serve_payload(b"new-binary");
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("tb"), b"old-binary").unwrap();

        let updater = create_updater(dir.path());
        let result =
            updater.download_and_save_bin(format!("http://127.0.0.1:{port}/bin"), "tb", 0o755, None);
        handle.join().unwrap();
        assert!(result.is_ok(), "download_and_save_bin failed: {result:?}");

        assert_eq!(fs::read(dir.path().join("tb")).unwrap(), b"new-binary");
        assert_eq!(fs::read(dir.path().join("tb.old")).unwrap(), b"old-binary");
    }

    #[test]
    fn test_download_and_save_bin_download_failure() {
        let dir = tempfile::tempdir().unwrap();
        let result = create_updater(dir.path()).download_and_save_bin(
            "http://127.0.0.1:1/nonexistent".to_string(),
            "tb",
            0o755,
            None,
        );
        assert!(result.unwrap_err().to_string().contains("Could not get binary"));
    }

    #[test]
    fn test_create_no_bin_path_client() {
        let dir = tempfile::tempdir().unwrap();
        let bin_dir = dir.path().join(".local").join("bin");
        env::set_var("HOME", dir.path());
        let updater = Updater::create(false, None, None, false).unwrap();
        assert_eq!(updater.bin_path, bin_dir);
        assert!(bin_dir.exists());
    }
}
