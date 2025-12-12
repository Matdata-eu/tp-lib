//! CSV parsing and writing

use crate::models::{GnssPosition, ProjectedPosition};
use crate::errors::ProjectionError;

/// Parse GNSS positions from CSV file
pub fn parse_gnss_csv(
    path: &str,
    crs: &str,
    lat_col: &str,
    lon_col: &str,
    time_col: &str,
) -> Result<Vec<GnssPosition>, ProjectionError> {
    // TODO: Implement CSV parsing with configurable columns
    Ok(Vec::new())
}

/// Write projected positions to CSV
pub fn write_csv(
    positions: &[ProjectedPosition],
    writer: &mut impl std::io::Write,
) -> Result<(), ProjectionError> {
    // TODO: Implement CSV writing with all required columns
    Ok(())
}
