//! Utility functions and helpers.

pub mod languages;
mod range;
pub mod tree;

pub use languages::{get_language, locales_match, LanguageItem, LANGUAGES};
pub use range::{parse_range, RangeSet};
pub use tree::{child_prefix, format_locales, format_subs, print_tree_item};

/// Expand tilde in paths.
pub fn expand_path(path: &str) -> String {
    if path.starts_with('~') {
        if let Some(home) = dirs::home_dir() {
            return path.replacen('~', &home.to_string_lossy(), 1);
        }
    }
    path.to_string()
}

/// Format bytes as human-readable string.
pub fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

/// Format duration as human-readable string.
pub fn format_duration(seconds: u64) -> String {
    let hours = seconds / 3600;
    let minutes = (seconds % 3600) / 60;
    let secs = seconds % 60;

    if hours > 0 {
        format!("{}:{:02}:{:02}", hours, minutes, secs)
    } else {
        format!("{}:{:02}", minutes, secs)
    }
}

/// Format speed as human-readable string.
pub fn format_speed(bytes_per_sec: u64) -> String {
    format!("{}/s", format_bytes(bytes_per_sec))
}

/// Redact sensitive strings for logging.
///
/// Shows first 4 and last 4 characters with "..." in between.
/// For short strings (<= 12 chars), returns all asterisks.
///
/// # Example
/// ```
/// use crunchy_cli::utils::redact;
/// assert_eq!(redact("my-secret-token-12345"), "my-s...2345");
/// assert_eq!(redact("short"), "*****");
/// ```
pub fn redact(s: &str) -> String {
    if s.len() <= 12 {
        return "*".repeat(s.len());
    }
    format!("{}...{}", &s[..4], &s[s.len() - 4..])
}

/// Redact a URL for logging by hiding query parameters.
///
/// Keeps the path but replaces query string values with "[REDACTED]".
pub fn redact_url(url: &str) -> String {
    if let Some(idx) = url.find('?') {
        let path = &url[..idx];
        // Count query params
        let query = &url[idx + 1..];
        let param_count = query.split('&').count();
        format!("{}?[{} params]", path, param_count)
    } else {
        url.to_string()
    }
}

/// Format a duration from std::time::Duration as human-readable.
pub fn format_elapsed(duration: std::time::Duration) -> String {
    let millis = duration.as_millis();
    if millis < 1000 {
        format!("{}ms", millis)
    } else if millis < 60_000 {
        format!("{:.1}s", duration.as_secs_f64())
    } else {
        let secs = duration.as_secs();
        let mins = secs / 60;
        let secs = secs % 60;
        format!("{}m {}s", mins, secs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(500), "500 B");
        assert_eq!(format_bytes(1024), "1.00 KB");
        assert_eq!(format_bytes(1536), "1.50 KB");
        assert_eq!(format_bytes(1048576), "1.00 MB");
        assert_eq!(format_bytes(1073741824), "1.00 GB");
    }

    #[test]
    fn test_format_duration() {
        assert_eq!(format_duration(30), "0:30");
        assert_eq!(format_duration(90), "1:30");
        assert_eq!(format_duration(3661), "1:01:01");
    }

    #[test]
    fn test_redact() {
        // Short strings get fully redacted
        assert_eq!(redact("short"), "*****");
        assert_eq!(redact("12chars12345"), "************");
        // Longer strings show first 4 and last 4
        assert_eq!(redact("my-secret-token-12345"), "my-s...2345");
        assert_eq!(
            redact("eyJhbGciOiJSUzI1NiIsInR5cCI6IkpXVCJ9"),
            "eyJh...VCJ9"
        );
    }

    #[test]
    fn test_redact_url() {
        assert_eq!(
            redact_url("https://api.example.com/path"),
            "https://api.example.com/path"
        );
        assert_eq!(
            redact_url("https://api.example.com/path?token=secret&key=value"),
            "https://api.example.com/path?[2 params]"
        );
    }

    #[test]
    fn test_format_elapsed() {
        use std::time::Duration;
        assert_eq!(format_elapsed(Duration::from_millis(50)), "50ms");
        assert_eq!(format_elapsed(Duration::from_millis(1500)), "1.5s");
        assert_eq!(format_elapsed(Duration::from_secs(90)), "1m 30s");
    }
}
