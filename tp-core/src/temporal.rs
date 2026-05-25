//! Temporal utilities for timezone handling

use crate::errors::ProjectionError;
use chrono::{DateTime, FixedOffset, Local, NaiveDateTime, TimeZone};

/// Parse a timestamp accepting either RFC3339 (with timezone) or a naive
/// ISO 8601 datetime without timezone. Naive datetimes are interpreted as
/// the host's local timezone and returned with that offset attached, so
/// downstream code always works on `DateTime<FixedOffset>` with explicit
/// timezone information.
pub fn parse_timestamp_flexible(s: &str) -> Result<DateTime<FixedOffset>, ProjectionError> {
    parse_timestamp_flexible_str(s).map_err(ProjectionError::InvalidTimestamp)
}

/// Same as [`parse_timestamp_flexible`] but returns the raw error string so
/// callers using their own error types (e.g. `DetectionError`) can wrap it.
pub fn parse_timestamp_flexible_str(s: &str) -> Result<DateTime<FixedOffset>, String> {
    if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
        return Ok(dt);
    }
    let naive = NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S%.f")
        .or_else(|_| NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S"))
        .or_else(|_| NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S%.f"))
        .or_else(|_| NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S"))
        .map_err(|e| {
            format!(
                "{} (expected RFC3339 with timezone, e.g. 2025-12-09T14:30:00+01:00, \
                 or ISO 8601 without timezone interpreted as local time)",
                e
            )
        })?;
    Local
        .from_local_datetime(&naive)
        .single()
        .map(|dt| dt.fixed_offset())
        .ok_or_else(|| format!("ambiguous or non-existent local time: '{}'", s))
}

/// Parse RFC3339 timestamp with strict timezone validation.
///
/// Kept for callers that explicitly require timezone-bearing input. Prefer
/// [`parse_timestamp_flexible`] for user-facing inputs (CSV/GeoJSON files,
/// detection feeds, etc.) where a naive datetime should be accepted.
pub fn parse_rfc3339_with_timezone(s: &str) -> Result<DateTime<FixedOffset>, ProjectionError> {
    DateTime::parse_from_rfc3339(s)
        .map_err(|e| ProjectionError::MissingTimezone(format!("Invalid timestamp: {}", e)))
}

/// Validate that timezone information is present
pub fn validate_timezone_present(_dt: &DateTime<FixedOffset>) -> Result<(), ProjectionError> {
    // DateTime<FixedOffset> always has timezone, this is a type-level guarantee
    // This function exists for API consistency
    Ok(())
}

#[cfg(test)]
mod tests;
