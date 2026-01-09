//! Projection engine for GNSS positions onto railway netelements

pub mod geom;
pub mod spatial;

pub use geom::{calculate_measure_along_linestring, project_point_onto_linestring};
pub use spatial::{find_nearest_netelement, NetworkIndex};
