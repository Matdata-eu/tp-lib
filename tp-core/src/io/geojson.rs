//! GeoJSON parsing and writing

use crate::models::{Netelement, ProjectedPosition};
use crate::errors::ProjectionError;

/// Parse railway network from GeoJSON file
pub fn parse_network_geojson(path: &str) -> Result<Vec<Netelement>, ProjectionError> {
    // TODO: Implement GeoJSON parsing for FeatureCollection with LineString geometries
    Ok(Vec::new())
}

/// Write projected positions as GeoJSON FeatureCollection
pub fn write_geojson(
    positions: &[ProjectedPosition],
    writer: &mut impl std::io::Write,
) -> Result<(), ProjectionError> {
    // TODO: Implement GeoJSON writing with Point geometries and properties
    Ok(())
}
