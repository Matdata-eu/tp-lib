//! GNSS Position data model

use crate::errors::ProjectionError;
use chrono::{DateTime, FixedOffset};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Represents a single GNSS measurement from a train journey
///
/// Each `GnssPosition` captures a timestamped geographic location with explicit
/// coordinate reference system (CRS) information. Additional metadata can be
/// preserved for audit trails and debugging.
///
/// # Validation
///
/// - Latitude must be in range [-90.0, 90.0]
/// - Longitude must be in range [-180.0, 180.0]
/// - Timestamp must include timezone information (RFC3339 format)
///
/// # Examples
///
/// ```
/// use tp_lib_core::GnssPosition;
/// use chrono::{DateTime, FixedOffset};
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let timestamp = DateTime::parse_from_rfc3339("2025-12-09T14:30:00+01:00")?;
///
/// let position = GnssPosition::new(
///     50.8503,  // latitude
///     4.3517,   // longitude
///     timestamp,
///     "EPSG:4326".to_string(),
/// )?;
///
/// assert_eq!(position.latitude, 50.8503);
/// assert_eq!(position.crs, "EPSG:4326");
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GnssPosition {
    /// Latitude in decimal degrees (-90.0 to 90.0)
    pub latitude: f64,

    /// Longitude in decimal degrees (-180.0 to 180.0)
    pub longitude: f64,

    /// Timestamp with timezone offset (e.g., 2025-12-09T14:30:00+01:00)
    pub timestamp: DateTime<FixedOffset>,

    /// Coordinate Reference System (e.g., "EPSG:4326" for WGS84)
    pub crs: String,

    /// Additional metadata from CSV (preserved for output)
    pub metadata: HashMap<String, String>,
}

impl GnssPosition {
    /// Create a new GNSS position with validation
    pub fn new(
        latitude: f64,
        longitude: f64,
        timestamp: DateTime<FixedOffset>,
        crs: String,
    ) -> Result<Self, ProjectionError> {
        let position = Self {
            latitude,
            longitude,
            timestamp,
            crs,
            metadata: HashMap::new(),
        };

        position.validate()?;
        Ok(position)
    }

    /// Validate latitude range
    pub fn validate_latitude(&self) -> Result<(), ProjectionError> {
        if self.latitude < -90.0 || self.latitude > 90.0 {
            return Err(ProjectionError::InvalidCoordinate(format!(
                "Latitude {} out of range [-90, 90]",
                self.latitude
            )));
        }
        Ok(())
    }

    /// Validate longitude range
    pub fn validate_longitude(&self) -> Result<(), ProjectionError> {
        if self.longitude < -180.0 || self.longitude > 180.0 {
            return Err(ProjectionError::InvalidCoordinate(format!(
                "Longitude {} out of range [-180, 180]",
                self.longitude
            )));
        }
        Ok(())
    }

    /// Validate timezone is present (type-level guarantee with `DateTime<FixedOffset>`)
    pub fn validate_timezone(&self) -> Result<(), ProjectionError> {
        // DateTime<FixedOffset> always has timezone information
        // This function exists for API completeness
        Ok(())
    }

    /// Validate all fields
    fn validate(&self) -> Result<(), ProjectionError> {
        self.validate_latitude()?;
        self.validate_longitude()?;
        self.validate_timezone()?;

        // Validate CRS format (basic check)
        if self.crs.is_empty() {
            return Err(ProjectionError::InvalidCrs(
                "CRS must not be empty".to_string(),
            ));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn test_valid_position() {
        let timestamp = FixedOffset::east_opt(3600)
            .unwrap()
            .with_ymd_and_hms(2025, 12, 9, 14, 30, 0)
            .unwrap();

        let pos = GnssPosition::new(50.8503, 4.3517, timestamp, "EPSG:4326".to_string());

        assert!(pos.is_ok());
    }

    #[test]
    fn test_invalid_latitude() {
        let timestamp = FixedOffset::east_opt(3600)
            .unwrap()
            .with_ymd_and_hms(2025, 12, 9, 14, 30, 0)
            .unwrap();

        let pos = GnssPosition::new(
            91.0, // Invalid
            4.3517,
            timestamp,
            "EPSG:4326".to_string(),
        );

        assert!(pos.is_err());
    }

    #[test]
    fn test_invalid_longitude() {
        let timestamp = FixedOffset::east_opt(3600)
            .unwrap()
            .with_ymd_and_hms(2025, 12, 9, 14, 30, 0)
            .unwrap();

        let pos = GnssPosition::new(
            50.8503,
            181.0, // Invalid
            timestamp,
            "EPSG:4326".to_string(),
        );

        assert!(pos.is_err());
    }
}
