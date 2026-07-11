use crate::client::update::Updater;
use crate::common::crypto::verify_ed25519;
use crate::common::fs::write_atomic_with_mode;
use crate::common::{change_file_ownership, info};
use anyhow::{anyhow, bail, Context};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use tempfile::NamedTempFile;

/// Hard ceiling on a single downloaded asset (binary or signature). Release binaries are a few
/// MB; this only guards against a misbehaving or compromised server making the client allocate
/// unbounded memory.
const MAX_DOWNLOAD_BYTES: u64 = 100 * 1024 * 1024;

impl Updater {
    pub(super) fn check_if_writable(path: &Path) -> anyhow::Result<bool> {
        Ok(NamedTempFile::new_in(path).is_ok())
    }

    fn download_bytes(url: &str) -> anyhow::Result<Vec<u8>> {
        let reader =
            ureq::get(url).call().map_err(|e| anyhow!("Could not get binary: {e}"))?.into_reader();
        let mut bytes = Vec::new();
        reader
            .take(MAX_DOWNLOAD_BYTES + 1)
            .read_to_end(&mut bytes)
            .with_context(|| "Could not get bytes")?;
        if bytes.len() as u64 > MAX_DOWNLOAD_BYTES {
            bail!("Download from {url} exceeded the {MAX_DOWNLOAD_BYTES} byte limit");
        }
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

    /// Download and Ed25519-verify one binary, without touching disk. Used so all targets of an
    /// update can be fetched and verified before any of them is swapped in.
    pub(super) fn download_and_verify_bin(
        &self,
        bin_url: String,
        sig_url: String,
        bin_name: &str,
    ) -> anyhow::Result<Vec<u8>> {
        info(format!("downloading from {bin_url}"));

        let bin_resp_bytes = Self::download_bytes(&bin_url)?;
        let sig_bytes = Self::download_bytes(&sig_url)?;

        verify_ed25519(&self.public_key_pem, &bin_resp_bytes, &sig_bytes)
            .with_context(|| format!("Signature verification failed for {bin_name}"))?;

        Ok(bin_resp_bytes)
    }

    /// Swap an already-verified binary onto disk. Called only after every target of an update has
    /// been downloaded and verified, so a failure partway through downloading never leaves some
    /// binaries updated and others not.
    pub(super) fn save_bin(
        &self,
        bin_bytes: &[u8],
        bin_name: &str,
        permissions_mode: u32,
        user_and_group: Option<&str>,
    ) -> anyhow::Result<()> {
        let target_bin_path = self.bin_path.join(bin_name);

        // Snapshot the current binary to a `.old` sibling for manual rollback. This is done
        // before the swap while the target is still present, so a crash here never removes it.
        if target_bin_path.exists() {
            let backup_path = Self::old_backup_path(&target_bin_path);
            fs::copy(&target_bin_path, &backup_path)
                .with_context(|| format!("Could not back up existing binary to {backup_path:?}"))?;
        }

        // Write the new binary (with exec bits) to a temp file in the same directory, then a
        // single atomic rename over the target. Renaming a running binary is fine on Linux, and
        // the target always holds either the old or the new complete binary, never nothing.
        write_atomic_with_mode(&target_bin_path, bin_bytes, Some(permissions_mode))
            .with_context(|| format!("Could not write new binary to {target_bin_path:?}"))?;

        if let Some(ug) = user_and_group {
            change_file_ownership(&target_bin_path, ug, ug)?
        }
        Ok(())
    }

    /// Undo `save_bin` for `bin_name`, restoring the pre-update state: if a `.old` backup exists
    /// (the target existed before this update), restore it; otherwise the target didn't exist
    /// before, so remove the partially-applied new binary. Safe to call on a target whose
    /// `save_bin` itself failed (e.g. the write succeeded but the chown didn't).
    pub(super) fn rollback_bin(&self, bin_name: &str) -> anyhow::Result<()> {
        let target_bin_path = self.bin_path.join(bin_name);
        let backup_path = Self::old_backup_path(&target_bin_path);

        if backup_path.exists() {
            fs::rename(&backup_path, &target_bin_path).with_context(|| {
                format!("Could not restore {target_bin_path:?} from {backup_path:?}")
            })?;
        } else if target_bin_path.exists() {
            fs::remove_file(&target_bin_path).with_context(|| {
                format!("Could not remove partially-applied {target_bin_path:?}")
            })?;
        }
        Ok(())
    }

    fn old_backup_path(target: &Path) -> PathBuf {
        let mut os = target.as_os_str().to_owned();
        os.push(".old");
        PathBuf::from(os)
    }
}
