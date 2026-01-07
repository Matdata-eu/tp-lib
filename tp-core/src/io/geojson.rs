//! GeoJSON parsing and writing

use crate::models::{GnssPosition, Netelement, ProjectedPosition};
use crate::errors::ProjectionError;
use geojson::{GeoJson, Feature, Value};
use geo::{LineString, Coord};
use std::fs;
use chrono::DateTime;

/// Parse railway network from GeoJSON file
pub fn parse_network_geojson(path: &str) -> Result<Vec<Netelement>, ProjectionError> {
    // Read file
    let geojson_str = fs::read_to_string(path)?;
    
    // Parse GeoJSON
    let geojson = geojson_str.parse::<GeoJson>()
        .map_err(|e| ProjectionError::InvalidGeometry(
            format!("Failed to parse GeoJSON: {}", e)
        ))?;
    
    // Extract FeatureCollection
    let feature_collection = match geojson {
        GeoJson::FeatureCollection(fc) => fc,
        _ => return Err(ProjectionError::InvalidGeometry(
            "GeoJSON must be a FeatureCollection".to_string()
        )),
    };
    
    // Determine CRS - RFC 7946 specifies WGS84 is the default
    let default_crs = "EPSG:4326".to_string();
    let crs = if let Some(crs_obj) = &feature_collection.foreign_members {
        if let Some(crs_value) = crs_obj.get("crs") {
            // Try to extract CRS from properties.name
            if let Some(props) = crs_value.get("properties") {
                if let Some(name) = props.get("name") {
                    if let Some(name_str) = name.as_str() {
                        // Handle URN format: "urn:ogc:def:crs:EPSG::4326"
                        if name_str.contains("EPSG") {
                            if let Some(code) = name_str.split("::").last() {
                                format!("EPSG:{}", code)
                            } else {
                                default_crs.clone()
                            }
                        } else {
                            default_crs.clone()
                        }
                    } else {
                        default_crs.clone()
                    }
                } else {
                    default_crs.clone()
                }
            } else {
                default_crs.clone()
            }
        } else {
            default_crs.clone()
        }
    } else {
        default_crs.clone()
    };
    
    // Validate CRS is WGS84 per RFC 7946
    if !crs.contains("4326") && !crs.contains("WGS84") {
        return Err(ProjectionError::InvalidCrs(
            format!("GeoJSON CRS must be WGS84 (EPSG:4326) per RFC 7946, got: {}", crs)
        ));
    }
    
    // Parse features
    let mut netelements = Vec::new();
    
    for (idx, feature) in feature_collection.features.iter().enumerate() {
        let netelement = parse_feature(feature, &crs, idx)?;
        netelements.push(netelement);
    }
    
    if netelements.is_empty() {
        return Err(ProjectionError::EmptyNetwork);
    }
    
    Ok(netelements)
}

/// Parse GNSS positions from GeoJSON file
///
/// Expects a FeatureCollection with Point geometries and properties:
/// - `timestamp`: RFC3339 timestamp with timezone
/// - Optional: other properties will be stored as metadata
///
/// # Arguments
///
/// * `path` - Path to GeoJSON file
/// * `crs` - Expected CRS of the coordinates (e.g., "EPSG:4326")
///
/// # Returns
///
/// Vector of GnssPosition structs
///
/// # Example GeoJSON
///
/// ```json
/// {
///   "type": "FeatureCollection",
///   "features": [
///     {
///       "type": "Feature",
///       "geometry": {
///         "type": "Point",
///         "coordinates": [4.3517, 50.8503]
///       },
///       "properties": {
///         "timestamp": "2025-12-09T14:30:00+01:00",
///         "vehicle_id": "TRAIN_001"
///       }
///     }
///   ]
/// }
/// ```
pub fn parse_gnss_geojson(path: &str, crs: &str) -> Result<Vec<GnssPosition>, ProjectionError> {
    // Read file
    let geojson_str = fs::read_to_string(path)?;
    
    // Parse GeoJSON
    let geojson = geojson_str.parse::<GeoJson>()
        .map_err(|e| ProjectionError::GeoJsonError(
            format!("Failed to parse GeoJSON: {}", e)
        ))?;
    
    // Extract FeatureCollection
    let feature_collection = match geojson {
        GeoJson::FeatureCollection(fc) => fc,
        _ => return Err(ProjectionError::GeoJsonError(
            "GeoJSON must be a FeatureCollection".to_string()
        )),
    };
    
    // Parse features
    let mut positions = Vec::new();
    
    for (idx, feature) in feature_collection.features.iter().enumerate() {
        let position = parse_gnss_feature(feature, crs, idx)?;
        positions.push(position);
    }
    
    if positions.is_empty() {
        return Err(ProjectionError::GeoJsonError(
            "GeoJSON contains no valid GNSS positions".to_string()
        ));
    }
    
    Ok(positions)
}

/// Parse a single GeoJSON feature into a GnssPosition
fn parse_gnss_feature(feature: &Feature, crs: &str, idx: usize) -> Result<GnssPosition, ProjectionError> {
    // Get geometry
    let geometry = feature.geometry.as_ref()
        .ok_or_else(|| ProjectionError::GeoJsonError(
            format!("Feature {} missing geometry", idx)
        ))?;
    
    // Extract Point coordinates (longitude, latitude)
    let (longitude, latitude) = match &geometry.value {
        Value::Point(coords) => {
            if coords.len() < 2 {
                return Err(ProjectionError::InvalidCoordinate(
                    format!("Feature {} Point must have at least 2 coordinates", idx)
                ));
            }
            (coords[0], coords[1])
        },
        _ => return Err(ProjectionError::GeoJsonError(
            format!("Feature {} must have Point geometry for GNSS position", idx)
        )),
    };
    
    // Validate coordinates
    if !(-90.0..=90.0).contains(&latitude) {
        return Err(ProjectionError::InvalidCoordinate(
            format!("Feature {}: latitude {} out of range [-90, 90]", idx, latitude)
        ));
    }
    if !(-180.0..=180.0).contains(&longitude) {
        return Err(ProjectionError::InvalidCoordinate(
            format!("Feature {}: longitude {} out of range [-180, 180]", idx, longitude)
        ));
    }
    
    // Get properties
    let properties = feature.properties.as_ref()
        .ok_or_else(|| ProjectionError::GeoJsonError(
            format!("Feature {} missing properties (timestamp required)", idx)
        ))?;
    
    // Extract timestamp
    let timestamp_str = properties.get("timestamp")
        .and_then(|v| v.as_str())
        .ok_or_else(|| ProjectionError::MissingTimezone(
            format!("Feature {} missing 'timestamp' property", idx)
        ))?;
    
    // Parse timestamp with timezone
    let timestamp = DateTime::parse_from_rfc3339(timestamp_str)
        .map_err(|e| ProjectionError::InvalidTimestamp(
            format!("Feature {}: invalid timestamp '{}': {}", idx, timestamp_str, e)
        ))?;
    
    // Extract metadata (all properties except timestamp)
    let mut metadata = std::collections::HashMap::new();
    for (key, value) in properties {
        if key != "timestamp" {
            if let Some(str_value) = value.as_str() {
                metadata.insert(key.clone(), str_value.to_string());
            } else {
                metadata.insert(key.clone(), value.to_string());
            }
        }
    }
    
    Ok(GnssPosition {
        latitude,
        longitude,
        timestamp,
        crs: crs.to_string(),
        metadata,
    })
}

/// Parse a single GeoJSON feature into a Netelement
fn parse_feature(feature: &Feature, crs: &str, idx: usize) -> Result<Netelement, ProjectionError> {
    // Get geometry
    let geometry = feature.geometry.as_ref()
        .ok_or_else(|| ProjectionError::InvalidGeometry(
            format!("Feature {} missing geometry", idx)
        ))?;
    
    // Extract LineString
    let linestring = match &geometry.value {
        Value::LineString(coords) => {
            let geo_coords: Vec<Coord<f64>> = coords.iter()
                .map(|pos| Coord { x: pos[0], y: pos[1] })
                .collect();
            LineString::from(geo_coords)
        },
        Value::MultiLineString(lines) => {
            // For MultiLineString, use first line (or could concatenate)
            if lines.is_empty() {
                return Err(ProjectionError::InvalidGeometry(
                    format!("Feature {} has empty MultiLineString", idx)
                ));
            }
            let geo_coords: Vec<Coord<f64>> = lines[0].iter()
                .map(|pos| Coord { x: pos[0], y: pos[1] })
                .collect();
            LineString::from(geo_coords)
        },
        _ => return Err(ProjectionError::InvalidGeometry(
            format!("Feature {} must have LineString or MultiLineString geometry", idx)
        )),
    };
    
    // Get ID from properties or generate from index
    let id = if let Some(props) = &feature.properties {
        if let Some(id_value) = props.get("id") {
            id_value.as_str()
                .map(|s| s.to_string())
                .or_else(|| id_value.as_i64().map(|i| i.to_string()))
                .unwrap_or_else(|| format!("NE_{}", idx))
        } else {
            format!("NE_{}", idx)
        }
    } else {
        format!("NE_{}", idx)
    };
    
    Netelement::new(id, linestring, crs.to_string())
}

/// Write projected positions as GeoJSON FeatureCollection
pub fn write_geojson(
    positions: &[ProjectedPosition],
    writer: &mut impl std::io::Write,
) -> Result<(), ProjectionError> {
    use geojson::{Feature, FeatureCollection, Geometry, Value};
    use serde_json::{Map, Value as JsonValue};
    
    let mut features = Vec::new();
    
    for pos in positions {
        // Create Point geometry for projected position
        let geometry = Geometry::new(Value::Point(vec![
            pos.projected_coords.x(),
            pos.projected_coords.y(),
        ]));
        
        // Create properties
        let mut properties = Map::new();
        properties.insert("original_lat".to_string(), JsonValue::from(pos.original.latitude));
        properties.insert("original_lon".to_string(), JsonValue::from(pos.original.longitude));
        properties.insert("original_time".to_string(), JsonValue::from(pos.original.timestamp.to_rfc3339()));
        properties.insert("netelement_id".to_string(), JsonValue::from(pos.netelement_id.clone()));
        properties.insert("measure_meters".to_string(), JsonValue::from(pos.measure_meters));
        properties.insert("projection_distance_meters".to_string(), JsonValue::from(pos.projection_distance_meters));
        properties.insert("crs".to_string(), JsonValue::from(pos.crs.clone()));
        
        // Add original metadata
        for (key, value) in &pos.original.metadata {
            properties.insert(format!("original_{}", key), JsonValue::from(value.clone()));
        }
        
        let feature = Feature {
            bbox: None,
            geometry: Some(geometry),
            id: None,
            properties: Some(properties),
            foreign_members: None,
        };
        
        features.push(feature);
    }
    
    let feature_collection = FeatureCollection {
        bbox: None,
        features,
        foreign_members: None,
    };
    
    let json = serde_json::to_string_pretty(&feature_collection)
        .map_err(|e| ProjectionError::GeoJsonError(format!("Failed to serialize GeoJSON: {}", e)))?;
    
    writer.write_all(json.as_bytes())?;
    Ok(())
}
