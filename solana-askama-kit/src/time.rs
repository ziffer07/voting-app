//! Datetime parsing and formatting helpers for Solana on-chain timestamps.
//!
//! Solana programs store time as Unix timestamps (`i64`). This module provides
//! conversions between the `datetime-local` HTML input format and those
//! timestamps, plus human-readable display formatting.

use chrono::{DateTime, NaiveDateTime, Utc};

/// Parse an HTML `datetime-local` string (`"2025-01-01T10:00"`) into a UTC
/// Unix timestamp.
///
/// # Errors
/// Returns a descriptive string error if parsing fails.
///
/// # Example
/// ```rust
/// use solana_askama_kit::time::parse_datetime_local;
/// let ts = parse_datetime_local("2025-06-15T14:30").unwrap();
/// assert!(ts > 0);
/// ```
pub fn parse_datetime_local(s: &str) -> Result<i64, String> {
    NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M")
        .map(|dt| DateTime::<Utc>::from_naive_utc_and_offset(dt, Utc).timestamp())
        .map_err(|e| format!("Invalid datetime '{}': {}", s, e))
}

/// Format a Unix timestamp as a human-readable UTC string.
///
/// Output format: `"2025-06-15 14:30 UTC"`
///
/// Returns the raw timestamp as a string if it is out of range.
///
/// # Example
/// ```rust
/// use solana_askama_kit::time::format_timestamp;
/// let s = format_timestamp(1_718_459_400);
/// assert!(s.ends_with("UTC"));
/// ```
pub fn format_timestamp(ts: i64) -> String {
    DateTime::from_timestamp(ts, 0)
        .map(|dt| dt.format("%Y-%m-%d %H:%M UTC").to_string())
        .unwrap_or_else(|| ts.to_string())
}

/// Format a Unix timestamp as an ISO 8601 date string (`"2025-06-15"`).
pub fn format_date(ts: i64) -> String {
    DateTime::from_timestamp(ts, 0)
        .map(|dt| dt.format("%Y-%m-%d").to_string())
        .unwrap_or_else(|| ts.to_string())
}

/// Returns `true` if the current wall-clock time is between `start_ts` and
/// `end_ts` (both inclusive, UTC Unix seconds).
pub fn is_active(start_ts: i64, end_ts: i64) -> bool {
    let now = Utc::now().timestamp();
    now >= start_ts && now <= end_ts
}

/// Returns `true` if `end_ts` is in the past (poll/event has closed).
pub fn is_expired(end_ts: i64) -> bool {
    Utc::now().timestamp() > end_ts
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        let ts = parse_datetime_local("2025-06-15T14:30").unwrap();
        let s = format_timestamp(ts);
        assert_eq!(s, "2025-06-15 14:30 UTC");
    }

    #[test]
    fn invalid_datetime() {
        assert!(parse_datetime_local("not-a-date").is_err());
    }

    #[test]
    fn format_date_smoke() {
        let ts = parse_datetime_local("2025-01-01T00:00").unwrap();
        assert_eq!(format_date(ts), "2025-01-01");
    }
}
