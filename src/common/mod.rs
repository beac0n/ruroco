pub mod data;

use openssl::hash::{Hasher, MessageDigest};
use openssl::rsa::Padding;
use sntpc::{NtpContext, StdTimestampGen};
use std::net::{ToSocketAddrs, UdpSocket};
use std::os::unix::fs::chown;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use std::{env, fs};

pub const RSA_PADDING: Padding = Padding::PKCS1;
pub const PADDING_SIZE: usize = 11; // see https://www.rfc-editor.org/rfc/rfc3447#section-7.2.1
pub const SHA256_DIGEST_LENGTH: usize = 32;
pub const NTP_SYSTEM: &str = "system";

pub fn set_permissions(path: &str, permissions_mode: u32) -> Result<(), String> {
    let metadata =
        fs::metadata(path).map_err(|e| format!("Could not get {path:?} meta data: {e}"))?;
    let mut permissions = metadata.permissions();
    permissions.set_mode(permissions_mode);
    fs::set_permissions(path, permissions)
        .map_err(|e| format!("Could not set file permissions for {path:?}: {e}"))
}

pub fn time_from_ntp(ntp_server: &str) -> Result<u128, String> {
    if ntp_server == NTP_SYSTEM {
        return time();
    }

    let socket = UdpSocket::bind("0.0.0.0:0")
        .map_err(|e| format!("Could not create UDP socket to connect to {ntp_server}: {e}"))?;

    socket
        .set_read_timeout(Some(Duration::from_secs(2)))
        .map_err(|e| format!("Could not set UDP socket read timeout: {e}"))?;

    let ntp_context = NtpContext::new(StdTimestampGen::default());
    let addr = ntp_server
        .to_socket_addrs()
        .map_err(|e| format!("Could not map socket addrs: {e:?}"))?
        .next()
        .ok_or("Could not resolve NTP server address")?;
    let time = sntpc::sync::get_time(addr, &socket, ntp_context)
        .map_err(|e| format!("Could not get time from NTP Server {ntp_server}: {e:?}"))?;

    let nano_seconds_fraction = sntpc::fraction_to_nanoseconds(time.sec_fraction()) as u128;
    let nano_seconds = (time.sec() as u128) * 1_000_000_000;
    Ok(nano_seconds + nano_seconds_fraction)
}

pub fn time() -> Result<u128, String> {
    let duration = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_err(|e| format!("Could not get duration since: {e}"))?;
    Ok(duration.as_nanos())
}

pub fn hash_public_key(pem_pub_key: Vec<u8>) -> Result<Vec<u8>, String> {
    let digest = MessageDigest::sha256();
    let mut hasher = Hasher::new(digest).map_err(|e| format!("Could not create hasher: {e}"))?;
    hasher.update(pem_pub_key.as_slice()).map_err(|e| format!("Could not update hasher: {e}"))?;
    let hash_bytes = hasher.finish().map_err(|e| format!("Could not finish hasher: {e}"))?;
    Ok(hash_bytes.to_vec())
}

pub fn get_commander_unix_socket_path(config_dir: &Path) -> PathBuf {
    resolve_path(config_dir).join("ruroco.socket")
}

pub fn get_blocklist_path(config_dir: &Path) -> PathBuf {
    resolve_path(config_dir).join("blocklist.toml")
}

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
    let date_time = get_date_time();
    println!("[{date_time} \x1b[32mINFO\x1b[0m ] {msg}")
}

pub fn error(msg: &str) {
    let date_time = get_date_time();
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
const SECONDS_PER_HOUR: u64 = 3600;

const SECONDS_PER_DAY: u64 = 86400;

const MONTHS: [u64; 12] = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];

fn is_leap_year(year: u64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

fn days_in_month(month: u64, year: u64) -> u64 {
    if month == 2 && is_leap_year(year) {
        29
    } else {
        MONTHS[(month - 1) as usize]
    }
}

fn get_date_time() -> String {
    let total_seconds =
        SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or(Duration::from_secs(0)).as_secs();
    let remaining_seconds = total_seconds % SECONDS_PER_DAY;
    let mut days_remaining = total_seconds / SECONDS_PER_DAY;

    let mut year = 1970;
    while days_remaining >= if is_leap_year(year) { 366 } else { 365 } {
        days_remaining -= if is_leap_year(year) { 366 } else { 365 };
        year += 1;
    }

    let mut month = 1;
    while days_remaining >= days_in_month(month, year) {
        days_remaining -= days_in_month(month, year);
        month += 1;
    }
    let day = days_remaining + 1;

    let hours = remaining_seconds / SECONDS_PER_HOUR;
    let minutes = (remaining_seconds % SECONDS_PER_HOUR) / 60;
    let seconds = ((remaining_seconds % SECONDS_PER_HOUR) % 60) + 1;

    format!("{year:04}-{month:02}-{day:02}T{hours:02}:{minutes:02}:{seconds:02}Z")
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
    use crate::common::{
        get_blocklist_path, get_commander_unix_socket_path, get_id_by_name_and_flag, resolve_path,
        time, time_from_ntp,
    };
    use std::path::PathBuf;
    use std::{env, fs};

    #[test]
    fn test_time_from_ntp_server() {
        let first_time = time_from_ntp("europe.pool.ntp.org:123").unwrap();
        let second_time = time_from_ntp("0.europe.pool.ntp.org:123").unwrap();

        let diff = second_time.abs_diff(first_time);

        dbg!(first_time);
        dbg!(second_time);
        dbg!(diff);

        let max_allowed_diff = 500_000_000; // 0.5 seconds
        assert!(diff < max_allowed_diff, "Time difference too large: {diff}");
    }

    #[test]
    fn test_time_from_ntp_system() {
        let start = time().unwrap();
        let first_time = time().unwrap();
        let second_time = time_from_ntp("system").unwrap();
        let diff = second_time - first_time;

        assert!(diff > 0);
        let one_milli_second = 1000000;
        assert!(diff < one_milli_second);
        assert!(first_time > start);
        assert!(second_time > start);
    }

    #[test]
    fn test_get_blocklist_path() {
        assert_eq!(
            get_blocklist_path(&PathBuf::from("/foo/bar/baz")),
            PathBuf::from("/foo/bar/baz/blocklist.toml")
        );
    }

    #[test]
    fn test_get_socket_path() {
        assert_eq!(
            get_commander_unix_socket_path(&PathBuf::from("/foo/bar/baz")),
            PathBuf::from("/foo/bar/baz/ruroco.socket")
        );
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
