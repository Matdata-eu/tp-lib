//! Projection engine for GNSS positions onto railway netelements

pub mod geom;
pub mod spatial;

pub use geom::{project_point_onto_linestring, calculate_measure_along_linestring};
pub use spatial::{NetworkIndex, find_nearest_netelement};
