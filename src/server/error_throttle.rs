use crate::common::logging::{debug, error};
use std::time::{Duration, Instant};

/// Minimum gap between two `error`-level lines. Suppressed occurrences are still logged at debug
/// and folded into the next line.
const LOG_INTERVAL: Duration = Duration::from_secs(5);

/// Throttles how often per-packet failures are logged at error level. Every failure the server
/// sees (garbage, wrong-size, rate-limited, unknown-key, replayed, ...) is attacker-triggerable, so
/// logging each one at error level would let a flood spam the journal.
#[derive(Debug, Default)]
pub(crate) struct ErrorThrottle {
    last_log: Option<Instant>,
    suppressed: u64,
}

impl ErrorThrottle {
    /// Always logs `e` at debug; only surfaces an error line once per `LOG_INTERVAL`, folding in
    /// how many were suppressed since the last one.
    pub(crate) fn log(&mut self, e: &anyhow::Error) {
        debug(format!("{e}"));

        let now = Instant::now();
        let due = match self.last_log {
            Some(last) => now.saturating_duration_since(last) >= LOG_INTERVAL,
            None => true,
        };
        if !due {
            self.suppressed += 1;
            return;
        }

        if self.suppressed > 0 {
            error(format!(
                "{e} ({} more suppressed in the last {LOG_INTERVAL:?})",
                self.suppressed
            ));
        } else {
            error(format!("{e}"));
        }
        self.last_log = Some(now);
        self.suppressed = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::{ErrorThrottle, LOG_INTERVAL};
    use std::time::Instant;

    #[test]
    fn test_first_error_logs_immediately() {
        let mut throttle = ErrorThrottle::default();
        throttle.log(&anyhow::anyhow!("boom"));
        assert_eq!(throttle.suppressed, 0);
        assert!(throttle.last_log.is_some());
    }

    #[test]
    fn test_errors_within_interval_are_suppressed() {
        let mut throttle = ErrorThrottle::default();
        let err = anyhow::anyhow!("boom");

        throttle.log(&err);
        throttle.log(&err);
        throttle.log(&err);

        assert_eq!(throttle.suppressed, 2);
    }

    #[test]
    fn test_count_resets_once_interval_elapses() {
        let mut throttle = ErrorThrottle::default();
        let err = anyhow::anyhow!("boom");

        throttle.log(&err);
        throttle.log(&err);
        throttle.last_log = Instant::now().checked_sub(LOG_INTERVAL);

        throttle.log(&err);

        assert_eq!(throttle.suppressed, 0);
    }
}
