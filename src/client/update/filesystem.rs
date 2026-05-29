use crate::client::update::Updater;
use crate::client::util::set_permissions;
use crate::common::crypto::verify_ed25519;
use crate::common::{change_file_ownership, info};
use anyhow::{anyhow, bail, Context};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use tempfile::NamedTempFile;

impl Updater {
    pub(super) fn check_if_writable(path: &Path) -> anyhow::Result<bool> {
        Ok(NamedTempFile::new_in(path).is_ok())
    }

    fn download_bytes(url: &str) -> anyhow::Result<Vec<u8>> {
        let mut reader =
            ureq::get(url).call().map_err(|e| anyhow!("Could not get binary: {e}"))?.into_reader();
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).with_context(|| "Could not get bytes")?;
        Ok(bytes)
    }

    pub(super) fn validate_dir_path(dir_path: PathBuf) -> anyhow::Result<PathBuf> {
        match dir_path {
            p if !p.exists() => {
                fs::create_dir_all(&p).with_context(|| "Could not create .bin directory")?;
                Ok(p)
            }
            p if !p.is_dir() => Err(anyhow!("{p:?} exists but is not a directory")),
            p if !Self::check_if_writable(&p)? => Err(anyhow!("can't write to {p:?}")),
            p => Ok(p),
        }
    }

    pub(super) fn download_and_save_bin(
        &self,
        bin_url: String,
        sig_url: String,
        bin_name: &str,
        permissions_mode: u32,
        user_and_group: Option<&str>,
    ) -> anyhow::Result<()> {
        info(format!("downloading from {bin_url}"));

        let target_bin_path = &self.bin_path.join(bin_name);

        let bin_resp_bytes = Self::download_bytes(&bin_url)?;
        let sig_bytes = Self::download_bytes(&sig_url)?;

        verify_ed25519(&self.public_key_pem, &bin_resp_bytes, &sig_bytes)
            .with_context(|| format!("Signature verification failed for {bin_name}"))?;

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
