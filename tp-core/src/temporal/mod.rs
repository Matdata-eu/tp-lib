//! Temporal utilities for timezone handling

use crate::errors::ProjectionError;
use chrono::{DateTime, FixedOffset};

/// Parse RFC3339 timestamp with timezone validation
pub fn parse_rfc3339_with_timezone(s: &str) -> Result<DateTime<FixedOffset>, ProjectionError> {
    DateTime::parse_from_rfc3339(s)
        .map_err(|e| ProjectionError::MissingTimezone(format!("Invalid timestamp: {}", e)))
}

/// Validate that timezone information is present
pub fn validate_timezone_present(dt: &DateTime<FixedOffset>) -> Result<(), ProjectionError> {
    // DateTime<FixedOffset> always has timezone, this is a type-level guarantee
    // This function exists for API consistency
    Ok(())
}
