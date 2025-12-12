//! CSV parsing and writing

use crate::models::{GnssPosition, ProjectedPosition};
use crate::errors::ProjectionError;
use polars::prelude::*;
use chrono::{DateTime, FixedOffset};
use std::collections::HashMap;

/// Parse GNSS positions from CSV file
pub fn parse_gnss_csv(
    path: &str,
    crs: &str,
    lat_col: &str,
    lon_col: &str,
    time_col: &str,
) -> Result<Vec<GnssPosition>, ProjectionError> {
    // Read CSV file using polars
    let df = CsvReadOptions::default()
        .with_has_header(true)
        .try_into_reader_with_file_path(Some(path.into()))
        .map_err(|e| ProjectionError::IoError(std::io::Error::new(std::io::ErrorKind::InvalidData, format!("Failed to read CSV: {}", e))))?
        .finish()
        .map_err(|e| ProjectionError::IoError(std::io::Error::new(std::io::ErrorKind::InvalidData, format!("Failed to parse CSV: {}", e))))?;
    
    // Validate required columns exist
    let schema = df.schema();
    if !schema.contains(lat_col) {
        return Err(ProjectionError::InvalidCoordinate(
            format!("Latitude column '{}' not found in CSV", lat_col)
        ));
    }
    if !schema.contains(lon_col) {
        return Err(ProjectionError::InvalidCoordinate(
            format!("Longitude column '{}' not found in CSV", lon_col)
        ));
    }
    if !schema.contains(time_col) {
        return Err(ProjectionError::InvalidTimestamp(
            format!("Timestamp column '{}' not found in CSV", time_col)
        ));
    }
    
    // Get all column names for metadata preservation
    let all_columns: Vec<String> = schema.iter_names().map(|s| s.to_string()).collect();
    
    // Extract required columns
    let lat_series = df.column(lat_col)
        .map_err(|e| ProjectionError::InvalidCoordinate(format!("Failed to get latitude: {}", e)))?;
    let lon_series = df.column(lon_col)
        .map_err(|e| ProjectionError::InvalidCoordinate(format!("Failed to get longitude: {}", e)))?;
    let time_series = df.column(time_col)
        .map_err(|e| ProjectionError::InvalidTimestamp(format!("Failed to get timestamp: {}", e)))?;
    
    // Convert to f64 arrays
    let lat_array = lat_series.f64()
        .map_err(|e| ProjectionError::InvalidCoordinate(format!("Latitude must be numeric: {}", e)))?;
    let lon_array = lon_series.f64()
        .map_err(|e| ProjectionError::InvalidCoordinate(format!("Longitude must be numeric: {}", e)))?;
    let time_array = time_series.str()
        .map_err(|e| ProjectionError::InvalidTimestamp(format!("Timestamp must be string: {}", e)))?;
    
    // Build GNSS positions
    let mut positions = Vec::new();
    let row_count = df.height();
    
    for i in 0..row_count {
        // Get coordinates
        let latitude = lat_array.get(i)
            .ok_or_else(|| ProjectionError::InvalidCoordinate(
                format!("Missing latitude at row {}", i)
            ))?;
        let longitude = lon_array.get(i)
            .ok_or_else(|| ProjectionError::InvalidCoordinate(
                format!("Missing longitude at row {}", i)
            ))?;
        
        // Get and parse timestamp
        let time_str = time_array.get(i)
            .ok_or_else(|| ProjectionError::InvalidTimestamp(
                format!("Missing timestamp at row {}", i)
            ))?;
        
        let timestamp = DateTime::<FixedOffset>::parse_from_rfc3339(time_str)
            .map_err(|e| ProjectionError::InvalidTimestamp(
                format!("Invalid timestamp '{}' at row {}: {} (expected RFC3339 format with timezone, e.g., 2025-12-09T14:30:00+01:00)", 
                    time_str, i, e)
            ))?;
        
        // Validate timezone is present
        if timestamp.timezone().local_minus_utc() == 0 && !time_str.contains('+') && !time_str.ends_with('Z') {
            return Err(ProjectionError::InvalidTimestamp(
                format!("Timestamp at row {} missing explicit timezone offset", i)
            ));
        }
        
        // Build metadata from other columns
        let mut metadata = HashMap::new();
        for col_name in &all_columns {
            if col_name != lat_col && col_name != lon_col && col_name != time_col {
                if let Ok(series) = df.column(col_name) {
                    if let Ok(str_series) = series.cast(&DataType::String) {
                        if let Ok(str_chunked) = str_series.str() {
                            if let Some(value) = str_chunked.get(i) {
                                metadata.insert(col_name.clone(), value.to_string());
                            }
                        }
                    }
                }
            }
        }
        
        // Create GNSS position
        let mut position = GnssPosition::new(latitude, longitude, timestamp, crs.to_string())?;
        position.metadata = metadata;
        positions.push(position);
    }
    
    Ok(positions)
}

/// Write projected positions to CSV
pub fn write_csv(
    positions: &[ProjectedPosition],
    writer: &mut impl std::io::Write,
) -> Result<(), ProjectionError> {
    // TODO: Implement CSV writing with all required columns
    Ok(())
}
