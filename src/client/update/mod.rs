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

        if !self.force && current_version == api_data.tag_name {
            info(format!("Already using version {current_version}"));
            return Ok(());
        }

        // Refuse to downgrade on an implicit "update to latest": an explicit --version always
        // does exactly what was asked (including rolling back), since that is a deliberate choice
        // the caller made, not something the tool should second-guess.
        if !self.force && self.version.is_none() {
            match Self::is_downgrade(&current_version, &api_data.tag_name) {
                Some(true) => bail!(
                    "Refusing to downgrade from {current_version} to {}; pass --version {} \
                     explicitly to roll back",
                    api_data.tag_name,
                    api_data.tag_name
                ),
                Some(false) => {}
                None => info(format!(
                    "Could not compare versions {current_version} and {}; proceeding",
                    api_data.tag_name
                )),
            }
        }

        let assets = &api_data.assets;
        let tag = &api_data.tag_name;

        for (prefix, target_name, mode, owner) in self.binary_targets() {
            let bin_name = format!("{prefix}-{tag}-{ARCH}-{OS}");
            self.download_and_save_bin(
                Self::get_download_url(assets, &bin_name)?,
                Self::get_download_url(assets, &format!("{bin_name}.sig"))?,
                target_name,
                mode,
                owner,
            )?;
        }

        Ok(())
    }

    /// The (asset name prefix, target binary name, file mode, chown user) tuples to download for
    /// this update, depending on whether `--server` was passed.
    fn binary_targets(&self) -> Vec<(&'static str, &'static str, u32, Option<&'static str>)> {
        if self.server {
            vec![
                ("commander", COMMANDER_BIN_NAME, 0o100, None), // execute for owner
                ("server", SERVER_BIN_NAME, 0o500, Some("ruroco")), // read|execute for owner
            ]
        } else {
            // read|write|execute for owner, read|execute for group and others.
            vec![
                ("client", CLIENT_BIN_NAME, 0o755, None),
                ("client-ui", CLIENT_UI_BIN_NAME, 0o755, None),
            ]
        }
    }

    fn get_download_url(assets: &[GithubApiAsset], bin_name: &str) -> anyhow::Result<String> {
        assets
            .iter()
            .find_map(|a| (a.name == bin_name).then(|| a.browser_download_url.clone()))
            .ok_or_else(|| anyhow::anyhow!("Could not find {bin_name}"))
    }

    /// Compares two `vMAJOR.MINOR.PATCH` tags. Returns `Some(true)` if `target` is older than
    /// `current`, `Some(false)` if it is equal or newer, `None` if either tag doesn't parse (in
    /// which case the caller should not block the update on an assumption it cannot verify).
    fn is_downgrade(current: &str, target: &str) -> Option<bool> {
        fn parse(tag: &str) -> Option<(u64, u64, u64)> {
            let mut parts = tag.trim_start_matches('v').splitn(3, '.');
            let major = parts.next()?.parse().ok()?;
            let minor = parts.next()?.parse().ok()?;
            let patch = parts.next()?.parse().ok()?;
            Some((major, minor, patch))
        }
        Some(parse(target)? < parse(current)?)
    }
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod tests;
