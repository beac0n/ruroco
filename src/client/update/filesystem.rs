use crate::client::update::Updater;
use crate::common::crypto::verify_ed25519;
use crate::common::fs::write_atomic_with_mode;
use crate::common::logging::error;
use crate::common::{change_file_ownership, info};
use anyhow::{anyhow, bail, Context};
use std::fs;
use std::io::Read;
use std::os::unix::fs::PermissionsExt;
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
        let reader = crate::client::update::http_agent()
            .get(url)
            .call()
            .map_err(|e| anyhow!("Could not get binary: {e}"))?
            .into_reader();
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

        if target_bin_path.exists() {
            Self::backup_bin(&target_bin_path)?;
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

    /// Undo `save_bin` for `bin_name`. `existed_before` must be sampled *before* `save_bin` ran:
    /// whether the target predates this update cannot be inferred from the presence of a backup,
    /// and guessing wrong deletes a working binary that was never touched.
    pub(super) fn rollback_bin(
        &self,
        bin_name: &str,
        user_and_group: Option<&str>,
        existed_before: bool,
    ) -> anyhow::Result<()> {
        let target_bin_path = self.bin_path.join(bin_name);
        let backup_path = Self::old_backup_path(&target_bin_path);

        if backup_path.exists() {
            fs::rename(&backup_path, &target_bin_path).with_context(|| {
                format!("Could not restore {target_bin_path:?} from {backup_path:?}")
            })?;
            // The backup was written by this process, so it is root-owned rather than carrying
            // the original's owner: without this the restored ruroco-server is root:root 0o500
            // and `User=ruroco` can no longer execute it.
            if let Some(ug) = user_and_group {
                change_file_ownership(&target_bin_path, ug, ug)?
            }
        } else if !existed_before && target_bin_path.exists() {
            fs::remove_file(&target_bin_path).with_context(|| {
                format!("Could not remove partially-applied {target_bin_path:?}")
            })?;
        }
        // Remaining case: the target predates this update but there is no backup, so `save_bin`
        // failed at or before the backup step and the original is still in place, untouched.
        Ok(())
    }

    /// Snapshot the current binary to a `.old` sibling, atomically: a half-written backup must
    /// never exist, or a later rollback would restore a truncated binary over an intact one. Any
    /// backup left by a crashed earlier update is dropped first, so the presence of a `.old`
    /// always means "this run made it".
    fn backup_bin(target: &Path) -> anyhow::Result<()> {
        let backup_path = Self::old_backup_path(target);
        let _ = fs::remove_file(&backup_path);

        let mode = fs::metadata(target)
            .with_context(|| format!("Could not stat {target:?}"))?
            .permissions()
            .mode();
        let bytes =
            fs::read(target).with_context(|| format!("Could not read {target:?} to back it up"))?;
        write_atomic_with_mode(&backup_path, &bytes, Some(mode))
            .with_context(|| format!("Could not back up existing binary to {backup_path:?}"))
    }

    /// Drop the rollback snapshot once every target has swapped successfully. Never fatal: the
    /// update already succeeded, and a leftover `.old` must not fail it. Leaving them around also
    /// feeds `rollback_bin` a stale backup on some *later* update.
    pub(super) fn remove_backup(&self, bin_name: &str) {
        let backup_path = Self::old_backup_path(&self.bin_path.join(bin_name));
        match fs::remove_file(&backup_path) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => error(format!("Could not remove backup {backup_path:?}: {e}")),
        }
    }

    fn old_backup_path(target: &Path) -> PathBuf {
        let mut os = target.as_os_str().to_owned();
        os.push(".old");
        PathBuf::from(os)
    }
}
