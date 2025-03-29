use crate::common::info;
use reqwest::blocking::{get, Client};
use serde::{Deserialize, Serialize};
use std::env::consts::{ARCH, OS};
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

/// Update the client binary to the latest version
///
/// * `force` - force the update even if the client is already up to date
pub fn update(force: bool, version: Option<String>) -> Result<(), String> {
    let response = Client::builder()
        .user_agent("rust-client")
        .build()
        .map_err(|e| format!("Could not build client: {e}"))?
        .get("https://api.github.com/repos/beac0n/ruroco/releases")
        .send()
        .map_err(|e| format!("Could not get API response: {e}"))?;

    let status_code = response.status();
    if !status_code.is_success() {
        let response_text =
            response.text().map_err(|e| format!("Could not get text from response: {e}"))?;
        return Err(format!("Request failed: {} - {}", status_code, response_text));
    }

    let current_version = format!("v{}", env!("CARGO_PKG_VERSION"));
    let version_to_download = version.unwrap_or_else(|| current_version.clone());

    let response_data: Vec<GithubApiData> =
        response.json().map_err(|e| format!("Could not parse json: {e}"))?;
    let api_data = match response_data.into_iter().find(|d| d.tag_name == version_to_download) {
        None => return Err(format!("Could not find version {version_to_download}")),
        Some(_) if current_version == version_to_download && !force => {
            info(&format!("Already using the latest version: {current_version}"));
            return Ok(());
        }
        Some(d) => d,
    };

    let bin_name = format!("client-{}-{}-{}", api_data.tag_name, ARCH, OS);
    let download_url = api_data.assets.into_iter().find_map(|a| {
        if a.name == bin_name {
            Some(a.browser_download_url)
        } else {
            None
        }
    });

    match download_url {
        Some(bin_url) => download_bin(&bin_name, &bin_url),
        None => Err(format!(
            "Could not find {bin_name} on https://github.com/beac0n/ruroco/releases/latest"
        )),
    }
}

fn download_bin(binary_name: &String, bin_url: &String) -> Result<(), String> {
    info(&format!("downloading {binary_name:?} from {bin_url:?}"));
    let current_exe = env::current_exe().map_err(|e| format!("Could not get exe path: {e}"))?;
    let exe_path = current_exe.to_str().unwrap();

    let bin_resp_bytes = get(bin_url)
        .map_err(|e| format!("Could not get binary: {e}"))?
        .bytes()
        .map_err(|e| format!("Could not get bytes: {e}"))?;

    fs::rename(exe_path, format!("{exe_path}.old"))
        .map_err(|e| format!("Could not rename existing binary: {e}"))?;

    match fs::write(exe_path, bin_resp_bytes) {
        Ok(_) => {}
        Err(_) => {
            fs::rename(format!("{exe_path}.old"), exe_path)
                .map_err(|e| format!("Could not recover old binary: {e}"))?;

            return Err(format!("Could not write new binary to {exe_path}"));
        }
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let metadata =
            fs::metadata(exe_path).map_err(|e| format!("Could not get exe path meta data: {e}"))?;
        let mut permissions = metadata.permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(exe_path, permissions)
            .map_err(|e| format!("Could not set file permissions: {e}"))?;
    }
    Ok(())
}
