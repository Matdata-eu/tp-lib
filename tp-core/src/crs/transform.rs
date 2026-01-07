//! CRS transformation utilities using PROJ library

use crate::errors::ProjectionError;
use geo::Point;

#[cfg(feature = "crs-transform")]
use proj::Proj;

/// Wrapper around PROJ for coordinate reference system transformations
///
/// This transformer is feature-gated behind `crs-transform` to avoid requiring
/// PROJ system dependencies by default. Enable with `--features crs-transform`.
pub struct CrsTransformer {
    source_crs: String,
    target_crs: String,
    #[cfg(feature = "crs-transform")]
    proj: Proj,
}

impl CrsTransformer {
    /// Create a new CRS transformer
    pub fn new(source_crs: String, target_crs: String) -> Result<Self, ProjectionError> {
        #[cfg(feature = "crs-transform")]
        {
            let proj = Proj::new_known_crs(&source_crs, &target_crs, None).map_err(|e| {
                ProjectionError::InvalidCrs(format!(
                    "Failed to create PROJ transformation from {} to {}: {}",
                    source_crs, target_crs, e
                ))
            })?;
            Ok(Self {
                source_crs,
                target_crs,
                proj,
            })
        }

        #[cfg(not(feature = "crs-transform"))]
        {
            Ok(Self {
                source_crs,
                target_crs,
            })
        }
    }

    /// Transform a point from source CRS to target CRS
    pub fn transform(&self, point: Point<f64>) -> Result<Point<f64>, ProjectionError> {
        #[cfg(feature = "crs-transform")]
        {
            let (x, y) = self.proj.convert((point.x(), point.y())).map_err(|e| {
                ProjectionError::TransformFailed(format!("PROJ transformation failed: {}", e))
            })?;
            Ok(Point::new(x, y))
        }

        #[cfg(not(feature = "crs-transform"))]
        {
            // Identity transformation when PROJ feature is disabled
            // This allows basic testing without the native PROJ dependency
            Ok(point)
        }
    }
}
