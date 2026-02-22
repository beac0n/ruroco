use crate::common::logging::error;
use anyhow::{bail, Context};
use std::os::unix::fs::chown;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::{env, fs};

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
    let user_id = match get_id_by_name_and_flag(user_name, "-u") {
        Some(id) => Some(id),
        None if user_name.is_empty() => None,
        None => bail!("Could not find user {user_name}"),
    };

    let group_id = match get_id_by_name_and_flag(group_name, "-g") {
        Some(id) => Some(id),
        None if group_name.is_empty() => None,
        None => bail!("Could not find group {group_name}"),
    };

    chown(path, user_id, group_id).with_context(|| {
        format!("Could not change ownership of {path:?} to {user_id:?}:{group_id:?}")
    })?;
    Ok(())
}

fn get_id_by_name_and_flag(name: &str, flag: &str) -> Option<u32> {
    if name.is_empty() {
        return None;
    }

    match Command::new("id").arg(flag).arg(name).output() {
        Ok(output) => match String::from_utf8_lossy(&output.stdout).trim().parse::<u32>() {
            Ok(uid) => Some(uid),
            Err(e) => {
                error(format!(
                    "Error parsing id from id command output: {} {} {e}",
                    String::from_utf8_lossy(&output.stdout),
                    String::from_utf8_lossy(&output.stderr)
                ));
                None
            }
        },
        Err(e) => {
            error(format!("Error getting id via id command: {e}"));
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::common::fs::{change_file_ownership, get_id_by_name_and_flag, resolve_path};
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
    fn test_get_id_by_name_and_flag() {
        assert_eq!(get_id_by_name_and_flag("root", "-u"), Some(0));
        assert_eq!(get_id_by_name_and_flag("root", "-g"), Some(0));
    }

    #[test]
    fn test_get_id_by_name_and_flag_unknown_user() {
        assert_eq!(get_id_by_name_and_flag("barfoobaz", "-u"), None);
        assert_eq!(get_id_by_name_and_flag("barfoobaz", "-g"), None);
    }

    #[test]
    fn test_get_id_by_name_and_flag_empty_name() {
        assert_eq!(get_id_by_name_and_flag("", "-u"), None);
        assert_eq!(get_id_by_name_and_flag("", "-g"), None);
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
