use crate::client::update::Updater;
use anyhow::{anyhow, Context};
use serde::{Deserialize, Serialize};

pub(super) const GH_RELEASES_URL: &str = "https://api.github.com/repos/beac0n/ruroco/releases";
pub(super) const SERVER_BIN_DIR: &str = "/usr/local/bin";
pub(super) const COMMANDER_BIN_NAME: &str = "ruroco-commander";
pub(super) const SERVER_BIN_NAME: &str = "ruroco-server";
pub(super) const CLIENT_BIN_NAME: &str = "ruroco-client";
pub(super) const CLIENT_UI_BIN_NAME: &str = "ruroco-client-ui";

/// Ed25519 public key used to verify release binaries during self-update. The matching
/// private key is held only as a CI secret (`RUROCO_SIGNING_KEY`) and signs the binaries
/// at release time. Embedded at build time so a downloaded binary cannot strip it.
pub(super) const RELEASE_PUBLIC_KEY: &[u8] =
    include_bytes!("../../../keys/ruroco-release-ed25519.pub.pem");

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

impl Updater {
    /// Used by the Android update path, which queries the releases API to locate the `.apk`
    /// asset and hands it to the OS installer (APK authenticity is enforced by Android's own
    /// package signing, so no Ed25519 check applies here).
    #[cfg(target_os = "android")]
    pub(crate) fn get_github_api_data(
        version_to_download: Option<&String>,
    ) -> anyhow::Result<GithubApiData> {
        Self::get_github_api_data_from(GH_RELEASES_URL, version_to_download)
    }

    pub(super) fn get_github_api_data_from(
        releases_url: &str,
        version_to_download: Option<&String>,
    ) -> anyhow::Result<GithubApiData> {
        let agent = ureq::AgentBuilder::new().user_agent("rust-client").build();
        let response_data: Vec<GithubApiData> = agent
            .get(releases_url)
            .call()
            .map_err(|e| anyhow!("Could not get API response: {e}"))?
            .into_json()
            .with_context(|| "Could not parse json")?;

        let data = match version_to_download {
            None => response_data.first().cloned(),
            Some(v) => response_data.into_iter().find(|d| d.tag_name == *v),
        };

        match data {
            None => Err(anyhow!("Could not find version {version_to_download:?}")),
            Some(d) => Ok(d),
        }
    }
}
