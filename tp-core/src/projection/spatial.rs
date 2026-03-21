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
    /// Segments of the linestring geometry stored as pairs of endpoints.
    /// Used by `distance_2` to compute the true point-to-linestring distance
    /// instead of the coarser bounding-box distance.  Without this, points
    /// that fall *inside* multiple bounding boxes all get distance² = 0,
    /// which makes `nearest_neighbor` return an arbitrary (often wrong) result.
    segments: Vec<([f64; 2], [f64; 2])>,
}

impl RTreeObject for NetelementIndexEntry {
    type Envelope = AABB<[f64; 2]>;

    fn envelope(&self) -> Self::Envelope {
        self.bbox
    }
}

/// Squared Euclidean distance from a point to a line segment (in degree units).
///
/// Returns the minimum squared distance from `p` to the segment `[a, b]`.
#[inline]
fn point_to_segment_dist_2(p: &[f64; 2], a: &[f64; 2], b: &[f64; 2]) -> f64 {
    let dx = b[0] - a[0];
    let dy = b[1] - a[1];
    let len_sq = dx * dx + dy * dy;
    let (px, py) = if len_sq == 0.0 {
        // Degenerate segment: both endpoints are identical
        (a[0], a[1])
    } else {
        let t = ((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / len_sq;
        let t = t.clamp(0.0, 1.0);
        (a[0] + t * dx, a[1] + t * dy)
    };
    let ex = p[0] - px;
    let ey = p[1] - py;
    ex * ex + ey * ey
}

impl PointDistance for NetelementIndexEntry {
    fn distance_2(&self, point: &[f64; 2]) -> f64 {
        // Use the actual point-to-linestring distance (squared, in degree units)
        // instead of the bounding-box distance.  The bbox distance is 0 for any
        // point that lies inside the bbox, which makes it impossible for the
        // R-tree to distinguish between overlapping netelements.
        self.segments
            .iter()
            .map(|(a, b)| point_to_segment_dist_2(point, a, b))
            .fold(f64::MAX, f64::min)
    }
}

/// Spatial index for netelements using R-tree
#[derive(Clone)]
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

        // Build R-tree entries with bounding boxes and segment lists
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

            // Store linestring segments for accurate distance computation
            let segments: Vec<([f64; 2], [f64; 2])> = coords
                .windows(2)
                .map(|w| ([w[0].x, w[0].y], [w[1].x, w[1].y]))
                .collect();

            entries.push(NetelementIndexEntry {
                index,
                bbox,
                segments,
            });
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

    // nearest_neighbor uses PointDistance::distance_2, which now computes the
    // true point-to-linestring distance, so this returns the geometrically
    // nearest netelement even when the query point is inside multiple bboxes.
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

    /// Regression test: when the query point falls inside multiple bounding boxes,
    /// the R-tree must still return the geometrically nearest linestring, not an
    /// arbitrary one with bbox-distance = 0.
    ///
    /// Setup (approximate layout, not to scale):
    ///
    /// ```text
    ///  y=51.0 ─────────────── A[0]────────────────── A[1]
    ///                                 ^ query     ↑ B[0]
    ///  y=50.9 ────────────────────────────────────────────
    ///                                             ↓ B[1]
    /// ```
    ///
    /// Both netelements' bboxes contain the query point, so the old bbox-based
    /// implementation returns the wrong result.  The query point is 0.1° above
    /// netelement B but only 0.01° below netelement A, so A is nearest.
    #[test]
    fn test_find_nearest_with_overlapping_bboxes() {
        // Netelement A: horizontal line at y = 51.0, x from 4.0 to 6.0
        //   bbox: x=[4.0, 6.0], y=[51.0, 51.0]  (degenerate in y)
        let ls_a = LineString::from(vec![Coord { x: 4.0, y: 51.0 }, Coord { x: 6.0, y: 51.0 }]);
        let ne_a = Netelement::new("A".to_string(), ls_a, "EPSG:4326".to_string()).unwrap();

        // Netelement B: vertical line at x = 5.5, y from 50.8 to 51.2 (tall bbox)
        //   bbox: x=[5.5, 5.5], y=[50.8, 51.2]
        let ls_b = LineString::from(vec![Coord { x: 5.5, y: 50.8 }, Coord { x: 5.5, y: 51.2 }]);
        let ne_b = Netelement::new("B".to_string(), ls_b, "EPSG:4326".to_string()).unwrap();

        let index = NetworkIndex::new(vec![ne_a, ne_b]).unwrap();

        // Query point: (5.0, 50.99)
        //   Distance to A (y=51.0): 0.01° in y  → dist² ≈ 0.0001
        //   Distance to B (x=5.5):  0.50° in x  → dist² ≈ 0.25
        //   → A is clearly closer
        let query = Point::new(5.0, 50.99);
        let nearest = find_nearest_netelement(&query, &index).unwrap();
        assert_eq!(
            nearest, 0,
            "Point at (5.0, 50.99) should be nearest to netelement A (idx 0), not B (idx 1)"
        );
    }

    #[test]
    fn test_point_to_segment_dist_2_midpoint() {
        // Closest point on [a,b] to p is the midpoint
        let p = [0.0, 1.0];
        let a = [0.0, 0.0];
        let b = [2.0, 0.0];
        // Foot is (0,0), distance² = 1
        let d = point_to_segment_dist_2(&p, &a, &b);
        assert!((d - 1.0).abs() < 1e-12, "expected 1.0, got {}", d);
    }

    #[test]
    fn test_point_to_segment_dist_2_beyond_endpoint() {
        // Closest point is endpoint b
        let p = [3.0, 0.0];
        let a = [0.0, 0.0];
        let b = [2.0, 0.0];
        // Foot is b=(2,0), distance² = 1
        let d = point_to_segment_dist_2(&p, &a, &b);
        assert!((d - 1.0).abs() < 1e-12, "expected 1.0, got {}", d);
    }
}
