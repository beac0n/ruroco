use sntpc::{NtpContext, StdTimestampGen};
use std::net::{ToSocketAddrs, UdpSocket};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
pub const NTP_SYSTEM: &str = "system";
const SECONDS_PER_HOUR: u64 = 3600;

const SECONDS_PER_DAY: u64 = 86400;

const MONTHS: [u64; 12] = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];

pub struct TimeUtil {}

impl TimeUtil {
    pub fn time_from_ntp(ntp_server: &str) -> Result<u128, String> {
        if ntp_server == NTP_SYSTEM {
            return Self::time();
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
    pub fn get_date_time() -> String {
        let total_seconds = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::from_secs(0))
            .as_secs();
        let remaining_seconds = total_seconds % SECONDS_PER_DAY;
        let mut days_remaining = total_seconds / SECONDS_PER_DAY;

        let mut year = 1970;
        while days_remaining >= if Self::is_leap_year(year) { 366 } else { 365 } {
            days_remaining -= if Self::is_leap_year(year) { 366 } else { 365 };
            year += 1;
        }

        let mut month = 1;
        while days_remaining >= Self::days_in_month(month, year) {
            days_remaining -= Self::days_in_month(month, year);
            month += 1;
        }
        let day = days_remaining + 1;

        let hours = remaining_seconds / SECONDS_PER_HOUR;
        let minutes = (remaining_seconds % SECONDS_PER_HOUR) / 60;
        let seconds = ((remaining_seconds % SECONDS_PER_HOUR) % 60) + 1;

        format!("{year:04}-{month:02}-{day:02}T{hours:02}:{minutes:02}:{seconds:02}Z")
    }

    fn is_leap_year(year: u64) -> bool {
        (year.is_multiple_of(4) && !year.is_multiple_of(100)) || year.is_multiple_of(400)
    }

    fn days_in_month(month: u64, year: u64) -> u64 {
        if month == 2 && Self::is_leap_year(year) {
            29
        } else {
            MONTHS[(month - 1) as usize]
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::common::time_util::TimeUtil;

    #[test]
    fn test_time_from_ntp_server() {
        let first_time = TimeUtil::time_from_ntp("europe.pool.ntp.org:123").unwrap();
        let second_time = TimeUtil::time_from_ntp("0.europe.pool.ntp.org:123").unwrap();

        let diff = second_time.abs_diff(first_time);

        dbg!(first_time);
        dbg!(second_time);
        dbg!(diff);

        let max_allowed_diff = 500_000_000; // 0.5 seconds
        assert!(diff < max_allowed_diff, "Time difference too large: {diff}");
    }

    #[test]
    fn test_time_from_ntp_system() {
        let start = TimeUtil::time().unwrap();
        let first_time = TimeUtil::time().unwrap();
        let second_time = TimeUtil::time_from_ntp("system").unwrap();
        let diff = second_time - first_time;

        assert!(diff > 0);
        let one_milli_second = 1000000;
        assert!(diff < one_milli_second);
        assert!(first_time > start);
        assert!(second_time > start);
    }
}
