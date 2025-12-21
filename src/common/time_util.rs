use std::time::{Duration, SystemTime, UNIX_EPOCH};

const SECONDS_PER_HOUR: u64 = 3600;

const SECONDS_PER_DAY: u64 = 86400;

const MONTHS: [u64; 12] = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];

pub(crate) struct TimeUtil {}

impl TimeUtil {
    pub(crate) fn time() -> Result<u128, String> {
        let duration = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map_err(|e| format!("Could not get duration since: {e}"))?;
        Ok(duration.as_nanos())
    }
    pub(crate) fn get_date_time() -> String {
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
