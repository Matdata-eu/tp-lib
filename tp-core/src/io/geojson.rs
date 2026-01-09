//! GeoJSON parsing and writing

use crate::errors::ProjectionError;
use crate::models::{GnssPosition, NetRelation, Netelement, ProjectedPosition};
use chrono::DateTime;
use geo::{Coord, LineString};
use geojson::{Feature, GeoJson, Value};
use std::fs;

/// Parse railway network from GeoJSON file
///
/// Loads both netelements and netrelations from a single GeoJSON FeatureCollection.
/// Netelements are features with LineString/MultiLineString geometry (without type="netrelation").
/// Netrelations are features with type="netrelation" property.
///
/// # Arguments
///
/// * `path` - Path to GeoJSON file containing both network elements and relations
///
/// # Returns
///
/// A tuple containing `(Vec<Netelement>, Vec<NetRelation>)`
///
/// # Example
///
/// ```no_run
/// use tp_lib_core::io::parse_network_geojson;
///
/// let (netelements, netrelations) = parse_network_geojson("network.geojson")?;
/// # Ok::<_, Box<dyn std::error::Error>>(())
/// ```
pub fn parse_network_geojson(
    path: &str,
) -> Result<(Vec<Netelement>, Vec<NetRelation>), ProjectionError> {
    // Read file
    let geojson_str = fs::read_to_string(path)?;

    // Parse GeoJSON
    let geojson = geojson_str
        .parse::<GeoJson>()
        .map_err(|e| ProjectionError::InvalidGeometry(format!("Failed to parse GeoJSON: {}", e)))?;

    // Extract FeatureCollection
    let feature_collection = match geojson {
        GeoJson::FeatureCollection(fc) => fc,
        _ => {
            return Err(ProjectionError::InvalidGeometry(
                "GeoJSON must be a FeatureCollection".to_string(),
            ))
        }
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
        return Err(ProjectionError::InvalidCrs(format!(
            "GeoJSON CRS must be WGS84 (EPSG:4326) per RFC 7946, got: {}",
            crs
        )));
    }

    // Parse features, separating netelements and netrelations
    let mut netelements = Vec::new();
    let mut netrelations = Vec::new();

    for (idx, feature) in feature_collection.features.iter().enumerate() {
        // Check if this is a netrelation feature
        if let Some(props) = &feature.properties {
            if let Some(feature_type) = props.get("type") {
                if feature_type.as_str() == Some("netrelation") {
                    let netrelation = parse_netrelation_feature(feature, idx)?;
                    netrelations.push(netrelation);
                    continue;
                }
            }
        }

        // Otherwise parse as netelement
        let netelement = parse_feature(feature, &crs, idx)?;
        netelements.push(netelement);
    }

    if netelements.is_empty() {
        return Err(ProjectionError::EmptyNetwork);
    }

    Ok((netelements, netrelations))
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
    let geojson = geojson_str
        .parse::<GeoJson>()
        .map_err(|e| ProjectionError::GeoJsonError(format!("Failed to parse GeoJSON: {}", e)))?;

    // Extract FeatureCollection
    let feature_collection = match geojson {
        GeoJson::FeatureCollection(fc) => fc,
        _ => {
            return Err(ProjectionError::GeoJsonError(
                "GeoJSON must be a FeatureCollection".to_string(),
            ))
        }
    };

    // Parse features
    let mut positions = Vec::new();

    for (idx, feature) in feature_collection.features.iter().enumerate() {
        let position = parse_gnss_feature(feature, crs, idx)?;
        positions.push(position);
    }

    if positions.is_empty() {
        return Err(ProjectionError::GeoJsonError(
            "GeoJSON contains no valid GNSS positions".to_string(),
        ));
    }

    Ok(positions)
}

/// Parse a single GeoJSON feature into a GnssPosition
fn parse_gnss_feature(
    feature: &Feature,
    crs: &str,
    idx: usize,
) -> Result<GnssPosition, ProjectionError> {
    // Get geometry
    let geometry = feature.geometry.as_ref().ok_or_else(|| {
        ProjectionError::GeoJsonError(format!("Feature {} missing geometry", idx))
    })?;

    // Extract Point coordinates (longitude, latitude)
    let (longitude, latitude) = match &geometry.value {
        Value::Point(coords) => {
            if coords.len() < 2 {
                return Err(ProjectionError::InvalidCoordinate(format!(
                    "Feature {} Point must have at least 2 coordinates",
                    idx
                )));
            }
            (coords[0], coords[1])
        }
        _ => {
            return Err(ProjectionError::GeoJsonError(format!(
                "Feature {} must have Point geometry for GNSS position",
                idx
            )))
        }
    };

    // Validate coordinates
    if !(-90.0..=90.0).contains(&latitude) {
        return Err(ProjectionError::InvalidCoordinate(format!(
            "Feature {}: latitude {} out of range [-90, 90]",
            idx, latitude
        )));
    }
    if !(-180.0..=180.0).contains(&longitude) {
        return Err(ProjectionError::InvalidCoordinate(format!(
            "Feature {}: longitude {} out of range [-180, 180]",
            idx, longitude
        )));
    }

    // Get properties
    let properties = feature.properties.as_ref().ok_or_else(|| {
        ProjectionError::GeoJsonError(format!(
            "Feature {} missing properties (timestamp required)",
            idx
        ))
    })?;

    // Extract timestamp
    let timestamp_str = properties
        .get("timestamp")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            ProjectionError::MissingTimezone(format!(
                "Feature {} missing 'timestamp' property",
                idx
            ))
        })?;

    // Parse timestamp with timezone
    let timestamp = DateTime::parse_from_rfc3339(timestamp_str).map_err(|e| {
        ProjectionError::InvalidTimestamp(format!(
            "Feature {}: invalid timestamp '{}': {}",
            idx, timestamp_str, e
        ))
    })?;

    // Extract metadata (all properties except timestamp, heading, distance)
    let mut metadata = std::collections::HashMap::new();
    let mut heading: Option<f64> = None;
    let mut distance: Option<f64> = None;

    for (key, value) in properties {
        match key.as_str() {
            "timestamp" => {} // Skip, already extracted
            "heading" => {
                // Extract optional heading (0-360°)
                if let Some(h) = value.as_f64() {
                    if (0.0..=360.0).contains(&h) {
                        heading = Some(h);
                    } else {
                        return Err(ProjectionError::InvalidGeometry(format!(
                            "Feature {}: heading {} not in [0, 360]",
                            idx, h
                        )));
                    }
                }
            }
            "distance" => {
                // Extract optional distance (>= 0)
                if let Some(d) = value.as_f64() {
                    if d >= 0.0 {
                        distance = Some(d);
                    } else {
                        return Err(ProjectionError::InvalidGeometry(format!(
                            "Feature {}: distance {} must be >= 0",
                            idx, d
                        )));
                    }
                }
            }
            _ => {
                // Store other properties as metadata
                if let Some(str_value) = value.as_str() {
                    metadata.insert(key.clone(), str_value.to_string());
                } else {
                    metadata.insert(key.clone(), value.to_string());
                }
            }
        }
    }

    Ok(GnssPosition {
        latitude,
        longitude,
        timestamp,
        crs: crs.to_string(),
        metadata,
        heading,
        distance,
    })
}

/// Parse a single GeoJSON feature into a Netelement
fn parse_feature(feature: &Feature, crs: &str, idx: usize) -> Result<Netelement, ProjectionError> {
    // Get geometry
    let geometry = feature.geometry.as_ref().ok_or_else(|| {
        ProjectionError::InvalidGeometry(format!("Feature {} missing geometry", idx))
    })?;

    // Extract LineString
    let linestring = match &geometry.value {
        Value::LineString(coords) => {
            let geo_coords: Vec<Coord<f64>> = coords
                .iter()
                .map(|pos| Coord {
                    x: pos[0],
                    y: pos[1],
                })
                .collect();
            LineString::from(geo_coords)
        }
        Value::MultiLineString(lines) => {
            // For MultiLineString, use first line (or could concatenate)
            if lines.is_empty() {
                return Err(ProjectionError::InvalidGeometry(format!(
                    "Feature {} has empty MultiLineString",
                    idx
                )));
            }
            let geo_coords: Vec<Coord<f64>> = lines[0]
                .iter()
                .map(|pos| Coord {
                    x: pos[0],
                    y: pos[1],
                })
                .collect();
            LineString::from(geo_coords)
        }
        _ => {
            return Err(ProjectionError::InvalidGeometry(format!(
                "Feature {} must have LineString or MultiLineString geometry",
                idx
            )))
        }
    };

    // Get ID from properties or generate from index
    let id = if let Some(props) = &feature.properties {
        if let Some(id_value) = props.get("id") {
            id_value
                .as_str()
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

/// Parse netrelations from GeoJSON file
///
/// Expects a FeatureCollection with features that have `type="netrelation"` property.
/// Netrelations can have optional Point geometry representing the connection point.
///
/// # Required Properties
///
/// - `type`: Must be "netrelation"
/// - `id`: Netrelation identifier
/// - `netelementA`: ID of first netelement
/// - `netelementB`: ID of second netelement
/// - `positionOnA`: Position on netelementA (0 or 1)
/// - `positionOnB`: Position on netelementB (0 or 1)
/// - `navigability`: "both", "AB", "BA", or "none"
///
/// # Arguments
///
/// * `path` - Path to GeoJSON file
///
/// # Returns
///
/// Vector of NetRelation structs
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
///         "type": "netrelation",
///         "id": "NR_001",
///         "netelementA": "NE_001",
///         "netelementB": "NE_002",
///         "positionOnA": 1,
///         "positionOnB": 0,
///         "navigability": "both"
///       }
///     }
///   ]
/// }
/// ```
pub fn parse_netrelations_geojson(path: &str) -> Result<Vec<NetRelation>, ProjectionError> {
    // Read file
    let geojson_str = fs::read_to_string(path)?;

    // Parse GeoJSON
    let geojson = geojson_str
        .parse::<GeoJson>()
        .map_err(|e| ProjectionError::GeoJsonError(format!("Failed to parse GeoJSON: {}", e)))?;

    // Extract FeatureCollection
    let feature_collection = match geojson {
        GeoJson::FeatureCollection(fc) => fc,
        _ => {
            return Err(ProjectionError::GeoJsonError(
                "GeoJSON must be a FeatureCollection".to_string(),
            ))
        }
    };

    // Parse features, filtering for type="netrelation"
    let mut netrelations = Vec::new();

    for (idx, feature) in feature_collection.features.iter().enumerate() {
        // Check if this is a netrelation feature
        if let Some(props) = &feature.properties {
            if let Some(feature_type) = props.get("type") {
                if feature_type.as_str() == Some("netrelation") {
                    let netrelation = parse_netrelation_feature(feature, idx)?;
                    netrelations.push(netrelation);
                }
            }
        }
    }

    Ok(netrelations)
}

/// Parse a single GeoJSON feature into a NetRelation
fn parse_netrelation_feature(
    feature: &Feature,
    idx: usize,
) -> Result<NetRelation, ProjectionError> {
    // Get properties
    let properties = feature.properties.as_ref().ok_or_else(|| {
        ProjectionError::GeoJsonError(format!("Netrelation feature {} missing properties", idx))
    })?;

    // Extract ID
    let id = properties
        .get("id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            ProjectionError::GeoJsonError(format!(
                "Netrelation feature {} missing 'id' property",
                idx
            ))
        })?
        .to_string();

    // Extract netelementA
    let netelement_a = properties
        .get("netelementA")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            ProjectionError::GeoJsonError(format!(
                "Netrelation feature {} missing 'netelementA' property",
                idx
            ))
        })?
        .to_string();

    // Extract netelementB
    let netelement_b = properties
        .get("netelementB")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            ProjectionError::GeoJsonError(format!(
                "Netrelation feature {} missing 'netelementB' property",
                idx
            ))
        })?
        .to_string();

    // Extract positionOnA (0 or 1)
    let position_on_a = properties
        .get("positionOnA")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| {
            ProjectionError::GeoJsonError(format!(
                "Netrelation feature {} missing or invalid 'positionOnA' property",
                idx
            ))
        })? as u8;

    // Extract positionOnB (0 or 1)
    let position_on_b = properties
        .get("positionOnB")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| {
            ProjectionError::GeoJsonError(format!(
                "Netrelation feature {} missing or invalid 'positionOnB' property",
                idx
            ))
        })? as u8;

    // Extract navigability and convert to boolean flags
    let navigability_str = properties
        .get("navigability")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            ProjectionError::GeoJsonError(format!(
                "Netrelation feature {} missing 'navigability' property",
                idx
            ))
        })?;

    let (navigable_forward, navigable_backward) = match navigability_str {
        "both" => (true, true),
        "AB" => (true, false),
        "BA" => (false, true),
        "none" => (false, false),
        _ => return Err(ProjectionError::GeoJsonError(
            format!("Netrelation feature {}: invalid navigability value '{}' (expected: both, AB, BA, or none)", idx, navigability_str)
        )),
    };

    // Create NetRelation
    let netrelation = NetRelation::new(
        id,
        netelement_a,
        netelement_b,
        position_on_a,
        position_on_b,
        navigable_forward,
        navigable_backward,
    )?;

    Ok(netrelation)
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
        properties.insert(
            "original_lat".to_string(),
            JsonValue::from(pos.original.latitude),
        );
        properties.insert(
            "original_lon".to_string(),
            JsonValue::from(pos.original.longitude),
        );
        properties.insert(
            "original_time".to_string(),
            JsonValue::from(pos.original.timestamp.to_rfc3339()),
        );
        properties.insert(
            "netelement_id".to_string(),
            JsonValue::from(pos.netelement_id.clone()),
        );
        properties.insert(
            "measure_meters".to_string(),
            JsonValue::from(pos.measure_meters),
        );
        properties.insert(
            "projection_distance_meters".to_string(),
            JsonValue::from(pos.projection_distance_meters),
        );
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

    let json = serde_json::to_string_pretty(&feature_collection).map_err(|e| {
        ProjectionError::GeoJsonError(format!("Failed to serialize GeoJSON: {}", e))
    })?;

    writer.write_all(json.as_bytes())?;
    Ok(())
}

/// Write TrainPath as GeoJSON FeatureCollection
///
/// Serializes a TrainPath to GeoJSON, with each segment as a separate feature.
/// The overall path probability and metadata are stored in the FeatureCollection properties.
///
/// # Arguments
///
/// * `train_path` - The TrainPath to serialize
/// * `netelements` - Map of netelement IDs to Netelement geometries (for creating LineString features)
/// * `writer` - Output writer
///
/// # Output Format
///
/// ```json
/// {
///   "type": "FeatureCollection",
///   "properties": {
///     "overall_probability": 0.89,
///     "calculated_at": "2025-01-15T10:30:00Z",
///     "distance_scale": 10.0,
///     "heading_scale": 2.0
///   },
///   "features": [
///     {
///       "type": "Feature",
///       "geometry": { "type": "LineString", "coordinates": [...] },
///       "properties": {
///         "netelement_id": "NE_A",
///         "probability": 0.87,
///         "start_intrinsic": 0.0,
///         "end_intrinsic": 1.0,
///         "gnss_start_index": 0,
///         "gnss_end_index": 10
///       }
///     }
///   ]
/// }
/// ```
pub fn write_trainpath_geojson(
    train_path: &crate::models::TrainPath,
    netelements: &std::collections::HashMap<String, Netelement>,
    writer: &mut impl std::io::Write,
) -> Result<(), ProjectionError> {
    use geojson::{Feature, FeatureCollection, Geometry, Value};
    use serde_json::{Map, Value as JsonValue};

    let mut features = Vec::new();

    // Create a feature for each segment
    for segment in &train_path.segments {
        // Look up the netelement geometry
        let netelement = netelements.get(&segment.netelement_id).ok_or_else(|| {
            ProjectionError::InvalidGeometry(format!(
                "Netelement {} not found in provided map",
                segment.netelement_id
            ))
        })?;

        // Extract the portion of the linestring covered by this segment
        // For simplicity, use the full linestring geometry
        // (In a production system, you'd extract the substring from start_intrinsic to end_intrinsic)
        let coords: Vec<Vec<f64>> = netelement
            .geometry
            .points()
            .map(|point| vec![point.x(), point.y()])
            .collect();

        let geometry = Geometry::new(Value::LineString(coords));

        // Create properties for this segment
        let mut properties = Map::new();
        properties.insert(
            "netelement_id".to_string(),
            JsonValue::from(segment.netelement_id.clone()),
        );
        properties.insert(
            "probability".to_string(),
            JsonValue::from(segment.probability),
        );
        properties.insert(
            "start_intrinsic".to_string(),
            JsonValue::from(segment.start_intrinsic),
        );
        properties.insert(
            "end_intrinsic".to_string(),
            JsonValue::from(segment.end_intrinsic),
        );
        properties.insert(
            "gnss_start_index".to_string(),
            JsonValue::from(segment.gnss_start_index as i64),
        );
        properties.insert(
            "gnss_end_index".to_string(),
            JsonValue::from(segment.gnss_end_index as i64),
        );

        let feature = Feature {
            bbox: None,
            geometry: Some(geometry),
            id: None,
            properties: Some(properties),
            foreign_members: None,
        };

        features.push(feature);
    }

    // Create FeatureCollection with overall properties
    let mut fc_properties = Map::new();
    fc_properties.insert(
        "overall_probability".to_string(),
        JsonValue::from(train_path.overall_probability),
    );

    if let Some(calculated_at) = &train_path.calculated_at {
        fc_properties.insert(
            "calculated_at".to_string(),
            JsonValue::from(calculated_at.to_rfc3339()),
        );
    }

    // Add metadata if present
    if let Some(metadata) = &train_path.metadata {
        fc_properties.insert(
            "distance_scale".to_string(),
            JsonValue::from(metadata.distance_scale),
        );
        fc_properties.insert(
            "heading_scale".to_string(),
            JsonValue::from(metadata.heading_scale),
        );
        fc_properties.insert(
            "cutoff_distance".to_string(),
            JsonValue::from(metadata.cutoff_distance),
        );
        fc_properties.insert(
            "heading_cutoff".to_string(),
            JsonValue::from(metadata.heading_cutoff),
        );
        fc_properties.insert(
            "probability_threshold".to_string(),
            JsonValue::from(metadata.probability_threshold),
        );
        if let Some(resampling_dist) = metadata.resampling_distance {
            fc_properties.insert(
                "resampling_distance".to_string(),
                JsonValue::from(resampling_dist),
            );
        }
        fc_properties.insert(
            "fallback_mode".to_string(),
            JsonValue::from(metadata.fallback_mode),
        );
        fc_properties.insert(
            "candidate_paths_evaluated".to_string(),
            JsonValue::from(metadata.candidate_paths_evaluated as i64),
        );
        fc_properties.insert(
            "bidirectional_path".to_string(),
            JsonValue::from(metadata.bidirectional_path),
        );
    }

    let mut foreign_members = Map::new();
    foreign_members.insert("properties".to_string(), JsonValue::Object(fc_properties));

    let feature_collection = FeatureCollection {
        bbox: None,
        features,
        foreign_members: Some(foreign_members),
    };

    let json = serde_json::to_string_pretty(&feature_collection).map_err(|e| {
        ProjectionError::GeoJsonError(format!("Failed to serialize TrainPath GeoJSON: {}", e))
    })?;

    writer.write_all(json.as_bytes())?;
    Ok(())
}
