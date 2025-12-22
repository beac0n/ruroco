use anyhow::Context;
use std::fs;
use std::os::unix::fs::PermissionsExt;

pub(crate) fn set_permissions(path: &str, permissions_mode: u32) -> anyhow::Result<()> {
    let metadata =
        fs::metadata(path).with_context(|| format!("Could not get {path:?} meta data"))?;
    let mut permissions = metadata.permissions();
    permissions.set_mode(permissions_mode);
    fs::set_permissions(path, permissions)
        .with_context(|| format!("Could not set file permissions for {path:?}"))
}
