//! Spatial indexing for efficient nearest-netelement queries

use crate::errors::ProjectionError;
use crate::models::Netelement;
use geo::Point;
use rstar::{PointDistance, RTree, RTreeObject, AABB};

/// Wrapper for netelement entries in the R-tree with bounding box
#[derive(Debug, Clone)]
struct NetelementIndexEntry {
    /// Index of the netelement in the original Vec<Netelement>
    index: usize,
    /// Bounding box of the netelement geometry (min/max lon/lat)
    bbox: AABB<[f64; 2]>,
}

impl RTreeObject for NetelementIndexEntry {
    type Envelope = AABB<[f64; 2]>;

    fn envelope(&self) -> Self::Envelope {
        self.bbox
    }
}

impl PointDistance for NetelementIndexEntry {
    fn distance_2(&self, point: &[f64; 2]) -> f64 {
        // Calculate squared distance from point to bounding box
        self.bbox.distance_2(point)
    }
}

/// Spatial index for netelements using R-tree
pub struct NetworkIndex {
    tree: RTree<NetelementIndexEntry>,
    netelements: Vec<Netelement>,
}

impl NetworkIndex {
    /// Build spatial index from netelements
    pub fn new(netelements: Vec<Netelement>) -> Result<Self, ProjectionError> {
        if netelements.is_empty() {
            return Err(ProjectionError::EmptyNetwork);
        }

        // Build R-tree entries with bounding boxes
        let mut entries = Vec::new();

        // Loop over all netelements
        for (index, netelement) in netelements.iter().enumerate() {
            let coords = &netelement.geometry.0;
            if coords.is_empty() {
                continue; // Skip empty geometries
            }

            // Calculate bounding box of netelement
            let mut min_x = f64::MAX;
            let mut max_x = f64::MIN;
            let mut min_y = f64::MAX;
            let mut max_y = f64::MIN;

            for coord in coords {
                min_x = min_x.min(coord.x);
                max_x = max_x.max(coord.x);
                min_y = min_y.min(coord.y);
                max_y = max_y.max(coord.y);
            }

            let bbox = AABB::from_corners([min_x, min_y], [max_x, max_y]);
            entries.push(NetelementIndexEntry { index, bbox });
        }

        let tree = RTree::bulk_load(entries);

        Ok(Self { tree, netelements })
    }

    /// Get a reference to the netelements
    pub fn netelements(&self) -> &[Netelement] {
        &self.netelements
    }
}

/// Find the nearest netelement to a given point using R-tree spatial index
pub fn find_nearest_netelement(
    point: &Point<f64>,
    index: &NetworkIndex,
) -> Result<usize, ProjectionError> {
    if index.netelements.is_empty() {
        return Err(ProjectionError::EmptyNetwork);
    }

    let query_point = [point.x(), point.y()];

    // Use R-tree's nearest_neighbor to efficiently find the closest netelement
    // This uses the distance to the bounding box envelope as approximation
    let nearest_entry = index.tree.nearest_neighbor(&query_point).ok_or_else(|| {
        ProjectionError::InvalidGeometry("Could not find nearest netelement".to_string())
    })?;

    Ok(nearest_entry.index)
}

#[cfg(test)]
mod tests {
    use super::*;
    use geo::{Coord, LineString};

    #[test]
    fn test_network_index_empty() {
        let result = NetworkIndex::new(vec![]);
        assert!(result.is_err());
        if let Err(ProjectionError::EmptyNetwork) = result {
            // Expected
        } else {
            panic!("Expected EmptyNetwork error");
        }
    }

    #[test]
    fn test_network_index_single_netelement() {
        let linestring =
            LineString::from(vec![Coord { x: 4.0, y: 50.0 }, Coord { x: 5.0, y: 50.0 }]);
        let netelement =
            Netelement::new("NE1".to_string(), linestring, "EPSG:4326".to_string()).unwrap();

        let index = NetworkIndex::new(vec![netelement]);
        assert!(index.is_ok());
        let index = index.unwrap();
        assert_eq!(index.netelements().len(), 1);
    }

    #[test]
    fn test_find_nearest_netelement() {
        // Create two netelements
        let linestring1 =
            LineString::from(vec![Coord { x: 4.0, y: 50.0 }, Coord { x: 5.0, y: 50.0 }]);
        let netelement1 =
            Netelement::new("NE1".to_string(), linestring1, "EPSG:4326".to_string()).unwrap();

        let linestring2 =
            LineString::from(vec![Coord { x: 6.0, y: 51.0 }, Coord { x: 7.0, y: 51.0 }]);
        let netelement2 =
            Netelement::new("NE2".to_string(), linestring2, "EPSG:4326".to_string()).unwrap();

        let index = NetworkIndex::new(vec![netelement1, netelement2]).unwrap();

        // Point closer to first netelement
        let point1 = Point::new(4.5, 50.0);
        let nearest1 = find_nearest_netelement(&point1, &index).unwrap();
        assert_eq!(nearest1, 0, "Point should be nearest to first netelement");

        // Point closer to second netelement
        let point2 = Point::new(6.5, 51.0);
        let nearest2 = find_nearest_netelement(&point2, &index).unwrap();
        assert_eq!(nearest2, 1, "Point should be nearest to second netelement");
    }
}
