use crate::common::logging::error;
#[cfg(any(feature = "with-client", feature = "with-server"))]
use crate::common::now_nanos;
use anyhow::{anyhow, Context};
#[cfg(any(feature = "with-client", feature = "with-server"))]
use std::io::Write;
use std::os::unix::fs::chown;
use std::path::{Path, PathBuf};
use std::{env, fs};

#[cfg(any(feature = "with-client", feature = "with-server"))]
pub(crate) fn write_atomic(path: &Path, contents: &[u8]) -> anyhow::Result<()> {
    let mut tmp_os = path.as_os_str().to_owned();
    tmp_os.push(format!(".{}.tmp", now_nanos()?));
    let tmp_path = PathBuf::from(tmp_os);

    {
        let mut f = fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&tmp_path)
            .with_context(|| format!("open {}", tmp_path.display()))?;

        f.write_all(contents).with_context(|| format!("write tmp {}", tmp_path.display()))?;
        f.sync_all().with_context(|| format!("fsync tmp {}", tmp_path.display()))?;
    }

    fs::rename(&tmp_path, path)
        .with_context(|| format!("rename {} -> {}", tmp_path.display(), path.display()))?;

    // Best-effort: fsync the parent directory so the rename itself is durable.
    if let Some(parent) = path.parent() {
        if let Ok(dir) = fs::File::open(parent) {
            let _ = dir.sync_all();
        }
    }
    Ok(())
}

pub(crate) fn resolve_path(path: &Path) -> PathBuf {
    if path.is_absolute() {
        return path.to_path_buf();
    }

    let mut full_path = match env::current_dir() {
        Ok(p) => p,
        Err(e) => {
            error(format!("Could not get current directory: {e}"));
            return path.to_path_buf();
        }
    };
    full_path.push(path);
    match fs::canonicalize(&full_path) {
        Ok(p) => p,
        Err(e) => {
            error(format!("Could not canonicalize {:?}: {e}", &full_path));
            full_path
        }
    }
}

pub(crate) fn change_file_ownership(
    path: &Path,
    user_name: &str,
    group_name: &str,
) -> anyhow::Result<()> {
    let user_id = if user_name.is_empty() {
        None
    } else {
        Some(get_uid_by_name(user_name)?)
    };

    let group_id = if group_name.is_empty() {
        None
    } else {
        Some(get_gid_by_name(group_name)?)
    };

    chown(path, user_id, group_id).with_context(|| {
        format!("Could not change ownership of {path:?} to {user_id:?}:{group_id:?}")
    })?;
    Ok(())
}

fn get_uid_by_name(name: &str) -> anyhow::Result<u32> {
    let user = nix::unistd::User::from_name(name)
        .with_context(|| format!("Could not find user {name}"))?
        .ok_or_else(|| anyhow!("Could not find user {name}"))?;
    Ok(user.uid.as_raw())
}

fn get_gid_by_name(name: &str) -> anyhow::Result<u32> {
    let group = nix::unistd::Group::from_name(name)
        .with_context(|| format!("Could not find group {name}"))?
        .ok_or_else(|| anyhow!("Could not find group {name}"))?;
    Ok(group.gid.as_raw())
}

#[cfg(test)]
mod tests {
    use crate::common::fs::write_atomic;
    use crate::common::fs::{
        change_file_ownership, get_gid_by_name, get_uid_by_name, resolve_path,
    };
    use std::path::PathBuf;
    use std::{env, fs};

    fn create_temp_file() -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test_file");
        fs::File::create(&path).unwrap();
        (dir, path)
    }

    #[test]
    fn test_resolve_absolute_path() {
        assert_eq!(resolve_path(&PathBuf::from("/foo/bar/baz")), PathBuf::from("/foo/bar/baz"));
    }

    #[test]
    fn test_resolve_relative_path() {
        let _ = fs::create_dir_all(PathBuf::from("./tmp/foo"));
        assert_eq!(
            resolve_path(&PathBuf::from("./tmp/foo")),
            env::current_dir().unwrap().join("tmp/foo")
        );
    }

    #[test]
    fn test_resolve_path_nonexistent_relative() {
        let result = resolve_path(&PathBuf::from("./does_not_exist_ruroco_test"));
        assert!(result.ends_with("does_not_exist_ruroco_test"));
    }

    #[test]
    fn test_get_uid_by_name_root() {
        assert_eq!(get_uid_by_name("root").unwrap(), 0);
    }

    #[test]
    fn test_get_gid_by_name_root() {
        assert_eq!(get_gid_by_name("root").unwrap(), 0);
    }

    #[test]
    fn test_get_uid_by_name_unknown() {
        assert!(get_uid_by_name("barfoobaz")
            .unwrap_err()
            .to_string()
            .contains("Could not find user"));
    }

    #[test]
    fn test_get_gid_by_name_unknown() {
        assert!(get_gid_by_name("barfoobaz")
            .unwrap_err()
            .to_string()
            .contains("Could not find group"));
    }

    #[test]
    fn test_change_file_ownership_empty_user_and_group() {
        let (_dir, path) = create_temp_file();
        assert!(change_file_ownership(&path, "", "").is_ok());
    }

    #[test]
    fn test_change_file_ownership_invalid_user() {
        let (_dir, path) = create_temp_file();
        assert!(change_file_ownership(&path, "nonexistent_ruroco_user_xyz", "")
            .unwrap_err()
            .to_string()
            .contains("Could not find user"));
    }

    #[test]
    fn test_change_file_ownership_invalid_group() {
        let (_dir, path) = create_temp_file();
        assert!(change_file_ownership(&path, "", "nonexistent_ruroco_group_xyz")
            .unwrap_err()
            .to_string()
            .contains("Could not find group"));
    }

    #[test]
    fn test_change_file_ownership_current_user() {
        let (_dir, path) = create_temp_file();
        assert!(change_file_ownership(&path, "", "").is_ok());
        // "root" always exists — chown may fail with permission denied but not "Could not find"
        if let Err(e) = change_file_ownership(&path, "root", "root") {
            assert!(e.to_string().contains("Could not change ownership"), "unexpected: {e}");
        }
    }

    #[test]
    fn test_write_atomic_creates_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("atomic_test");
        write_atomic(&path, b"hello atomic").unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"hello atomic");
    }

    #[test]
    fn test_write_atomic_overwrites_existing() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("atomic_test");
        fs::write(&path, b"old content").unwrap();
        write_atomic(&path, b"new content").unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"new content");
    }

    #[test]
    fn test_write_atomic_fails_on_nonexistent_parent() {
        let path = PathBuf::from("/nonexistent_ruroco_dir_xyz/file.txt");
        let result = write_atomic(&path, b"content");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("open "));
    }

    #[test]
    fn test_change_file_ownership_nonexistent_path() {
        assert!(change_file_ownership(
            &PathBuf::from("/tmp/no_such_file_ruroco_xyz"),
            "root",
            "root"
        )
        .unwrap_err()
        .to_string()
        .contains("Could not change ownership"));
    }
}
