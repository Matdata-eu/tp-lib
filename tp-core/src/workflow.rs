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

    // Only treat the topology as incomplete when *every* netelement is coarse.
    // A few 2-point segments are expected (straight sections, short links) and
    // are not a reason to reject the whole bundle.
    if !coarse_ids.is_empty() && coarse_ids.len() == netelements.len() {
        return TopologyValidationReport {
            status: TopologyValidationStatus::IncompleteTopology,
            netelement_count: netelements.len(),
            netrelation_count: netrelations.len(),
            coarse_geometry_ids: coarse_ids,
            uncovered_gnss_indices: Vec::new(),
            message: "Retrieved topology contains only coarse netelement geometries".to_string(),
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
        coarse_geometry_ids: coarse_ids,
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

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{DateTime, FixedOffset};
    use geo::LineString;
    use std::collections::HashMap;

    fn gnss(lat: f64, lon: f64) -> GnssPosition {
        let ts: DateTime<FixedOffset> =
            DateTime::parse_from_rfc3339("2026-05-13T08:00:00+00:00").unwrap();
        GnssPosition {
            latitude: lat,
            longitude: lon,
            timestamp: ts,
            crs: "EPSG:4326".to_string(),
            metadata: HashMap::new(),
            heading: None,
            distance: None,
        }
    }

    fn ne(id: &str, wkt: &str) -> Netelement {
        Netelement::new(
            id.to_string(),
            crate::io::rinf::parse_wkt_linestring(wkt).unwrap(),
            "EPSG:4326".to_string(),
        )
        .unwrap()
    }

    #[test]
    fn retrieval_config_builder_methods_override_defaults() {
        let cfg = RetrievalConfig::default()
            .with_endpoint("https://example.invalid/sparql")
            .with_buffer_meters(250.0);
        assert_eq!(cfg.endpoint_url, "https://example.invalid/sparql");
        assert_eq!(cfg.buffer_meters, 250.0);
    }

    #[test]
    fn build_retrieval_area_skips_non_finite_and_out_of_range_points() {
        let positions = vec![
            gnss(f64::NAN, 4.0),
            gnss(200.0, 4.0),
            gnss(50.0, 4.0),
            gnss(50.1, 4.2),
        ];

        let area = build_retrieval_area(&positions, 100.0).unwrap();
        assert!(area.min_latitude < 50.0);
        assert!(area.max_latitude > 50.1);
        assert!(area.min_longitude < 4.0);
        assert!(area.max_longitude > 4.2);
    }

    #[test]
    fn build_retrieval_area_rejects_when_all_points_invalid() {
        let positions = vec![gnss(f64::NAN, 4.0), gnss(95.0, 4.0), gnss(40.0, 190.0)];
        let err = build_retrieval_area(&positions, 100.0).unwrap_err();
        assert!(err.to_string().contains("No usable WGS84 coordinates"));
    }

    #[test]
    fn uncovered_gnss_indices_returns_all_when_no_netelements() {
        let positions = vec![gnss(50.0, 4.0), gnss(50.1, 4.1)];
        let uncovered = uncovered_gnss_indices(&positions, &[]);
        assert_eq!(uncovered, vec![0, 1]);
    }

    #[test]
    fn uncovered_gnss_indices_marks_outside_points() {
        let positions = vec![gnss(50.0, 4.0), gnss(51.0, 5.0)];
        let netelements = vec![ne("NE-1", "LINESTRING(3.9 49.9, 4.2 50.2)")];
        let uncovered = uncovered_gnss_indices(&positions, &netelements);
        assert_eq!(uncovered, vec![1]);
    }

    #[test]
    fn validate_topology_returns_missing_coverage_for_empty_netelements() {
        let report = validate_topology(&[], &[], &[], &[gnss(50.0, 4.0)]);
        assert_eq!(report.status, TopologyValidationStatus::MissingCoverage);
        assert_eq!(report.uncovered_gnss_indices, vec![0]);
    }

    #[test]
    fn validate_topology_returns_incomplete_when_all_netelements_coarse() {
        let netelements = vec![ne("NE-1", "LINESTRING(4.0 50.0, 4.2 50.0)")];
        let report = validate_topology(
            &netelements,
            &[],
            &[("NE-1".to_string(), 20_000.0, 2)],
            &[gnss(50.0, 4.0)],
        );
        assert_eq!(report.status, TopologyValidationStatus::IncompleteTopology);
        assert_eq!(report.coarse_geometry_ids, vec!["NE-1".to_string()]);
    }

    #[test]
    fn validate_topology_returns_incomplete_when_no_netrelations() {
        let netelements = vec![Netelement::new(
            "NE-1".to_string(),
            LineString::from(vec![(4.0, 50.0), (4.0001, 50.0001), (4.0002, 50.0002)]),
            "EPSG:4326".to_string(),
        )
        .unwrap()];
        let report = validate_topology(&netelements, &[], &[("NE-1".to_string(), 100.0, 3)], &[]);
        assert_eq!(report.status, TopologyValidationStatus::IncompleteTopology);
    }
}
