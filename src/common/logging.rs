use chrono::Utc;
use std::io::{IsTerminal, Write};
use std::sync::OnceLock;

pub(crate) fn info(msg: impl std::fmt::Display) {
    let line =
        format!("[{} {} ] {msg}", get_date_time(), colorize("INFO", "32", stdout_is_terminal()));
    let _ = writeln!(std::io::stdout().lock(), "{line}");
}

pub(crate) fn debug(msg: impl std::fmt::Display) {
    if debug_enabled() {
        let line = format!(
            "[{} {} ] {msg}",
            get_date_time(),
            colorize("DEBUG", "36", stdout_is_terminal())
        );
        let _ = writeln!(std::io::stdout().lock(), "{line}");
    }
}

pub(crate) fn error(msg: impl std::fmt::Display) {
    let line =
        format!("[{} {} ] {msg}", get_date_time(), colorize("ERROR", "31", stderr_is_terminal()));
    let _ = writeln!(std::io::stderr().lock(), "{line}");
}

fn colorize(label: &str, ansi_code: &str, use_color: bool) -> String {
    if use_color {
        format!("\x1b[{ansi_code}m{label}\x1b[0m")
    } else {
        label.to_string()
    }
}

fn stdout_is_terminal() -> bool {
    static STDOUT_IS_TERMINAL: OnceLock<bool> = OnceLock::new();
    *STDOUT_IS_TERMINAL.get_or_init(|| std::io::stdout().is_terminal())
}

fn stderr_is_terminal() -> bool {
    static STDERR_IS_TERMINAL: OnceLock<bool> = OnceLock::new();
    *STDERR_IS_TERMINAL.get_or_init(|| std::io::stderr().is_terminal())
}

fn debug_enabled() -> bool {
    static DEBUG_ENABLED: OnceLock<bool> = OnceLock::new();
    *DEBUG_ENABLED.get_or_init(|| is_debug_log_level(std::env::var("RUROCO_LOG").ok()))
}

/// Pure helper so the `RUROCO_LOG` parsing itself is testable without racing on the real env var.
fn is_debug_log_level(value: Option<String>) -> bool {
    value.is_some_and(|v| v.eq_ignore_ascii_case("debug"))
}

fn get_date_time() -> String {
    Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string()
}

#[cfg(test)]
mod tests {
    use super::{colorize, is_debug_log_level};

    #[test]
    fn test_is_debug_log_level() {
        assert!(is_debug_log_level(Some("debug".to_string())));
        assert!(is_debug_log_level(Some("DEBUG".to_string())));
        assert!(is_debug_log_level(Some("Debug".to_string())));
        assert!(!is_debug_log_level(Some("info".to_string())));
        assert!(!is_debug_log_level(Some("".to_string())));
        assert!(!is_debug_log_level(None));
    }

    #[test]
    fn test_colorize() {
        assert_eq!(colorize("INFO", "32", true), "\x1b[32mINFO\x1b[0m");
        assert_eq!(colorize("INFO", "32", false), "INFO");
    }

    #[test]
    fn test_get_date_time_has_millisecond_resolution() {
        let formatted = super::get_date_time();
        assert!(formatted.ends_with('Z'), "unexpected format: {formatted}");
        // YYYY-MM-DDTHH:MM:SS.mmmZ
        assert_eq!(formatted.len(), 24, "unexpected format: {formatted}");
    }
}
