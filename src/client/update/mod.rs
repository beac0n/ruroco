mod filesystem;
mod github;

pub(crate) use github::GithubApiAsset;
use github::{
    CLIENT_BIN_NAME, CLIENT_UI_BIN_NAME, COMMANDER_BIN_NAME, GH_RELEASES_URL, RELEASE_PUBLIC_KEY,
    SERVER_BIN_DIR, SERVER_BIN_NAME,
};

use crate::common::info;
use anyhow::{bail, Context};
use std::env;
use std::env::consts::{ARCH, OS};
use std::path::PathBuf;

#[derive(Debug)]
pub(crate) struct Updater {
    pub(super) force: bool,
    pub(super) version: Option<String>,
    pub(super) bin_path: PathBuf,
    pub(super) server: bool,
    /// Ed25519 public key (PEM) used to verify downloaded binaries. Defaults to the
    /// embedded release key; overridable in tests for hermetic signing.
    pub(super) public_key_pem: Vec<u8>,
    /// GitHub releases API URL. Defaults to the real endpoint; overridable in tests.
    pub(super) releases_url: String,
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
            Some(p) if !Self::check_if_writable(&p)? => {
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
            public_key_pem: RELEASE_PUBLIC_KEY.to_vec(),
            releases_url: GH_RELEASES_URL.to_string(),
        })
    }

    pub(crate) fn update(&self) -> anyhow::Result<()> {
        let current_version = format!("v{}", env!("CARGO_PKG_VERSION"));

        if !self.force && Some(current_version.clone()) == self.version {
            info(format!("Already using version {current_version}"));
            return Ok(());
        }

        let api_data = Self::get_github_api_data_from(&self.releases_url, self.version.as_ref())?;

        if !self.force && current_version.clone() == api_data.tag_name {
            info(format!("Already using version {current_version}"));
            return Ok(());
        }

        let assets = &api_data.assets;

        if self.server {
            let commander_bin_name = format!("commander-{}-{}-{}", api_data.tag_name, ARCH, OS);
            self.download_and_save_bin(
                self.get_download_url(assets, &commander_bin_name)?,
                self.get_download_url(assets, &format!("{commander_bin_name}.sig"))?,
                COMMANDER_BIN_NAME,
                0o100, // execute for owner
                None,
            )?;

            let server_bin_name = format!("server-{}-{}-{}", api_data.tag_name, ARCH, OS);
            self.download_and_save_bin(
                self.get_download_url(assets, &server_bin_name)?,
                self.get_download_url(assets, &format!("{server_bin_name}.sig"))?,
                SERVER_BIN_NAME,
                0o500, // read|execute for owner
                Some("ruroco"),
            )?;
        } else {
            let client_bin_name = format!("client-{}-{}-{}", api_data.tag_name, ARCH, OS);
            self.download_and_save_bin(
                self.get_download_url(assets, &client_bin_name)?,
                self.get_download_url(assets, &format!("{client_bin_name}.sig"))?,
                CLIENT_BIN_NAME,
                0o755, // read|write|execute for owner, read|execute for group and others.
                None,
            )?;

            let client_ui_bin_name = format!("client-ui-{}-{}-{}", api_data.tag_name, ARCH, OS);
            self.download_and_save_bin(
                self.get_download_url(assets, &client_ui_bin_name)?,
                self.get_download_url(assets, &format!("{client_ui_bin_name}.sig"))?,
                CLIENT_UI_BIN_NAME,
                0o755, // read|write|execute for owner, read|execute for group and others.
                None,
            )?;
        }

        Ok(())
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
            .ok_or_else(|| anyhow::anyhow!("Could not find {client_bin_name}"))
    }
}

#[cfg(test)]
mod tests {
    use crate::client::update::{GithubApiAsset, Updater, GH_RELEASES_URL};
    use openssl::pkey::{PKey, Private};
    use openssl::sign::Signer;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::os::unix::fs::PermissionsExt;
    use std::path::{Path, PathBuf};
    use std::thread::JoinHandle;
    use std::{env, fs};

    fn create_updater(dir: &Path) -> Updater {
        Updater::create(false, None, Some(dir.to_path_buf()), false).unwrap()
    }

    /// Builds an Updater that verifies against `public_key_pem` (so tests can sign payloads
    /// with a matching private key) instead of the embedded release key.
    fn updater_with_key(dir: &Path, public_key_pem: Vec<u8>) -> Updater {
        Updater {
            force: false,
            version: None,
            bin_path: dir.to_path_buf(),
            server: false,
            public_key_pem,
            releases_url: GH_RELEASES_URL.to_string(),
        }
    }

    fn test_keypair() -> (Vec<u8>, PKey<Private>) {
        let key = PKey::generate_ed25519().unwrap();
        (key.public_key_to_pem().unwrap(), key)
    }

    fn sign_bytes(key: &PKey<Private>, message: &[u8]) -> Vec<u8> {
        Signer::new_without_digest(key).unwrap().sign_oneshot_to_vec(message).unwrap()
    }

    fn create_readonly_dir(parent: &Path) -> PathBuf {
        let dir = parent.join("readonly");
        fs::create_dir_all(&dir).unwrap();
        fs::set_permissions(&dir, fs::Permissions::from_mode(0o444)).unwrap();
        dir
    }

    /// Spawns a local HTTP server that serves `payload` once, returns (port, join handle).
    fn serve_payload(payload: Vec<u8>) -> (u16, JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let handle = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut buf = [0u8; 1024];
            let _ = stream.read(&mut buf);
            let resp = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                payload.len()
            );
            stream.write_all(resp.as_bytes()).unwrap();
            stream.write_all(&payload).unwrap();
        });
        (port, handle)
    }

    fn make_asset(name: &str, url: &str) -> GithubApiAsset {
        GithubApiAsset {
            name: name.to_string(),
            browser_download_url: url.to_string(),
        }
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
    fn test_check_if_writable() {
        let dir = tempfile::tempdir().unwrap();
        assert!(Updater::check_if_writable(dir.path()).unwrap());
    }

    #[test]
    fn test_check_if_writable_readonly() {
        let dir = tempfile::tempdir().unwrap();
        let readonly_dir = create_readonly_dir(dir.path());
        assert!(!Updater::check_if_writable(&readonly_dir).unwrap());
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
    fn test_validate_dir_path_existing_writable_dir() {
        let dir = tempfile::tempdir().unwrap();
        let result = Updater::validate_dir_path(dir.path().to_path_buf()).unwrap();
        assert_eq!(result, dir.path());
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
        let data = Updater::get_github_api_data_from(GH_RELEASES_URL, None).unwrap();
        assert!(!data.tag_name.is_empty());
        assert!(!data.assets.is_empty());
    }

    #[test_with::env(TEST_UPDATER)]
    #[test]
    fn test_get_github_api_data_specific_version() {
        assert_eq!(
            Updater::get_github_api_data_from(GH_RELEASES_URL, Some(&"v0.10.0".to_string()))
                .unwrap()
                .tag_name,
            "v0.10.0"
        );
    }

    #[test_with::env(TEST_UPDATER)]
    #[test]
    fn test_get_github_api_data_nonexistent_version() {
        assert!(Updater::get_github_api_data_from(GH_RELEASES_URL, Some(&"v99.99.99".to_string()))
            .is_err());
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
    fn test_create_server_mode_auto_bin_path() {
        // server mode with no bin_path falls back to SERVER_BIN_DIR; whether /usr/local/bin is
        // writable decides ok vs err, so we only assert it doesn't panic.
        let _ = Updater::create(false, None, None, true);
    }

    #[test]
    fn test_create_no_home_env_returns_error() {
        env::remove_var("RUROCO_CONF_DIR");
        env::remove_var("HOME");
        let result = Updater::create(false, None, None, false);
        assert!(result.unwrap_err().to_string().contains("Could not get home env"));
    }

    #[test]
    fn test_download_and_save_bin_creates_file() {
        let (pub_pem, key) = test_keypair();
        let content = b"fake-binary-content".to_vec();
        let sig = sign_bytes(&key, &content);
        let (bin_port, bin_handle) = serve_payload(content.clone());
        let (sig_port, sig_handle) = serve_payload(sig);
        let dir = tempfile::tempdir().unwrap();
        let updater = updater_with_key(dir.path(), pub_pem);
        let result = updater.download_and_save_bin(
            format!("http://127.0.0.1:{bin_port}/bin"),
            format!("http://127.0.0.1:{sig_port}/sig"),
            "tb",
            0o755,
            None,
        );
        bin_handle.join().unwrap();
        sig_handle.join().unwrap();
        assert!(result.is_ok(), "download_and_save_bin failed: {result:?}");

        let target = dir.path().join("tb");
        assert_eq!(fs::read(&target).unwrap(), content);
        assert_eq!(fs::metadata(&target).unwrap().permissions().mode() & 0o777, 0o755);
    }

    #[test]
    fn test_download_and_save_bin_renames_existing_to_old() {
        let (pub_pem, key) = test_keypair();
        let content = b"new-binary".to_vec();
        let sig = sign_bytes(&key, &content);
        let (bin_port, bin_handle) = serve_payload(content.clone());
        let (sig_port, sig_handle) = serve_payload(sig);
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("tb"), b"old-binary").unwrap();

        let updater = updater_with_key(dir.path(), pub_pem);
        let result = updater.download_and_save_bin(
            format!("http://127.0.0.1:{bin_port}/bin"),
            format!("http://127.0.0.1:{sig_port}/sig"),
            "tb",
            0o755,
            None,
        );
        bin_handle.join().unwrap();
        sig_handle.join().unwrap();
        assert!(result.is_ok(), "download_and_save_bin failed: {result:?}");

        assert_eq!(fs::read(dir.path().join("tb")).unwrap(), b"new-binary");
        assert_eq!(fs::read(dir.path().join("tb.old")).unwrap(), b"old-binary");
    }

    #[test]
    fn test_download_and_save_bin_download_failure() {
        let dir = tempfile::tempdir().unwrap();
        let result = create_updater(dir.path()).download_and_save_bin(
            "http://127.0.0.1:1/nonexistent".to_string(),
            "http://127.0.0.1:1/nonexistent.sig".to_string(),
            "tb",
            0o755,
            None,
        );
        assert!(result.unwrap_err().to_string().contains("Could not get binary"));
    }

    #[test]
    fn test_download_and_save_bin_invalid_signature_aborts() {
        let (pub_pem, key) = test_keypair();
        let content = b"genuine-binary".to_vec();
        // Sign different bytes so the signature does not match the served binary.
        let sig = sign_bytes(&key, b"some-other-bytes");
        let (bin_port, bin_handle) = serve_payload(content);
        let (sig_port, sig_handle) = serve_payload(sig);
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("tb"), b"old-binary").unwrap();

        let updater = updater_with_key(dir.path(), pub_pem);
        let result = updater.download_and_save_bin(
            format!("http://127.0.0.1:{bin_port}/bin"),
            format!("http://127.0.0.1:{sig_port}/sig"),
            "tb",
            0o755,
            None,
        );
        bin_handle.join().unwrap();
        sig_handle.join().unwrap();

        assert!(result.unwrap_err().to_string().contains("Signature verification failed"));
        // The live binary must be untouched and no .old left behind.
        assert_eq!(fs::read(dir.path().join("tb")).unwrap(), b"old-binary");
        assert!(!dir.path().join("tb.old").exists());
    }

    #[test]
    fn test_download_and_save_bin_with_empty_user_group() {
        let (pub_pem, key) = test_keypair();
        let content = b"binary-with-ownership".to_vec();
        let sig = sign_bytes(&key, &content);
        let (bin_port, bin_handle) = serve_payload(content.clone());
        let (sig_port, sig_handle) = serve_payload(sig);
        let dir = tempfile::tempdir().unwrap();
        let updater = updater_with_key(dir.path(), pub_pem);
        let result = updater.download_and_save_bin(
            format!("http://127.0.0.1:{bin_port}/bin"),
            format!("http://127.0.0.1:{sig_port}/sig"),
            "tb",
            0o755,
            Some(""),
        );
        bin_handle.join().unwrap();
        sig_handle.join().unwrap();
        assert!(result.is_ok(), "download_and_save_bin with ownership failed: {result:?}");
        assert_eq!(fs::read(dir.path().join("tb")).unwrap(), content);
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

    #[test]
    fn test_update_no_force_version_matches_skips_network() {
        let dir = tempfile::tempdir().unwrap();
        let current_version = format!("v{}", env!("CARGO_PKG_VERSION"));
        let updater =
            Updater::create(false, Some(current_version), Some(dir.path().to_path_buf()), false)
                .unwrap();
        assert!(updater.update().is_ok());
    }
}
