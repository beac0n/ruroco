pub mod client_data;
pub mod crypto_handler;
pub mod data_parser;
pub mod time_util;

use crate::common::time_util::TimeUtil;
use std::os::unix::fs::chown;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::{env, fs};

pub fn resolve_path(path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        let mut full_path = match env::current_dir() {
            Ok(p) => p,
            Err(e) => {
                error(&format!("Could not get current directory: {e}"));
                return path.to_path_buf();
            }
        };
        full_path.push(path);
        match fs::canonicalize(&full_path) {
            Ok(p) => p,
            Err(e) => {
                error(&format!("Could not canonicalize {:?}: {e}", &full_path));
                full_path
            }
        }
    }
}

pub fn info(msg: &str) {
    let date_time = TimeUtil::get_date_time();
    println!("[{date_time} \x1b[32mINFO\x1b[0m ] {msg}")
}

pub fn error(msg: &str) {
    let date_time = TimeUtil::get_date_time();
    println!("[{date_time} \x1b[31mERROR\x1b[0m ] {msg}")
}

pub fn change_file_ownership(path: &Path, user_name: &str, group_name: &str) -> Result<(), String> {
    let user_id = match get_id_by_name_and_flag(user_name, "-u") {
        Some(id) => Some(id),
        None if user_name.is_empty() => None,
        None => return Err(format!("Could not find user {user_name}")),
    };

    let group_id = match get_id_by_name_and_flag(group_name, "-g") {
        Some(id) => Some(id),
        None if group_name.is_empty() => None,
        None => return Err(format!("Could not find group {group_name}")),
    };

    chown(path, user_id, group_id).map_err(|e| {
        format!("Could not change ownership of {path:?} to {user_id:?}:{group_id:?}: {e}")
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
                error(&format!(
                    "Error parsing id from id command output: {} {} {e}",
                    String::from_utf8_lossy(&output.stdout),
                    String::from_utf8_lossy(&output.stderr)
                ));
                None
            }
        },
        Err(e) => {
            error(&format!("Error getting id via id command: {e}"));
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::common::{get_id_by_name_and_flag, resolve_path};
    use std::path::PathBuf;
    use std::{env, fs};

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
    fn test_get_id_by_name_and_flag() {
        assert_eq!(get_id_by_name_and_flag("root", "-u"), Some(0));
        assert_eq!(get_id_by_name_and_flag("root", "-g"), Some(0));
    }

    #[test]
    fn test_get_id_by_name_and_flag_unknown_user() {
        assert_eq!(get_id_by_name_and_flag("barfoobaz", "-u"), None);
        assert_eq!(get_id_by_name_and_flag("barfoobaz", "-g"), None);
    }
}
