//! Source selection and retrieval-area construction for automatic topology
//! retrieval (feature 006).
//!
//! Combines GNSS-derived bounding boxes, the RINF SPARQL client trait, and the
//! validation pipeline that produces a [`RetrievedTopology`] ready for use by
//! the existing path/projection/detection algorithms.

use chrono::Utc;

use crate::errors::ProjectionError;
use crate::io::rinf::{
    fetch_netelements, fetch_netrelations, map_netelements_to_core, map_netrelations_to_core,
    SparqlClient,
};
use crate::models::{
    AutoTopologyRequest, GnssPosition, NetRelation, Netelement, RetrievalArea, RetrievalOutcome,
    RetrievalStatus, RetrievedTopology, TopologySource, TopologyValidationReport,
    TopologyValidationStatus, WorkflowKind, COARSE_GEOMETRY_LENGTH_THRESHOLD_METERS,
    DEFAULT_RETRIEVAL_BUFFER_METERS, DEFAULT_RINF_ENDPOINT,
};

/// Retrieval configuration shared by the CLI and language bindings.
#[derive(Debug, Clone)]
pub struct RetrievalConfig {
    pub endpoint_url: String,
    pub buffer_meters: f64,
}

impl Default for RetrievalConfig {
    fn default() -> Self {
        Self {
            endpoint_url: DEFAULT_RINF_ENDPOINT.to_string(),
            buffer_meters: DEFAULT_RETRIEVAL_BUFFER_METERS,
        }
    }
}

impl RetrievalConfig {
    pub fn with_endpoint(mut self, endpoint: impl Into<String>) -> Self {
        self.endpoint_url = endpoint.into();
        self
    }

    pub fn with_buffer_meters(mut self, buffer_meters: f64) -> Self {
        self.buffer_meters = buffer_meters;
        self
    }
}

/// Build a 1 km-expanded (configurable) WGS84 axis-aligned search polygon from
/// GNSS positions.
///
/// Returns `Err(InvalidGnssInput)` if no usable WGS84 coordinates are present.
pub fn build_retrieval_area(
    positions: &[GnssPosition],
    buffer_meters: f64,
) -> Result<RetrievalArea, ProjectionError> {
    if positions.is_empty() {
        return Err(ProjectionError::InvalidGnssInput(
            "GNSS dataset is empty".to_string(),
        ));
    }

    let mut min_lon = f64::INFINITY;
    let mut max_lon = f64::NEG_INFINITY;
    let mut min_lat = f64::INFINITY;
    let mut max_lat = f64::NEG_INFINITY;
    let mut count = 0usize;

    for p in positions {
        let lat = p.latitude;
        let lon = p.longitude;
        if !lat.is_finite() || !lon.is_finite() {
            continue;
        }
        if !(-90.0..=90.0).contains(&lat) || !(-180.0..=180.0).contains(&lon) {
            continue;
        }
        min_lon = min_lon.min(lon);
        max_lon = max_lon.max(lon);
        min_lat = min_lat.min(lat);
        max_lat = max_lat.max(lat);
        count += 1;
    }

    if count == 0 {
        return Err(ProjectionError::InvalidGnssInput(
            "No usable WGS84 coordinates in GNSS dataset".to_string(),
        ));
    }

    let center_lat = (min_lat + max_lat) / 2.0;
    let lat_expand = buffer_meters / 111_320.0;
    let lon_expand = buffer_meters / (111_320.0 * center_lat.to_radians().cos().max(1e-6));

    let exp_min_lon = min_lon - lon_expand;
    let exp_max_lon = max_lon + lon_expand;
    let exp_min_lat = min_lat - lat_expand;
    let exp_max_lat = max_lat + lat_expand;

    let polygon_wkt = format!(
        "POLYGON(({lo1} {la1}, {lo2} {la1}, {lo2} {la2}, {lo1} {la2}, {lo1} {la1}))",
        lo1 = exp_min_lon,
        lo2 = exp_max_lon,
        la1 = exp_min_lat,
        la2 = exp_max_lat,
    );

    Ok(RetrievalArea {
        min_longitude: exp_min_lon,
        max_longitude: exp_max_lon,
        min_latitude: exp_min_lat,
        max_latitude: exp_max_lat,
        expansion_meters: buffer_meters,
        polygon_wkt,
        source_crs: "EPSG:4326".to_string(),
    })
}

/// Find indices of GNSS positions that fall outside the bounding box of the
/// retrieved netelements (used for `uncovered_gnss_indices` diagnostics).
pub fn uncovered_gnss_indices(
    positions: &[GnssPosition],
    netelements: &[Netelement],
) -> Vec<usize> {
    if netelements.is_empty() {
        return (0..positions.len()).collect();
    }
    let mut min_lon = f64::INFINITY;
    let mut max_lon = f64::NEG_INFINITY;
    let mut min_lat = f64::INFINITY;
    let mut max_lat = f64::NEG_INFINITY;
    for ne in netelements {
        for c in ne.geometry.coords() {
            min_lon = min_lon.min(c.x);
            max_lon = max_lon.max(c.x);
            min_lat = min_lat.min(c.y);
            max_lat = max_lat.max(c.y);
        }
    }
    positions
        .iter()
        .enumerate()
        .filter_map(|(i, p)| {
            let inside = p.longitude >= min_lon
                && p.longitude <= max_lon
                && p.latitude >= min_lat
                && p.latitude <= max_lat;
            if inside {
                None
            } else {
                Some(i)
            }
        })
        .collect()
}

/// Validate a topology bundle produced from RINF.
pub fn validate_topology(
    netelements: &[Netelement],
    netrelations: &[NetRelation],
    netelement_lengths: &[(String, f64, usize)],
    positions: &[GnssPosition],
) -> TopologyValidationReport {
    if netelements.is_empty() {
        return TopologyValidationReport {
            status: TopologyValidationStatus::MissingCoverage,
            netelement_count: 0,
            netrelation_count: 0,
            coarse_geometry_ids: Vec::new(),
            uncovered_gnss_indices: (0..positions.len()).collect(),
            message: "No netelements returned for the search area".to_string(),
        };
    }

    let coarse_ids: Vec<String> = netelement_lengths
        .iter()
        .filter_map(|(id, length, points)| {
            if *length > COARSE_GEOMETRY_LENGTH_THRESHOLD_METERS && *points <= 2 {
                Some(id.clone())
            } else {
                None
            }
        })
        .collect();

    if !coarse_ids.is_empty() {
        return TopologyValidationReport {
            status: TopologyValidationStatus::IncompleteTopology,
            netelement_count: netelements.len(),
            netrelation_count: netrelations.len(),
            coarse_geometry_ids: coarse_ids,
            uncovered_gnss_indices: Vec::new(),
            message: "Retrieved topology contains coarse netelement geometries".to_string(),
        };
    }

    if netrelations.is_empty() {
        return TopologyValidationReport {
            status: TopologyValidationStatus::IncompleteTopology,
            netelement_count: netelements.len(),
            netrelation_count: 0,
            coarse_geometry_ids: Vec::new(),
            uncovered_gnss_indices: Vec::new(),
            message: "Retrieved topology has zero netrelations".to_string(),
        };
    }

    let uncovered = uncovered_gnss_indices(positions, netelements);

    TopologyValidationReport {
        status: TopologyValidationStatus::Valid,
        netelement_count: netelements.len(),
        netrelation_count: netrelations.len(),
        coarse_geometry_ids: Vec::new(),
        uncovered_gnss_indices: uncovered,
        message: "Topology validated successfully".to_string(),
    }
}

/// Resolve the topology for a workflow. Returns the bundle plus an outcome
/// summary suitable for surfacing to callers.
///
/// If `supplied` is `Some`, it is used verbatim and no retrieval is performed.
/// Otherwise the SPARQL client is invoked with a polygon derived from `positions`.
pub fn resolve_topology(
    workflow_kind: WorkflowKind,
    positions: &[GnssPosition],
    supplied: Option<(Vec<Netelement>, Vec<NetRelation>)>,
    config: &RetrievalConfig,
    client: &dyn SparqlClient,
) -> Result<(RetrievedTopology, RetrievalOutcome), ProjectionError> {
    if let Some((nes, nrs)) = supplied {
        let area = RetrievalArea {
            min_longitude: 0.0,
            max_longitude: 0.0,
            min_latitude: 0.0,
            max_latitude: 0.0,
            expansion_meters: 0.0,
            polygon_wkt: String::new(),
            source_crs: "EPSG:4326".to_string(),
        };
        let report = TopologyValidationReport {
            status: TopologyValidationStatus::Valid,
            netelement_count: nes.len(),
            netrelation_count: nrs.len(),
            coarse_geometry_ids: Vec::new(),
            uncovered_gnss_indices: Vec::new(),
            message: "Supplied topology".to_string(),
        };
        let topology = RetrievedTopology {
            netelements: nes,
            netrelations: nrs,
            retrieval_area: area,
            endpoint_url: String::new(),
            retrieved_at: Utc::now(),
            validation_report: report,
        };
        return Ok((topology, RetrievalOutcome::supplied_success()));
    }

    let area = build_retrieval_area(positions, config.buffer_meters)?;

    let _request = AutoTopologyRequest {
        workflow_kind,
        supplied_topology_present: false,
        rinf_endpoint_url: config.endpoint_url.clone(),
        retrieval_area: Some(area.clone()),
        requested_at: Utc::now(),
    };

    let netelement_rows = fetch_netelements(client, &config.endpoint_url, &area.polygon_wkt)
        .map_err(|e| ProjectionError::RinfRetrievalFailed(e.to_string()))?;

    if netelement_rows.is_empty() {
        let report = TopologyValidationReport {
            status: TopologyValidationStatus::MissingCoverage,
            netelement_count: 0,
            netrelation_count: 0,
            coarse_geometry_ids: Vec::new(),
            uncovered_gnss_indices: (0..positions.len()).collect(),
            message: "No netelements returned for the search area".to_string(),
        };
        let outcome = RetrievalOutcome {
            source_used: TopologySource::EraRinf,
            status: RetrievalStatus::MissingCoverage,
            detail_message: report.message.clone(),
            diagnostic_area_wkt: Some(area.polygon_wkt.clone()),
            affected_gnss_indices: report.uncovered_gnss_indices.clone(),
        };
        let topology = RetrievedTopology {
            netelements: Vec::new(),
            netrelations: Vec::new(),
            retrieval_area: area,
            endpoint_url: config.endpoint_url.clone(),
            retrieved_at: Utc::now(),
            validation_report: report,
        };
        return Ok((topology, outcome));
    }

    let (netelements, lengths) = map_netelements_to_core(&netelement_rows)?;

    let seed_iris: Vec<String> = netelement_rows
        .iter()
        .map(|r| r.netelement_iri.clone())
        .collect();
    let netrelation_rows = fetch_netrelations(client, &config.endpoint_url, &seed_iris)
        .map_err(|e| ProjectionError::RinfRetrievalFailed(e.to_string()))?;
    let netrelations = map_netrelations_to_core(&netrelation_rows, &netelements)?;

    let report = validate_topology(&netelements, &netrelations, &lengths, positions);
    let status = match report.status {
        TopologyValidationStatus::Valid => RetrievalStatus::Success,
        TopologyValidationStatus::MissingCoverage => RetrievalStatus::MissingCoverage,
        TopologyValidationStatus::IncompleteTopology => RetrievalStatus::IncompleteTopology,
        TopologyValidationStatus::EndpointFailure => RetrievalStatus::EndpointFailure,
        TopologyValidationStatus::InvalidInput => RetrievalStatus::InvalidInput,
    };
    let outcome = RetrievalOutcome {
        source_used: TopologySource::EraRinf,
        status,
        detail_message: report.message.clone(),
        diagnostic_area_wkt: Some(area.polygon_wkt.clone()),
        affected_gnss_indices: report.uncovered_gnss_indices.clone(),
    };

    let topology = RetrievedTopology {
        netelements,
        netrelations,
        retrieval_area: area,
        endpoint_url: config.endpoint_url.clone(),
        retrieved_at: Utc::now(),
        validation_report: report,
    };

    Ok((topology, outcome))
}
