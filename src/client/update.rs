use crate::common::{change_file_ownership, info};
use rand::distr::{Alphanumeric, SampleString};
use reqwest::blocking::{get, Client};
use serde::{Deserialize, Serialize};
use std::env::consts::{ARCH, OS};
use std::path::{Path, PathBuf};
use std::{env, fs};

#[derive(Serialize, Deserialize, Debug)]
pub struct GithubApiAsset {
    pub name: String,
    pub browser_download_url: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GithubApiData {
    pub tag_name: String,
    pub assets: Vec<GithubApiAsset>,
}

const GH_RELEASES_URL: &str = "https://api.github.com/repos/beac0n/ruroco/releases";

fn check_if_writeable(path: &Path) -> Result<bool, String> {
    let tmp_path = path.join(Alphanumeric.sample_string(&mut rand::rng(), 16));
    match fs::write(&tmp_path, b"test") {
        Ok(_) => {
            fs::remove_file(&tmp_path)
                .map_err(|e| format!("Could not remove temporary test file {tmp_path:?}: {e}"))?;
            Ok(true)
        }
        Err(_) => Ok(false),
    }
}

/// Update the client binary to the latest version
///
/// * `force` - force the update even if the client is already up to date
pub fn update(
    force: bool,
    version: Option<String>,
    bin_path: Option<PathBuf>,
    server: bool,
) -> Result<(), String> {
    let bin_path = match bin_path {
        Some(p) if !p.exists() || !p.is_dir() => {
            return Err(format!("{p:?} does not exist or is not a directory"))
        }
        Some(p) if !check_if_writeable(&p)? => {
            return Err(format!("can't write to {p:?}"));
        }
        Some(p) => p,
        None if server => validate_dir_path(PathBuf::from("/usr/local/bin"))?,
        None => {
            let home_env = env::var("HOME").map_err(|e| format!("Could not get home env: {e}"))?;
            validate_dir_path(PathBuf::from(home_env).join(".bin"))?
        }
    };

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
        return Err(format!("Request failed: {} - {}", status_code, response_text));
    }

    let current_version = format!("v{}", env!("CARGO_PKG_VERSION"));
    let version_to_download = version.as_ref().unwrap_or(&current_version);

    let response_data: Vec<GithubApiData> =
        response.json().map_err(|e| format!("Could not parse json: {e}"))?;
    let api_data = match response_data.into_iter().find(|d| d.tag_name == *version_to_download) {
        None => return Err(format!("Could not find version {version_to_download}")),
        Some(_) if current_version == *version_to_download && !force => {
            info(&format!("Already using the latest version: {current_version}"));
            return Ok(());
        }
        Some(d) => d,
    };

    if server {
        let commander_bin_name = format!("commander-{}-{}-{}", api_data.tag_name, ARCH, OS);
        download_and_save_bin(
            get_download_url(&api_data.assets, &commander_bin_name)?,
            &bin_path.join("ruroco-commander"),
            0o100, // execute for owner
            None,
        )?;

        let server_bin_name = format!("server-{}-{}-{}", api_data.tag_name, ARCH, OS);
        download_and_save_bin(
            get_download_url(&api_data.assets, &server_bin_name)?,
            &bin_path.join("ruroco-server"),
            0o500, // read|execute for owner
            Some("ruroco"),
        )?;
    } else {
        let client_bin_name = format!("client-{}-{}-{}", api_data.tag_name, ARCH, OS);
        download_and_save_bin(
            get_download_url(&api_data.assets, &client_bin_name)?,
            &bin_path.join("ruroco-client"),
            0o755, // read|write|execute for owner, read|execute for group and others.
            None,
        )?;

        let client_ui_bin_name = format!("client-ui-{}-{}-{}", api_data.tag_name, ARCH, OS);
        download_and_save_bin(
            get_download_url(&api_data.assets, &client_ui_bin_name)?,
            &bin_path.join("ruroco-client-ui"),
            0o755, // read|write|execute for owner, read|execute for group and others.
            None,
        )?;
    }

    Ok(())
}

fn validate_dir_path(dir_path: PathBuf) -> Result<PathBuf, String> {
    match dir_path {
        p if !p.exists() => {
            fs::create_dir_all(&p)
                .map_err(|e| format!("Could not create .bin directory: {e:?}"))?;
            Ok(p)
        }
        p if !p.is_dir() => Err(format!("{p:?} exists but is not a directory")),
        p if !check_if_writeable(&p)? => Err(format!("can't write to {p:?}")),
        p => Ok(p),
    }
}

fn get_download_url(
    assets: &Vec<GithubApiAsset>,
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
    bin_url: String,
    target_bin_path: &Path,
    permissions_mode: u32,
    user_and_group: Option<&str>,
) -> Result<(), String> {
    info(&format!("downloading from {bin_url}"));

    let bin_resp_bytes = get(bin_url)
        .map_err(|e| format!("Could not get binary: {e}"))?
        .bytes()
        .map_err(|e| format!("Could not get bytes: {e}"))?;

    let target_bin_path_str =
        target_bin_path.to_str().ok_or(format!("Could not convert {target_bin_path:?} to str"))?;

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
        use std::os::unix::fs::PermissionsExt;
        let metadata = fs::metadata(target_bin_path_str)
            .map_err(|e| format!("Could not get exe path meta data: {e}"))?;
        let mut permissions = metadata.permissions();
        permissions.set_mode(permissions_mode); // 0o755
        fs::set_permissions(target_bin_path_str, permissions)
            .map_err(|e| format!("Could not set file permissions: {e}"))?;

        if let Some(ug) = user_and_group {
            change_file_ownership(target_bin_path, ug, ug)?
        }
    }
    Ok(())
}
