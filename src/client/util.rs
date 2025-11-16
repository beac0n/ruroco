use std::fs;
use std::os::unix::fs::PermissionsExt;

pub fn set_permissions(path: &str, permissions_mode: u32) -> Result<(), String> {
    let metadata =
        fs::metadata(path).map_err(|e| format!("Could not get {path:?} meta data: {e}"))?;
    let mut permissions = metadata.permissions();
    permissions.set_mode(permissions_mode);
    fs::set_permissions(path, permissions)
        .map_err(|e| format!("Could not set file permissions for {path:?}: {e}"))
}
