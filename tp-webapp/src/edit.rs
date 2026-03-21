//! Path editing operations: add and remove segments
//!
//! These functions implement the browser-side "click to add/remove segment" logic.
//! They operate on a [`TrainPath`] and return a new (edited) path.
//!
//! ## Snap Insertion Strategy
//!
//! When a netelement `N` is added to the path, we try to insert it at the
//! topologically correct position using geometric endpoint matching:
//!
//! 1. Check if `N`'s start-point matches the path's tail endpoint → **append**.
//! 2. Check if `N`'s end-point matches the path's head start-point → **prepend**.
//! 3. If both match (loop) → pick the end that is geometrically closer.
//! 4. If neither matches → **append** and the client renders a disconnected-marker
//!    style (the segment already carries `origin = Manual`).
//!
//! All manually-added segments are created with:
//! - `probability = 1.0`
//! - `origin = PathOrigin::Manual`
//! - `gnss_start_index = gnss_end_index` = the adjacent segment's end index (append) or
//!   start index (prepend), so GNSS ordering invariants are preserved
//! - `start_intrinsic = 0.0`, `end_intrinsic = 1.0`

use tp_lib_core::{AssociatedNetElement, PathOrigin, RailwayNetwork, TrainPath};

/// Build a manual [`AssociatedNetElement`] with fixed invariants.
fn manual_segment(
    netelement_id: String,
    gnss_start_index: usize,
    gnss_end_index: usize,
) -> AssociatedNetElement {
    let mut seg = AssociatedNetElement::new(
        netelement_id,
        1.0, // probability
        0.0, // start_intrinsic
        1.0, // end_intrinsic
        gnss_start_index,
        gnss_end_index,
    )
    .expect("invariants guarantee valid construction");
    seg.origin = PathOrigin::Manual;
    seg
}

/// Coordinate tolerance for endpoint matching (in degrees, ~1 cm at equatorial scale).
const ENDPOINT_TOLERANCE_DEG: f64 = 1e-7;

/// Returns `[first_x, first_y]` of a netelement's geometry, or `None` if not found.
fn first_coord(network: &RailwayNetwork, id: &str) -> Option<[f64; 2]> {
    network
        .netelements()
        .iter()
        .find(|ne| ne.id == id)
        .and_then(|ne| ne.geometry.0.first())
        .map(|c| [c.x, c.y])
}

/// Returns `[last_x, last_y]` of a netelement's geometry, or `None` if not found.
fn last_coord(network: &RailwayNetwork, id: &str) -> Option<[f64; 2]> {
    network
        .netelements()
        .iter()
        .find(|ne| ne.id == id)
        .and_then(|ne| ne.geometry.0.last())
        .map(|c| [c.x, c.y])
}

/// Squared Euclidean distance between two 2-D points (degree units).
fn dist2(a: [f64; 2], b: [f64; 2]) -> f64 {
    let dx = a[0] - b[0];
    let dy = a[1] - b[1];
    dx * dx + dy * dy
}

/// Returns `true` when two coordinates are within the snap tolerance.
fn near(a: [f64; 2], b: [f64; 2]) -> bool {
    dist2(a, b) <= ENDPOINT_TOLERANCE_DEG * ENDPOINT_TOLERANCE_DEG
}

/// Add a netelement to the path using geometric snap-insertion.
///
/// The function returns a new [`TrainPath`] with the segment inserted at the
/// most topologically appropriate position. When no exact match is found the
/// segment is appended; the browser is responsible for rendering it with a
/// disconnected-marker style.
///
/// # Arguments
///
/// - `netelement_id` – ID of the segment to add (must exist in `network`)
/// - `network` – the loaded railway network (read-only)
/// - `path` – the current train path
pub fn add_segment(netelement_id: &str, network: &RailwayNetwork, path: &TrainPath) -> TrainPath {
    // If the path is empty, just add the segment as the sole element.
    // There is no adjacent segment, so GNSS indices start at 0.
    if path.segments.is_empty() {
        let mut new_path = path.clone();
        new_path
            .segments
            .push(manual_segment(netelement_id.to_string(), 0, 0));
        return new_path;
    }

    // GNSS indices inherited from the adjacent segment at each potential insertion point.
    // • Prepend: inherit the current first segment's gnss_start_index (preserves ordering).
    // • Append:  inherit the current last segment's gnss_end_index (preserves ordering).
    let first_gnss = path.segments[0].gnss_start_index;
    let last_gnss = path.segments[path.segments.len() - 1].gnss_end_index;

    // Look up the new segment's endpoints from the network geometry.
    let new_head = first_coord(network, netelement_id);
    let new_tail = last_coord(network, netelement_id);

    // Look up the current path's head/tail endpoints.
    let path_head_id = &path.segments[0].netelement_id;
    let path_tail_id = &path.segments[path.segments.len() - 1].netelement_id;

    let path_head_start = first_coord(network, path_head_id);
    let path_tail_end = last_coord(network, path_tail_id);

    // Check connectivity:
    // • `can_append`: new segment's head attaches to path tail's end
    // • `can_prepend`: new segment's tail attaches to path head's start
    let can_append = match (new_head, path_tail_end) {
        (Some(nh), Some(pt)) => near(nh, pt),
        _ => false,
    };
    let can_prepend = match (new_tail, path_head_start) {
        (Some(nt), Some(ph)) => near(nt, ph),
        _ => false,
    };

    let mut new_path = path.clone();

    match (can_prepend, can_append) {
        (true, false) => {
            new_path.segments.insert(
                0,
                manual_segment(netelement_id.to_string(), first_gnss, first_gnss),
            );
        }
        (false, true) | (false, false) => {
            // Append (also the fallback / disconnected case)
            new_path.segments.push(manual_segment(
                netelement_id.to_string(),
                last_gnss,
                last_gnss,
            ));
        }
        (true, true) => {
            // Ambiguous: segment connects to both ends.
            // Pick the end where the new segment's midpoint is geometrically closest.
            let new_mid = match (new_head, new_tail) {
                (Some(h), Some(t)) => [(h[0] + t[0]) / 2.0, (h[1] + t[1]) / 2.0],
                _ => {
                    new_path.segments.push(manual_segment(
                        netelement_id.to_string(),
                        last_gnss,
                        last_gnss,
                    ));
                    return new_path;
                }
            };
            let d_head = path_head_start.map_or(f64::MAX, |h| dist2(new_mid, h));
            let d_tail = path_tail_end.map_or(f64::MAX, |t| dist2(new_mid, t));

            if d_head <= d_tail {
                new_path.segments.insert(
                    0,
                    manual_segment(netelement_id.to_string(), first_gnss, first_gnss),
                );
            } else {
                new_path.segments.push(manual_segment(
                    netelement_id.to_string(),
                    last_gnss,
                    last_gnss,
                ));
            }
        }
    }

    new_path
}

/// Remove a netelement from the path by ID.
///
/// If the segment appears multiple times, all occurrences are removed.
/// Returns the path unchanged when the segment is not present.
///
/// # Arguments
///
/// - `netelement_id` – ID of the segment to remove
/// - `path` – the current train path
pub fn remove_segment(netelement_id: &str, path: &TrainPath) -> TrainPath {
    let mut new_path = path.clone();
    new_path
        .segments
        .retain(|s| s.netelement_id != netelement_id);
    new_path
}

// ---------------------------------------------------------------------------
// Unit tests (T010 — written before implementation, must fail first)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use geo::LineString;
    use tp_lib_core::{AssociatedNetElement, Netelement, PathOrigin, RailwayNetwork, TrainPath};

    /// Build a minimal two-netelement network where NE_A and NE_B share a node.
    ///
    /// NE_A: (0.0, 0.0) → (1.0, 0.0)
    /// NE_B: (1.0, 0.0) → (2.0, 0.0)   ← NE_B's head == NE_A's tail
    fn simple_network() -> RailwayNetwork {
        let ne_a = Netelement::new(
            "NE_A".to_string(),
            LineString::from(vec![(0.0_f64, 0.0_f64), (1.0, 0.0)]),
            "EPSG:4326".to_string(),
        )
        .unwrap();
        let ne_b = Netelement::new(
            "NE_B".to_string(),
            LineString::from(vec![(1.0_f64, 0.0_f64), (2.0, 0.0)]),
            "EPSG:4326".to_string(),
        )
        .unwrap();
        let ne_c = Netelement::new(
            "NE_C".to_string(),
            LineString::from(vec![(5.0_f64, 5.0_f64), (6.0, 5.0)]),
            "EPSG:4326".to_string(),
        )
        .unwrap();
        RailwayNetwork::new(vec![ne_a, ne_b, ne_c]).unwrap()
    }

    fn path_with(ids: &[&str]) -> TrainPath {
        let segments = ids
            .iter()
            .map(|id| AssociatedNetElement::new(id.to_string(), 0.9, 0.0, 1.0, 0, 5).unwrap())
            .collect();
        TrainPath {
            segments,
            overall_probability: 0.9,
            calculated_at: None,
            metadata: None,
        }
    }

    // -----------------------------------------------------------------------
    // add_segment tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_add_segment_to_empty_path() {
        let network = simple_network();
        let path = path_with(&[]);
        let result = add_segment("NE_A", &network, &path);

        assert_eq!(result.segments.len(), 1);
        assert_eq!(result.segments[0].netelement_id, "NE_A");
        assert_eq!(result.segments[0].origin, PathOrigin::Manual);
        assert_eq!(result.segments[0].probability, 1.0);
    }

    #[test]
    fn test_add_segment_appends_when_topologically_adjacent_to_tail() {
        // NE_A ends at (1,0); NE_B starts at (1,0) → NE_B should be appended.
        let network = simple_network();
        let path = path_with(&["NE_A"]);
        let result = add_segment("NE_B", &network, &path);

        assert_eq!(result.segments.len(), 2);
        assert_eq!(result.segments[1].netelement_id, "NE_B");
    }

    #[test]
    fn test_add_segment_prepends_when_topologically_adjacent_to_head() {
        // NE_B starts at (1,0); NE_A ends at (1,0) → NE_A should be prepended.
        let network = simple_network();
        let path = path_with(&["NE_B"]);
        let result = add_segment("NE_A", &network, &path);

        assert_eq!(result.segments.len(), 2);
        assert_eq!(result.segments[0].netelement_id, "NE_A");
    }

    #[test]
    fn test_add_segment_disconnected_appended_to_nearest_end() {
        // NE_C is far from NE_A — should be appended (disconnected case).
        let network = simple_network();
        let path = path_with(&["NE_A"]);
        let result = add_segment("NE_C", &network, &path);

        assert_eq!(result.segments.len(), 2);
        assert_eq!(result.segments[1].netelement_id, "NE_C");
        assert_eq!(result.segments[1].origin, PathOrigin::Manual);
    }

    // -----------------------------------------------------------------------
    // remove_segment tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_remove_segment_present() {
        let network = simple_network();
        let _ = network; // satisfy unused warning
        let path = path_with(&["NE_A", "NE_B"]);
        let result = remove_segment("NE_A", &path);

        assert_eq!(result.segments.len(), 1);
        assert_eq!(result.segments[0].netelement_id, "NE_B");
    }

    #[test]
    fn test_remove_segment_not_present_returns_unchanged() {
        let path = path_with(&["NE_A", "NE_B"]);
        let result = remove_segment("NE_C", &path);

        assert_eq!(result.segments.len(), 2);
    }

    #[test]
    fn test_remove_segment_all_duplicates() {
        let path = path_with(&["NE_A", "NE_B", "NE_A"]);
        let result = remove_segment("NE_A", &path);

        assert_eq!(result.segments.len(), 1);
        assert_eq!(result.segments[0].netelement_id, "NE_B");
    }

    #[test]
    fn test_remove_segment_to_empty_path() {
        let path = path_with(&["NE_A"]);
        let result = remove_segment("NE_A", &path);

        assert!(result.segments.is_empty());
    }
}
