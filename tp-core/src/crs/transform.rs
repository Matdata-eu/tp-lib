//! CRS transformation utilities using proj4rs library
//!
//! This module uses proj4rs, a pure Rust implementation of PROJ.4,
//! with crs-definitions for EPSG code lookup.

use crate::errors::ProjectionError;
use geo::Point;
use proj4rs::proj::Proj;

/// Wrapper around proj4rs for coordinate reference system transformations
///
/// Uses proj4rs (pure Rust PROJ.4 implementation) with no system dependencies.
/// EPSG codes are resolved using the crs-definitions crate.
pub struct CrsTransformer {
    source_crs: String,
    target_crs: String,
    from_proj: Proj,
    to_proj: Proj,
    source_is_geographic: bool,
    target_is_geographic: bool,
}

impl CrsTransformer {
    /// Create a new CRS transformer
    ///
    /// # Arguments
    /// * `source_crs` - Source CRS as EPSG code (e.g., "EPSG:4326") or PROJ string
    /// * `target_crs` - Target CRS as EPSG code (e.g., "EPSG:31370") or PROJ string
    pub fn new(source_crs: String, target_crs: String) -> Result<Self, ProjectionError> {
        // Convert EPSG codes to PROJ strings
        let source_proj_str = Self::epsg_to_proj_string(&source_crs)?;
        let target_proj_str = Self::epsg_to_proj_string(&target_crs)?;

        let from_proj = Proj::from_proj_string(&source_proj_str).map_err(|e| {
            ProjectionError::InvalidCrs(format!(
                "Failed to create source projection from {}: {:?}",
                source_crs, e
            ))
        })?;

        let to_proj = Proj::from_proj_string(&target_proj_str).map_err(|e| {
            ProjectionError::InvalidCrs(format!(
                "Failed to create target projection from {}: {:?}",
                target_crs, e
            ))
        })?;

        // Detect if projections are geographic (longlat)
        let source_is_geographic = source_proj_str.contains("+proj=longlat");
        let target_is_geographic = target_proj_str.contains("+proj=longlat");

        Ok(Self {
            source_crs,
            target_crs,
            from_proj,
            to_proj,
            source_is_geographic,
            target_is_geographic,
        })
    }

    fn epsg_to_proj_string(epsg: &str) -> Result<String, ProjectionError> {
        // Handle EPSG:xxxx format
        let code = if epsg.starts_with("EPSG:") {
            epsg.strip_prefix("EPSG:")
                .and_then(|s| s.parse::<u16>().ok())
        } else {
            epsg.parse::<u16>().ok()
        };

        if let Some(code) = code {
            // Use crs-definitions to get PROJ string
            crs_definitions::from_code(code)
                .ok_or_else(|| ProjectionError::InvalidCrs(format!("Unknown EPSG code: {}", epsg)))
                .map(|def| def.proj4.to_string())
        } else {
            // If not EPSG code, assume it's already a PROJ string
            Ok(epsg.to_string())
        }
    }

    /// Transform a point from source CRS to target CRS
    ///
    /// Handles automatic radian/degree conversion for geographic coordinate systems.
    pub fn transform(&self, point: Point<f64>) -> Result<Point<f64>, ProjectionError> {
        let mut coord = (point.x(), point.y(), 0.0);

        // Convert input from degrees to radians if source is geographic
        if self.source_is_geographic {
            coord.0 = coord.0.to_radians();
            coord.1 = coord.1.to_radians();
        }

        // Perform transformation
        proj4rs::transform::transform(&self.from_proj, &self.to_proj, &mut coord).map_err(|e| {
            ProjectionError::TransformFailed(format!(
                "proj4rs transformation failed ({} -> {}): {:?}",
                self.source_crs, self.target_crs, e
            ))
        })?;

        // Convert output from radians to degrees if target is geographic
        if self.target_is_geographic {
            coord.0 = coord.0.to_degrees();
            coord.1 = coord.1.to_degrees();
        }

        Ok(Point::new(coord.0, coord.1))
    }
}
