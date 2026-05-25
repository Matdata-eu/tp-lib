//! Data models for GNSS positioning and railway network

pub mod associated_net_element;
pub mod detection;
pub mod detection_record;
pub mod gnss;
pub mod gnss_net_element_link;
pub mod netelement;
pub mod netrelation;
pub mod path_metadata;
pub mod path_origin;
pub mod projected_position;
pub mod retrieval;
pub mod train_path;

pub use associated_net_element::AssociatedNetElement;
pub use detection::{
    Detection, GeographicLocation, LinearDetection, PunctualDetection, ResolvedAnchor,
    TopologicalLocation,
};
pub use detection_record::{
    DetectionKind, DetectionRecord, DetectionStatus, DiscardReason, TimestampOrRange,
};
pub use gnss::GnssPosition;
pub use gnss_net_element_link::GnssNetElementLink;
pub use netelement::Netelement;
pub use netrelation::NetRelation;
pub use path_metadata::{PathDiagnosticInfo, PathMetadata, SegmentDiagnostic};
pub use path_origin::PathOrigin;
pub use projected_position::ProjectedPosition;
pub use retrieval::{
    AutoTopologyRequest, RetrievalArea, RetrievalOutcome, RetrievalStatus, RetrievedTopology,
    RinfNavigability, RinfNetelementRow, RinfNetrelationRow, TopologySource,
    TopologyValidationReport, TopologyValidationStatus, WorkflowKind,
    COARSE_GEOMETRY_LENGTH_THRESHOLD_METERS, DEFAULT_RETRIEVAL_BUFFER_METERS,
    DEFAULT_RINF_ENDPOINT,
};
pub use train_path::TrainPath;
