//! CSV parsing and writing

use crate::errors::ProjectionError;
use crate::models::{GnssPosition, ProjectedPosition, TrainPath, AssociatedNetElement};
use chrono::{DateTime, FixedOffset};
use polars::prelude::*;
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
        .map_err(|e| {
            ProjectionError::IoError(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Failed to read CSV: {}", e),
            ))
        })?
        .finish()
        .map_err(|e| {
            ProjectionError::IoError(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Failed to parse CSV: {}", e),
            ))
        })?;

    // Validate required columns exist
    let schema = df.schema();
    if !schema.contains(lat_col) {
        return Err(ProjectionError::InvalidCoordinate(format!(
            "Latitude column '{}' not found in CSV",
            lat_col
        )));
    }
    if !schema.contains(lon_col) {
        return Err(ProjectionError::InvalidCoordinate(format!(
            "Longitude column '{}' not found in CSV",
            lon_col
        )));
    }
    if !schema.contains(time_col) {
        return Err(ProjectionError::InvalidTimestamp(format!(
            "Timestamp column '{}' not found in CSV",
            time_col
        )));
    }

    // Get all column names for metadata preservation
    let all_columns: Vec<String> = schema.iter_names().map(|s| s.to_string()).collect();

    // Extract required columns
    let lat_series = df.column(lat_col).map_err(|e| {
        ProjectionError::InvalidCoordinate(format!("Failed to get latitude: {}", e))
    })?;
    let lon_series = df.column(lon_col).map_err(|e| {
        ProjectionError::InvalidCoordinate(format!("Failed to get longitude: {}", e))
    })?;
    let time_series = df.column(time_col).map_err(|e| {
        ProjectionError::InvalidTimestamp(format!("Failed to get timestamp: {}", e))
    })?;

    // Convert to f64 arrays
    let lat_array = lat_series.f64().map_err(|e| {
        ProjectionError::InvalidCoordinate(format!("Latitude must be numeric: {}", e))
    })?;
    let lon_array = lon_series.f64().map_err(|e| {
        ProjectionError::InvalidCoordinate(format!("Longitude must be numeric: {}", e))
    })?;
    let time_array = time_series.str().map_err(|e| {
        ProjectionError::InvalidTimestamp(format!("Timestamp must be string: {}", e))
    })?;

    // Build GNSS positions
    let mut positions = Vec::new();
    let row_count = df.height();

    for i in 0..row_count {
        // Get coordinates
        let latitude = lat_array.get(i).ok_or_else(|| {
            ProjectionError::InvalidCoordinate(format!("Missing latitude at row {}", i))
        })?;
        let longitude = lon_array.get(i).ok_or_else(|| {
            ProjectionError::InvalidCoordinate(format!("Missing longitude at row {}", i))
        })?;

        // Get and parse timestamp
        let time_str = time_array.get(i).ok_or_else(|| {
            ProjectionError::InvalidTimestamp(format!("Missing timestamp at row {}", i))
        })?;

        let timestamp = DateTime::<FixedOffset>::parse_from_rfc3339(time_str)
            .map_err(|e| ProjectionError::InvalidTimestamp(
                format!("Invalid timestamp '{}' at row {}: {} (expected RFC3339 format with timezone, e.g., 2025-12-09T14:30:00+01:00)", 
                    time_str, i, e)
            ))?;

        // Validate timezone is present
        if timestamp.timezone().local_minus_utc() == 0
            && !time_str.contains('+')
            && !time_str.ends_with('Z')
        {
            return Err(ProjectionError::InvalidTimestamp(format!(
                "Timestamp at row {} missing explicit timezone offset",
                i
            )));
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
    use csv::Writer;

    let mut csv_writer = Writer::from_writer(writer);

    // Write header
    csv_writer.write_record(&[
        "original_lat",
        "original_lon",
        "original_time",
        "projected_lat",
        "projected_lon",
        "netelement_id",
        "measure_meters",
        "projection_distance_meters",
        "crs",
    ])?;

    // Write data rows
    for pos in positions {
        csv_writer.write_record(&[
            pos.original.latitude.to_string(),
            pos.original.longitude.to_string(),
            pos.original.timestamp.to_rfc3339(),
            pos.projected_coords.y().to_string(),
            pos.projected_coords.x().to_string(),
            pos.netelement_id.clone(),
            pos.measure_meters.to_string(),
            pos.projection_distance_meters.to_string(),
            pos.crs.clone(),
        ])?;
    }

    csv_writer.flush()?;
    Ok(())
}

/// Write TrainPath to CSV
///
/// Output format: One row per segment with columns:
/// - netelement_id: ID of the netelement
/// - probability: Segment probability (0.0 to 1.0)
/// - start_intrinsic: Entry point on netelement (0.0 to 1.0)
/// - end_intrinsic: Exit point on netelement (0.0 to 1.0)
/// - gnss_start_index: First GNSS position index
/// - gnss_end_index: Last GNSS position index
///
/// The overall_probability is written as a comment in the first line.
///
/// # Example Output
///
/// ```csv
/// # overall_probability: 0.89
/// netelement_id,probability,start_intrinsic,end_intrinsic,gnss_start_index,gnss_end_index
/// NE_A,0.87,0.0,1.0,0,10
/// NE_B,0.92,0.0,1.0,11,18
/// ```
pub fn write_trainpath_csv(
    train_path: &TrainPath,
    writer: &mut impl std::io::Write,
) -> Result<(), ProjectionError> {
    use csv::Writer;

    // Write overall probability as comment
    writeln!(writer, "# overall_probability: {}", train_path.overall_probability)?;
    
    if let Some(calculated_at) = &train_path.calculated_at {
        writeln!(writer, "# calculated_at: {}", calculated_at.to_rfc3339())?;
    }

    let mut csv_writer = Writer::from_writer(writer);

    // Write header
    csv_writer.write_record(&[
        "netelement_id",
        "probability",
        "start_intrinsic",
        "end_intrinsic",
        "gnss_start_index",
        "gnss_end_index",
    ])?;

    // Write data rows
    for segment in &train_path.segments {
        csv_writer.write_record(&[
            segment.netelement_id.clone(),
            segment.probability.to_string(),
            segment.start_intrinsic.to_string(),
            segment.end_intrinsic.to_string(),
            segment.gnss_start_index.to_string(),
            segment.gnss_end_index.to_string(),
        ])?;
    }

    csv_writer.flush()?;
    Ok(())
}

/// Parse TrainPath from CSV
///
/// Reads a CSV file in the format produced by write_trainpath_csv.
/// Expects columns: netelement_id, probability, start_intrinsic, end_intrinsic,
/// gnss_start_index, gnss_end_index
///
/// The overall_probability can be specified in a comment line starting with
/// `# overall_probability:` or will default to the average of segment probabilities.
///
/// # Arguments
///
/// * `path` - Path to CSV file
///
/// # Returns
///
/// A TrainPath struct reconstructed from the CSV data
pub fn parse_trainpath_csv(path: &str) -> Result<TrainPath, ProjectionError> {
    // Read the file to extract comment lines and filter them out
    let file_content = std::fs::read_to_string(path)?;
    let mut overall_probability: Option<f64> = None;
    let mut calculated_at: Option<chrono::DateTime<chrono::Utc>> = None;
    let mut csv_lines = Vec::new();

    // Parse comment lines and collect non-comment lines
    for line in file_content.lines() {
        if let Some(comment) = line.strip_prefix('#') {
            let comment = comment.trim();
            if let Some(value) = comment.strip_prefix("overall_probability:") {
                overall_probability = value.trim().parse().ok();
            } else if let Some(value) = comment.strip_prefix("calculated_at:") {
                if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(value.trim()) {
                    calculated_at = Some(dt.with_timezone(&chrono::Utc));
                }
            }
        } else {
            csv_lines.push(line);
        }
    }

    // Write filtered CSV to temporary string for polars
    let filtered_csv = csv_lines.join("\n");
    let temp_file = std::env::temp_dir().join(format!("trainpath_{}.csv", std::process::id()));
    std::fs::write(&temp_file, filtered_csv)?;

    // Read CSV using polars
    let df = CsvReadOptions::default()
        .with_has_header(true)
        .try_into_reader_with_file_path(Some(temp_file.clone()))
        .map_err(|e| {
            ProjectionError::IoError(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Failed to read TrainPath CSV: {}", e),
            ))
        })?
        .finish()
        .map_err(|e| {
            ProjectionError::IoError(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Failed to parse TrainPath CSV: {}", e),
            ))
        })?;

    // Clean up temp file
    let _ = std::fs::remove_file(temp_file);

    // Extract columns and cast to correct types
    let netelement_id = df.column("netelement_id")
        .map_err(|e| ProjectionError::GeoJsonError(format!("Missing netelement_id column: {}", e)))?
        .str()
        .map_err(|e| ProjectionError::GeoJsonError(format!("netelement_id must be string: {}", e)))?
        .clone();

    let probability_series = df.column("probability")
        .map_err(|e| ProjectionError::GeoJsonError(format!("Missing probability column: {}", e)))?
        .cast(&DataType::Float64)
        .map_err(|e| ProjectionError::GeoJsonError(format!("probability cast failed: {}", e)))?;
    let probability = probability_series.f64()
        .map_err(|e| ProjectionError::GeoJsonError(format!("probability must be numeric: {}", e)))?;

    let start_intrinsic_series = df.column("start_intrinsic")
        .map_err(|e| ProjectionError::GeoJsonError(format!("Missing start_intrinsic column: {}", e)))?
        .cast(&DataType::Float64)
        .map_err(|e| ProjectionError::GeoJsonError(format!("start_intrinsic cast failed: {}", e)))?;
    let start_intrinsic = start_intrinsic_series.f64()
        .map_err(|e| ProjectionError::GeoJsonError(format!("start_intrinsic must be numeric: {}", e)))?;

    let end_intrinsic_series = df.column("end_intrinsic")
        .map_err(|e| ProjectionError::GeoJsonError(format!("Missing end_intrinsic column: {}", e)))?
        .cast(&DataType::Float64)
        .map_err(|e| ProjectionError::GeoJsonError(format!("end_intrinsic cast failed: {}", e)))?;
    let end_intrinsic = end_intrinsic_series.f64()
        .map_err(|e| ProjectionError::GeoJsonError(format!("end_intrinsic must be numeric: {}", e)))?;

    let gnss_start_index_series = df.column("gnss_start_index")
        .map_err(|e| ProjectionError::GeoJsonError(format!("Missing gnss_start_index column: {}", e)))?
        .cast(&DataType::UInt32)
        .map_err(|e| ProjectionError::GeoJsonError(format!("gnss_start_index cast failed: {}", e)))?;
    let gnss_start_index = gnss_start_index_series.u32()
        .map_err(|e| ProjectionError::GeoJsonError(format!("gnss_start_index must be integer: {}", e)))?;

    let gnss_end_index_series = df.column("gnss_end_index")
        .map_err(|e| ProjectionError::GeoJsonError(format!("Missing gnss_end_index column: {}", e)))?
        .cast(&DataType::UInt32)
        .map_err(|e| ProjectionError::GeoJsonError(format!("gnss_end_index cast failed: {}", e)))?;
    let gnss_end_index = gnss_end_index_series.u32()
        .map_err(|e| ProjectionError::GeoJsonError(format!("gnss_end_index must be integer: {}", e)))?;

    // Build segments
    let mut segments = Vec::new();
    let row_count = df.height();

    for i in 0..row_count {
        let id = netelement_id.get(i).ok_or_else(|| {
            ProjectionError::GeoJsonError(format!("Missing netelement_id at row {}", i))
        })?.to_string();

        let prob = probability.get(i).ok_or_else(|| {
            ProjectionError::GeoJsonError(format!("Missing probability at row {}", i))
        })?;

        let start_intr = start_intrinsic.get(i).ok_or_else(|| {
            ProjectionError::GeoJsonError(format!("Missing start_intrinsic at row {}", i))
        })?;

        let end_intr = end_intrinsic.get(i).ok_or_else(|| {
            ProjectionError::GeoJsonError(format!("Missing end_intrinsic at row {}", i))
        })?;

        let start_idx = gnss_start_index.get(i).ok_or_else(|| {
            ProjectionError::GeoJsonError(format!("Missing gnss_start_index at row {}", i))
        })? as usize;

        let end_idx = gnss_end_index.get(i).ok_or_else(|| {
            ProjectionError::GeoJsonError(format!("Missing gnss_end_index at row {}", i))
        })? as usize;

        let segment = AssociatedNetElement::new(
            id,
            prob,
            start_intr,
            end_intr,
            start_idx,
            end_idx,
        )?;

        segments.push(segment);
    }

    // Calculate overall probability if not provided
    let overall_prob = overall_probability.unwrap_or_else(|| {
        let sum: f64 = segments.iter().map(|s| s.probability).sum();
        sum / segments.len() as f64
    });

    // Create TrainPath
    TrainPath::new(segments, overall_prob, calculated_at, None)
}
