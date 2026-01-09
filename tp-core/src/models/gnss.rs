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

    /// Train heading in degrees (0-360), None if not available
    /// 0° = North, 90° = East, 180° = South, 270° = West
    pub heading: Option<f64>,

    /// Distance from previous GNSS position in meters, None if not available or first position
    pub distance: Option<f64>,
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
            heading: None,
            distance: None,
        };

        position.validate()?;
        Ok(position)
    }

    /// Create a new GNSS position with optional heading and distance
    pub fn with_heading_distance(
        latitude: f64,
        longitude: f64,
        timestamp: DateTime<FixedOffset>,
        crs: String,
        heading: Option<f64>,
        distance: Option<f64>,
    ) -> Result<Self, ProjectionError> {
        let position = Self {
            latitude,
            longitude,
            timestamp,
            crs,
            metadata: HashMap::new(),
            heading,
            distance,
        };

        position.validate()?;
        position.validate_heading()?;
        Ok(position)
    }

    /// Validate heading if present (must be 0-360°)
    pub fn validate_heading(&self) -> Result<(), ProjectionError> {
        if let Some(heading) = self.heading {
            if !(0.0..=360.0).contains(&heading) {
                return Err(ProjectionError::InvalidGeometry(format!(
                    "Heading must be in range [0, 360], got {}",
                    heading
                )));
            }
        }
        Ok(())
    }

    /// Check if two headings are opposite
    /// Returns true if headings are closer to 180° apart than to 0° apart
    ///
    /// Logic: Compare distance to 180° shift vs normal distance
    /// If shifting by 180° gives smaller circular distance, they're opposite
    pub fn is_opposite_heading(h1: f64, h2: f64) -> bool {
        // Calculate normal circular distance
        let diff_normal = (h1 - h2).abs();
        let dist_normal = diff_normal.min(360.0 - diff_normal);

        // Calculate distance when one heading is shifted by 180°
        let diff_shifted = (h1 - h2 - 180.0).abs() % 360.0;
        let dist_shifted = diff_shifted.min(360.0 - diff_shifted);

        // If shifted distance is smaller, they're opposite
        dist_shifted < dist_normal
    }

    /// Calculate angular difference between two headings
    /// Accounts for circular nature of compass bearings
    /// Accounts for possible opposite headings (180° apart)
    pub fn heading_difference(h1: f64, h2: f64) -> f64 {
        // Check if headings are opposite
        if Self::is_opposite_heading(h1, h2) {
            // Opposite headings: return the small angular deviation from exactly 180°
            let diff_shifted = (h1 - h2 - 180.0).abs() % 360.0;
            diff_shifted.min(360.0 - diff_shifted)
        } else {
            // Not opposite: return normal circular distance
            let diff = (h1 - h2).abs();
            diff.min(360.0 - diff)
        }
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
