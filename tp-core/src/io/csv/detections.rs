//! CSV parser for punctual & linear detections (T009).
//!
//! See `specs/004-train-detections/contracts/detections-csv.md`.

use std::collections::BTreeMap;
use std::path::Path;

use chrono::{DateTime, FixedOffset};

use crate::detections::error::DetectionError;
use crate::models::{
    Detection, DetectionKind, GeographicLocation, LinearDetection, PunctualDetection,
    TopologicalLocation,
};

/// Reserved (kind-specific) column names. All other columns become metadata.
const PUNCTUAL_RESERVED: &[&str] = &[
    "timestamp",
    "netelement_id",
    "intrinsic",
    "lat",
    "lon",
    "crs",
    "id",
    "source",
];

const LINEAR_RESERVED: &[&str] = &[
    "t_from",
    "t_to",
    "netelement_id",
    "start_intrinsic",
    "end_intrinsic",
    "id",
    "source",
];

/// Load detections from a CSV file.
pub fn load(path: &Path, expected_kind: DetectionKind) -> Result<Vec<Detection>, DetectionError> {
    let source_file = path.display().to_string();
    let text = std::fs::read_to_string(path)?;
    load_str(&text, &source_file, expected_kind)
}

/// In-memory variant of [`load`] that accepts the full CSV text. Required by
/// the .NET bindings (FR-012, no temp files).
pub fn load_str(
    text: &str,
    source_file: &str,
    expected_kind: DetectionKind,
) -> Result<Vec<Detection>, DetectionError> {
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(true)
        .flexible(false)
        .from_reader(text.as_bytes());

    let headers: Vec<String> = rdr
        .headers()
        .map_err(|e| DetectionError::InvalidSchema(format!("failed to read CSV header: {e}")))?
        .iter()
        .map(|s| s.trim_start_matches('\u{feff}').to_string())
        .collect();

    match expected_kind {
        DetectionKind::Punctual => parse_punctual(&mut rdr, &headers, source_file),
        DetectionKind::Linear => parse_linear(&mut rdr, &headers, source_file),
    }
}

fn require_columns(headers: &[String], required: &[&str]) -> Result<(), DetectionError> {
    for col in required {
        if !headers.iter().any(|h| h == col) {
            return Err(DetectionError::InvalidSchema(format!(
                "missing required column '{col}'"
            )));
        }
    }
    Ok(())
}

fn col<'a>(headers: &'a [String], record: &'a csv::StringRecord, name: &str) -> Option<&'a str> {
    let idx = headers.iter().position(|h| h == name)?;
    let v = record.get(idx)?;
    let v = v.trim();
    if v.is_empty() {
        None
    } else {
        Some(v)
    }
}

fn parse_timestamp(
    s: &str,
    source_file: &str,
    source_row: usize,
) -> Result<DateTime<FixedOffset>, DetectionError> {
    crate::temporal::parse_timestamp_flexible_str(s).map_err(|e| DetectionError::InvalidTimestamp {
        source_file: source_file.to_string(),
        source_row,
        message: format!("'{s}': {e}"),
    })
}

fn parse_intrinsic(s: &str, source_file: &str, source_row: usize) -> Result<f64, DetectionError> {
    let v: f64 = s.parse().map_err(|e| DetectionError::Parse {
        source_file: source_file.to_string(),
        source_row,
        message: format!("invalid float '{s}': {e}"),
    })?;
    if !(0.0..=1.0).contains(&v) {
        return Err(DetectionError::InvalidIntrinsic {
            source_file: source_file.to_string(),
            source_row,
            value: v,
        });
    }
    Ok(v)
}

fn parse_float(
    s: &str,
    source_file: &str,
    source_row: usize,
    field: &str,
) -> Result<f64, DetectionError> {
    s.parse::<f64>().map_err(|e| DetectionError::Parse {
        source_file: source_file.to_string(),
        source_row,
        message: format!("invalid float for '{field}': '{s}': {e}"),
    })
}

fn collect_metadata(
    headers: &[String],
    record: &csv::StringRecord,
    reserved: &[&str],
) -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();
    for (idx, name) in headers.iter().enumerate() {
        if reserved.iter().any(|r| r == name) {
            continue;
        }
        if let Some(v) = record.get(idx) {
            let v = v.trim();
            if !v.is_empty() {
                map.insert(name.clone(), v.to_string());
            }
        }
    }
    map
}

fn parse_punctual<R: std::io::Read>(
    rdr: &mut csv::Reader<R>,
    headers: &[String],
    source_file: &str,
) -> Result<Vec<Detection>, DetectionError> {
    require_columns(headers, &["timestamp"])?;

    let mut out = Vec::new();
    for (row_idx, result) in rdr.records().enumerate() {
        let record = result.map_err(|e| DetectionError::Parse {
            source_file: source_file.to_string(),
            source_row: row_idx + 2, // header is row 1
            message: format!("CSV read error: {e}"),
        })?;
        let source_row = row_idx + 2;

        let timestamp_str =
            col(headers, &record, "timestamp").ok_or_else(|| DetectionError::InvalidTimestamp {
                source_file: source_file.to_string(),
                source_row,
                message: "empty timestamp".to_string(),
            })?;
        let timestamp = parse_timestamp(timestamp_str, source_file, source_row)?;

        let netelement_id = col(headers, &record, "netelement_id").map(str::to_string);
        let lat = col(headers, &record, "lat");
        let lon = col(headers, &record, "lon");
        let crs = col(headers, &record, "crs");

        let has_topo = netelement_id.is_some();
        let has_coord = lat.is_some() || lon.is_some();

        if has_topo && has_coord {
            return Err(DetectionError::InvalidSchema(format!(
                "row {source_row}: cannot specify both 'netelement_id' and 'lat'/'lon'"
            )));
        }
        if !has_topo && !has_coord {
            return Err(DetectionError::InvalidSchema(format!(
                "row {source_row}: must specify either 'netelement_id' or 'lat'+'lon'+'crs'"
            )));
        }

        let intrinsic_value = match col(headers, &record, "intrinsic") {
            Some(s) => Some(parse_intrinsic(s, source_file, source_row)?),
            None => None,
        };

        let location = netelement_id.as_ref().map(|ne_id| TopologicalLocation {
            netelement_id: ne_id.clone(),
            intrinsic: intrinsic_value.unwrap_or(0.5),
        });

        let coordinates = if has_coord {
            let lat_s = lat.ok_or_else(|| {
                DetectionError::InvalidSchema(format!("row {source_row}: missing 'lat'"))
            })?;
            let lon_s = lon.ok_or_else(|| {
                DetectionError::InvalidSchema(format!("row {source_row}: missing 'lon'"))
            })?;
            let crs_s = crs.ok_or(DetectionError::MissingCrs {
                source_file: source_file.to_string(),
                source_row,
            })?;
            Some(GeographicLocation {
                latitude: parse_float(lat_s, source_file, source_row, "lat")?,
                longitude: parse_float(lon_s, source_file, source_row, "lon")?,
                crs: crs_s.to_string(),
            })
        } else {
            None
        };

        let id = col(headers, &record, "id").map(str::to_string);
        let source = col(headers, &record, "source").map(str::to_string);
        let metadata = collect_metadata(headers, &record, PUNCTUAL_RESERVED);

        out.push(Detection::Punctual(PunctualDetection {
            timestamp,
            location,
            coordinates,
            intrinsic: intrinsic_value,
            id,
            source,
            source_file: source_file.to_string(),
            source_row,
            metadata,
        }));
    }
    Ok(out)
}

fn parse_linear<R: std::io::Read>(
    rdr: &mut csv::Reader<R>,
    headers: &[String],
    source_file: &str,
) -> Result<Vec<Detection>, DetectionError> {
    require_columns(headers, &["t_from", "t_to", "netelement_id"])?;

    let mut out = Vec::new();
    for (row_idx, result) in rdr.records().enumerate() {
        let record = result.map_err(|e| DetectionError::Parse {
            source_file: source_file.to_string(),
            source_row: row_idx + 2,
            message: format!("CSV read error: {e}"),
        })?;
        let source_row = row_idx + 2;

        let t_from_s =
            col(headers, &record, "t_from").ok_or_else(|| DetectionError::InvalidTimestamp {
                source_file: source_file.to_string(),
                source_row,
                message: "empty t_from".to_string(),
            })?;
        let t_to_s =
            col(headers, &record, "t_to").ok_or_else(|| DetectionError::InvalidTimestamp {
                source_file: source_file.to_string(),
                source_row,
                message: "empty t_to".to_string(),
            })?;
        let t_from = parse_timestamp(t_from_s, source_file, source_row)?;
        let t_to = parse_timestamp(t_to_s, source_file, source_row)?;

        let netelement_id = col(headers, &record, "netelement_id")
            .ok_or_else(|| {
                DetectionError::InvalidSchema(format!("row {source_row}: empty 'netelement_id'"))
            })?
            .to_string();

        let start_intrinsic = match col(headers, &record, "start_intrinsic") {
            Some(s) => parse_intrinsic(s, source_file, source_row)?,
            None => 0.0,
        };
        let end_intrinsic = match col(headers, &record, "end_intrinsic") {
            Some(s) => parse_intrinsic(s, source_file, source_row)?,
            None => 1.0,
        };

        let id = col(headers, &record, "id").map(str::to_string);
        let source = col(headers, &record, "source").map(str::to_string);
        let metadata = collect_metadata(headers, &record, LINEAR_RESERVED);

        out.push(Detection::Linear(LinearDetection {
            t_from,
            t_to,
            netelement_id,
            start_intrinsic,
            end_intrinsic,
            id,
            source,
            source_file: source_file.to_string(),
            source_row,
            metadata,
        }));
    }
    Ok(out)
}
