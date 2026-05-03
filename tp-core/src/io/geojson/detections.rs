//! GeoJSON parser for punctual & linear detections (T010).
//!
//! See `specs/004-train-detections/contracts/detections-geojson.md`.

use std::collections::BTreeMap;
use std::path::Path;

use chrono::{DateTime, FixedOffset};
use geojson::{GeoJson, Geometry, Value};
use serde_json::{Map as JsonMap, Value as Json};

use crate::detections::error::DetectionError;
use crate::models::{
    Detection, DetectionKind, GeographicLocation, LinearDetection, PunctualDetection,
    TopologicalLocation,
};

/// Reserved property names per kind. All others go to `metadata`.
const PUNCTUAL_RESERVED: &[&str] = &[
    "kind",
    "timestamp",
    "netelement_id",
    "intrinsic",
    "crs",
    "id",
    "source",
];

const LINEAR_RESERVED: &[&str] = &[
    "kind",
    "t_from",
    "t_to",
    "netelement_id",
    "start_intrinsic",
    "end_intrinsic",
    "id",
    "source",
];

/// Load detections from a GeoJSON / JSON file.
pub fn load(path: &Path, expected_kind: DetectionKind) -> Result<Vec<Detection>, DetectionError> {
    let source_file = path.display().to_string();
    let raw = std::fs::read_to_string(path)?;
    let gj: GeoJson = raw.parse().map_err(|e: geojson::Error| {
        DetectionError::InvalidSchema(format!("invalid GeoJSON: {e}"))
    })?;

    let fc = match gj {
        GeoJson::FeatureCollection(fc) => fc,
        _ => {
            return Err(DetectionError::InvalidSchema(
                "top-level must be a FeatureCollection".to_string(),
            ))
        }
    };

    let mut out = Vec::with_capacity(fc.features.len());
    for (idx, feature) in fc.features.into_iter().enumerate() {
        let source_row = idx;
        let props = feature
            .properties
            .ok_or_else(|| DetectionError::InvalidSchema(format!("feature[{idx}]: missing 'properties'")))?;

        let kind_str = require_str(&props, "kind", &source_file, source_row)?;
        let actual_kind = match kind_str.as_str() {
            "punctual" => DetectionKind::Punctual,
            "linear" => DetectionKind::Linear,
            other => {
                return Err(DetectionError::InvalidSchema(format!(
                    "feature[{idx}]: unknown kind '{other}'"
                )))
            }
        };
        if actual_kind != expected_kind {
            return Err(DetectionError::InvalidSchema(format!(
                "feature[{idx}]: kind '{kind_str}' does not match expected"
            )));
        }

        let detection = match expected_kind {
            DetectionKind::Punctual => parse_punctual(
                &props,
                feature.geometry.as_ref(),
                &source_file,
                source_row,
            )?,
            DetectionKind::Linear => parse_linear(&props, &source_file, source_row)?,
        };
        out.push(detection);
    }
    Ok(out)
}

fn require_str(
    props: &JsonMap<String, Json>,
    key: &str,
    source_file: &str,
    source_row: usize,
) -> Result<String, DetectionError> {
    match props.get(key) {
        Some(Json::String(s)) if !s.trim().is_empty() => Ok(s.clone()),
        Some(_) => Err(DetectionError::InvalidSchema(format!(
            "feature[{source_row}]: property '{key}' must be a non-empty string"
        ))),
        None => Err(DetectionError::InvalidSchema(format!(
            "feature[{source_row}]: missing required property '{key}' in {source_file}"
        ))),
    }
}

fn opt_str(props: &JsonMap<String, Json>, key: &str) -> Option<String> {
    match props.get(key) {
        Some(Json::String(s)) if !s.trim().is_empty() => Some(s.clone()),
        _ => None,
    }
}

fn opt_intrinsic(
    props: &JsonMap<String, Json>,
    key: &str,
    source_file: &str,
    source_row: usize,
) -> Result<Option<f64>, DetectionError> {
    let Some(v) = props.get(key) else {
        return Ok(None);
    };
    let n = v.as_f64().ok_or_else(|| DetectionError::Parse {
        source_file: source_file.to_string(),
        source_row,
        message: format!("'{key}' must be a number"),
    })?;
    if !(0.0..=1.0).contains(&n) {
        return Err(DetectionError::InvalidIntrinsic {
            source_file: source_file.to_string(),
            source_row,
            value: n,
        });
    }
    Ok(Some(n))
}

fn parse_ts(
    s: &str,
    source_file: &str,
    source_row: usize,
) -> Result<DateTime<FixedOffset>, DetectionError> {
    DateTime::parse_from_rfc3339(s).map_err(|e| DetectionError::InvalidTimestamp {
        source_file: source_file.to_string(),
        source_row,
        message: format!("'{s}': {e}"),
    })
}

fn collect_metadata(props: &JsonMap<String, Json>, reserved: &[&str]) -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();
    for (k, v) in props.iter() {
        if reserved.iter().any(|r| r == k) {
            continue;
        }
        let s = match v {
            Json::String(s) => s.clone(),
            Json::Null => continue,
            other => other.to_string(),
        };
        map.insert(k.clone(), s);
    }
    map
}

fn parse_punctual(
    props: &JsonMap<String, Json>,
    geom: Option<&Geometry>,
    source_file: &str,
    source_row: usize,
) -> Result<Detection, DetectionError> {
    let timestamp_s = require_str(props, "timestamp", source_file, source_row)?;
    let timestamp = parse_ts(&timestamp_s, source_file, source_row)?;

    let netelement_id = opt_str(props, "netelement_id");
    let intrinsic_value = opt_intrinsic(props, "intrinsic", source_file, source_row)?;

    let coordinates = match geom {
        None => None,
        Some(g) => match &g.value {
            Value::Point(coords) => {
                if coords.len() < 2 {
                    return Err(DetectionError::InvalidSchema(format!(
                        "feature[{source_row}]: Point must have [lon, lat]"
                    )));
                }
                let crs = opt_str(props, "crs").unwrap_or_else(|| "EPSG:4326".to_string());
                Some(GeographicLocation {
                    latitude: coords[1],
                    longitude: coords[0],
                    crs,
                })
            }
            _ => {
                return Err(DetectionError::InvalidSchema(format!(
                    "feature[{source_row}]: punctual geometry must be Point or null"
                )))
            }
        },
    };

    let has_topo = netelement_id.is_some();
    let has_coord = coordinates.is_some();
    if has_topo && has_coord {
        return Err(DetectionError::InvalidSchema(format!(
            "feature[{source_row}]: cannot specify both 'netelement_id' and Point geometry"
        )));
    }
    if !has_topo && !has_coord {
        return Err(DetectionError::InvalidSchema(format!(
            "feature[{source_row}]: must specify either 'netelement_id' or Point geometry"
        )));
    }

    let location = netelement_id.map(|id| TopologicalLocation {
        netelement_id: id,
        intrinsic: intrinsic_value.unwrap_or(0.5),
    });

    Ok(Detection::Punctual(PunctualDetection {
        timestamp,
        location,
        coordinates,
        intrinsic: intrinsic_value,
        id: opt_str(props, "id"),
        source: opt_str(props, "source"),
        source_file: source_file.to_string(),
        source_row,
        metadata: collect_metadata(props, PUNCTUAL_RESERVED),
    }))
}

fn parse_linear(
    props: &JsonMap<String, Json>,
    source_file: &str,
    source_row: usize,
) -> Result<Detection, DetectionError> {
    let t_from = parse_ts(
        &require_str(props, "t_from", source_file, source_row)?,
        source_file,
        source_row,
    )?;
    let t_to = parse_ts(
        &require_str(props, "t_to", source_file, source_row)?,
        source_file,
        source_row,
    )?;
    let netelement_id = require_str(props, "netelement_id", source_file, source_row)?;
    let start_intrinsic = opt_intrinsic(props, "start_intrinsic", source_file, source_row)?
        .unwrap_or(0.0);
    let end_intrinsic =
        opt_intrinsic(props, "end_intrinsic", source_file, source_row)?.unwrap_or(1.0);

    Ok(Detection::Linear(LinearDetection {
        t_from,
        t_to,
        netelement_id,
        start_intrinsic,
        end_intrinsic,
        id: opt_str(props, "id"),
        source: opt_str(props, "source"),
        source_file: source_file.to_string(),
        source_row,
        metadata: collect_metadata(props, LINEAR_RESERVED),
    }))
}
