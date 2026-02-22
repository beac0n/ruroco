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

#[cfg(test)]
mod tests {
    use super::set_permissions;
    use std::fs;
    use std::os::unix::fs::PermissionsExt;

    #[test]
    fn test_set_permissions() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("test_file");
        fs::write(&file_path, "test").unwrap();
        let path_str = file_path.to_str().unwrap();

        set_permissions(path_str, 0o644).unwrap();
        let mode = fs::metadata(&file_path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o644);

        set_permissions(path_str, 0o600).unwrap();
        let mode = fs::metadata(&file_path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
    }

    #[test]
    fn test_set_permissions_nonexistent_file() {
        let result = set_permissions("/tmp/nonexistent_ruroco_test_file", 0o644);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Could not get"));
    }
}
