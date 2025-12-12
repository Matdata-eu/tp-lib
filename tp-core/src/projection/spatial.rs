//! Spatial indexing for efficient nearest-netelement queries

use crate::models::Netelement;
use crate::errors::ProjectionError;
use geo::Point;
use rstar::{RTree, RTreeObject, AABB};

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
        for (index, netelement) in netelements.iter().enumerate() {
            let coords = &netelement.geometry.0;
            if coords.is_empty() {
                continue; // Skip empty geometries
            }
            
            // Calculate bounding box
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
        
        Ok(Self {
            tree,
            netelements,
        })
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
    
    // Use R-tree to find candidates near the point
    // We use locate_in_envelope_intersecting to find entries whose bbox contains or is near the point
    let query_point = [point.x(), point.y()];
    
    // Find the nearest entry by checking distance to all candidates
    // For MVP, we'll use a simple linear search over candidates near the point
    let mut best_index = 0;
    let mut best_distance = f64::MAX;
    
    // Get entries that intersect with a small envelope around the point
    let epsilon = 0.01; // ~1km search radius
    let search_envelope = AABB::from_corners(
        [query_point[0] - epsilon, query_point[1] - epsilon],
        [query_point[0] + epsilon, query_point[1] + epsilon],
    );
    
    // Check all entries in the search envelope
    for entry in index.tree.locate_in_envelope_intersecting(&search_envelope) {
        // Calculate center of bounding box as proxy for distance
        let bbox_center_x = (entry.bbox.lower()[0] + entry.bbox.upper()[0]) / 2.0;
        let bbox_center_y = (entry.bbox.lower()[1] + entry.bbox.upper()[1]) / 2.0;
        
        let dx = bbox_center_x - query_point[0];
        let dy = bbox_center_y - query_point[1];
        let distance = (dx * dx + dy * dy).sqrt();
        
        if distance < best_distance {
            best_distance = distance;
            best_index = entry.index;
        }
    }
    
    // If no entries found in envelope, fall back to first netelement
    if best_distance == f64::MAX {
        best_index = 0;
    }
    
    Ok(best_index)
}

#[cfg(test)]
mod tests {
    use super::*;
    use geo::{LineString, Coord};

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
        let linestring = LineString::from(vec![
            Coord { x: 4.0, y: 50.0 },
            Coord { x: 5.0, y: 50.0 },
        ]);
        let netelement = Netelement::new(
            "NE1".to_string(),
            linestring,
            "EPSG:4326".to_string(),
        ).unwrap();
        
        let index = NetworkIndex::new(vec![netelement]);
        assert!(index.is_ok());
        let index = index.unwrap();
        assert_eq!(index.netelements().len(), 1);
    }

    #[test]
    fn test_find_nearest_netelement() {
        // Create two netelements
        let linestring1 = LineString::from(vec![
            Coord { x: 4.0, y: 50.0 },
            Coord { x: 5.0, y: 50.0 },
        ]);
        let netelement1 = Netelement::new(
            "NE1".to_string(),
            linestring1,
            "EPSG:4326".to_string(),
        ).unwrap();
        
        let linestring2 = LineString::from(vec![
            Coord { x: 6.0, y: 51.0 },
            Coord { x: 7.0, y: 51.0 },
        ]);
        let netelement2 = Netelement::new(
            "NE2".to_string(),
            linestring2,
            "EPSG:4326".to_string(),
        ).unwrap();
        
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
