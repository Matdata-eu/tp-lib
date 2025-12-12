//! GeoJSON parsing and writing

use crate::models::{Netelement, ProjectedPosition};
use crate::errors::ProjectionError;
use geojson::{GeoJson, Feature, Value};
use geo::{LineString, Coord};
use std::fs;

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
    // TODO: Implement GeoJSON writing with Point geometries and properties
    Ok(())
}
