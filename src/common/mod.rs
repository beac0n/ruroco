#[cfg(target_os = "android")]
pub mod android_util;
pub(crate) mod client_data;
pub(crate) mod crypto_handler;
pub(crate) mod data_parser;
pub(crate) mod serialization_util;

use anyhow::{anyhow, Context};
use blake2::digest::{Update, VariableOutput};
use blake2::Blake2bVar;
use chrono::Utc;
use openssl::rand::rand_bytes;
use std::os::unix::fs::chown;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::{env, fs};

pub(crate) fn blake2b_u64(s: &str) -> anyhow::Result<u64> {
    let mut hasher = Blake2bVar::new(8)
        .with_context(|| format!("Could not create Blake2b hasher for string {s}"))?;
    hasher.update(s.as_bytes());
    let mut out = [0u8; 8];
    hasher
        .finalize_variable(&mut out)
        .with_context(|| format!("Could not finalize Blake2b hash for string {s}"))?;
    Ok(u64::from_be_bytes(out))
}

pub fn get_random_string(len: usize) -> anyhow::Result<String> {
    let mut buf = vec![0u8; len];
    rand_bytes(&mut buf).with_context(|| "Could not generate random")?;
    let chars = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
    Ok(buf.iter().map(|b| chars.as_bytes()[(*b as usize) % chars.len()] as char).collect())
}

pub fn get_random_range(from: u16, to: u16) -> anyhow::Result<u16> {
    let mut buf = [0u8; 2];
    rand_bytes(&mut buf).with_context(|| "Could not generate number")?;

    let span = to - from;
    let v = u16::from_be_bytes(buf) % span;
    Ok(from + v)
}

pub(crate) fn resolve_path(path: &Path) -> PathBuf {
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

pub(crate) fn info(msg: &str) {
    let date_time = get_date_time();
    println!("[{date_time} \x1b[32mINFO\x1b[0m ] {msg}")
}

pub(crate) fn error(msg: impl std::fmt::Display) {
    let date_time = get_date_time();
    println!("[{date_time} \x1b[31mERROR\x1b[0m ] {msg}")
}

pub(crate) fn change_file_ownership(
    path: &Path,
    user_name: &str,
    group_name: &str,
) -> anyhow::Result<()> {
    let user_id = match get_id_by_name_and_flag(user_name, "-u") {
        Some(id) => Some(id),
        None if user_name.is_empty() => None,
        None => return Err(anyhow!("Could not find user {user_name}")),
    };

    let group_id = match get_id_by_name_and_flag(group_name, "-g") {
        Some(id) => Some(id),
        None if group_name.is_empty() => None,
        None => return Err(anyhow!("Could not find group {group_name}")),
    };

    chown(path, user_id, group_id).with_context(|| {
        format!("Could not change ownership of {path:?} to {user_id:?}:{group_id:?}")
    })?;
    Ok(())
}

fn get_date_time() -> String {
    Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string()
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
