use openssl::hash::{Hasher, MessageDigest};
use openssl::rsa::Padding;
use std::net::UdpSocket;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use std::{env, fs};

pub const RSA_PADDING: Padding = Padding::PKCS1;
pub const PADDING_SIZE: usize = 11; // see https://www.rfc-editor.org/rfc/rfc3447#section-7.2.1
pub const SHA256_DIGEST_LENGTH: usize = 32;
pub const NTP_SYSTEM: &str = "system";

pub fn time_from_ntp(ntp_server: &str) -> Result<u128, String> {
    if ntp_server == NTP_SYSTEM {
        return time();
    }

    let socket = UdpSocket::bind("0.0.0.0:0")
        .map_err(|e| format!("Could not create UDP socket to connect to {ntp_server}: {e}"))?;

    socket
        .set_read_timeout(Some(Duration::from_secs(2)))
        .map_err(|e| format!("Could not set UDP socket read timeout: {e}"))?;

    let time = sntpc::simple_get_time(ntp_server, &socket)
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
    println!("[{} \x1b[32mINFO\x1b[0m ] {}", date_time, msg)
}

pub fn error(msg: &str) {
    let date_time = get_date_time();
    println!("[{} \x1b[31mERROR\x1b[0m ] {}", date_time, msg)
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

    format!("{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z", year, month, day, hours, minutes, seconds)
}
