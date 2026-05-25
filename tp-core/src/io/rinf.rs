//! ERA RINF SPARQL retrieval module (feature 006).
//!
//! Provides:
//! - The [`SparqlClient`] trait so production code can hit the real endpoint
//!   via [`UreqSparqlClient`] while tests inject deterministic fixtures.
//! - A tiny inline WKT `LINESTRING` parser (avoids pulling a wkt crate).
//! - Builders for the two SPARQL queries documented in
//!   `specs/006-download-rinf-topology/research.md`.
//! - Row-to-core-model mappers that produce [`Netelement`] /
//!   [`NetRelation`] instances ready for downstream workflows.

use std::time::Duration;

use chrono::NaiveDate;
use geo::{LineString, Point};
use serde_json::Value;

use crate::errors::ProjectionError;
use crate::models::{
    NetRelation, Netelement, RinfNavigability, RinfNetelementRow, RinfNetrelationRow,
};

/// Pluggable SPARQL transport — production uses ureq, tests use mocks.
pub trait SparqlClient: Send + Sync {
    /// Execute a SPARQL query and return parsed JSON (SPARQL-Results 1.1 shape).
    fn query(&self, endpoint_url: &str, sparql: &str) -> Result<Value, ProjectionError>;
}

/// Default blocking SPARQL client backed by [`ureq`].
pub struct UreqSparqlClient {
    timeout: Duration,
}

impl Default for UreqSparqlClient {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(60),
        }
    }
}

impl UreqSparqlClient {
    pub fn new(timeout: Duration) -> Self {
        Self { timeout }
    }
}

impl SparqlClient for UreqSparqlClient {
    fn query(&self, endpoint_url: &str, sparql: &str) -> Result<Value, ProjectionError> {
        let agent = ureq::AgentBuilder::new()
            .timeout(self.timeout)
            .user_agent("tp-lib/006-rinf-retrieval")
            .build();
        let response = agent
            .post(endpoint_url)
            .set("Accept", "application/sparql-results+json")
            .set("Content-Type", "application/sparql-query")
            .send_string(sparql)
            .map_err(|e| ProjectionError::RinfRetrievalFailed(format!("HTTP error: {e}")))?;
        let json: Value = response
            .into_json()
            .map_err(|e| ProjectionError::RinfRetrievalFailed(format!("JSON parse error: {e}")))?;
        Ok(json)
    }
}

/// Build the netelements SPARQL query for a given closed WGS84 polygon WKT.
pub fn build_netelements_query(polygon_wkt: &str) -> String {
    format!(
        r#"PREFIX era: <http://data.europa.eu/949/>
PREFIX gsp: <http://www.opengis.net/ont/geosparql#>
PREFIX geof: <http://www.opengis.net/def/function/geosparql/>
PREFIX time: <http://www.w3.org/2006/time#>
PREFIX xsd: <http://www.w3.org/2001/XMLSchema#>

SELECT ?netelement ?netelement_wkt ?valid_from_date ?valid_to_date
WHERE {{
  ?netelement a era:LinearElement ;
              era:validity/time:hasBeginning/time:inXSDDate ?valid_from_date ;
              gsp:hasGeometry/gsp:asWKT ?netelement_wkt .
  FILTER(geof:sfIntersects(
    ?netelement_wkt,
    "{polygon}"^^gsp:wktLiteral
  ))
    OPTIONAL {{
      ?netelement era:validity/time:hasEnd/time:inXSDDate ?valid_to_date .
      FILTER (xsd:date(now()) >= ?valid_to_date)
    }}
    FILTER (xsd:date(now()) >= ?valid_from_date && !BOUND(?valid_to_date))
}}"#,
        polygon = polygon_wkt
    )
}

/// Build the netrelations SPARQL query for a list of seed element IRIs.
pub fn build_netrelations_query(seed_iris: &[String]) -> String {
    let values = seed_iris
        .iter()
        .map(|iri| format!("<{}>", iri))
        .collect::<Vec<_>>()
        .join(" ");
    format!(
        r#"PREFIX era: <http://data.europa.eu/949/>
PREFIX xsd: <http://www.w3.org/2001/XMLSchema#>
PREFIX time: <http://www.w3.org/2006/time#>

SELECT ?netrelation ?netelementA ?netelementB ?isOnOriginOfElementA ?isOnOriginOfElementB ?navigability ?valid_from_date ?valid_to_date
WHERE {{
  VALUES ?seed_element {{ {values} }}
  {{
    BIND(?seed_element AS ?netelementA)
    ?netrelation a era:NetRelation ;
                 era:elementA ?netelementA ;
                 era:elementB ?netelementB ;
                 era:isOnOriginOfElementA ?isOnOriginOfElementA ;
                 era:isOnOriginOfElementB ?isOnOriginOfElementB ;
                 era:navigability ?navigability ;
                 era:validity/time:hasBeginning/time:inXSDDate ?valid_from_date .
    OPTIONAL {{
      ?netrelation era:validity/time:hasEnd/time:inXSDDate ?valid_to_date .
      FILTER (xsd:date(now()) >= ?valid_to_date)
    }}
    FILTER (xsd:date(now()) >= ?valid_from_date && !BOUND(?valid_to_date))
  }}
  UNION
  {{
    BIND(?seed_element AS ?netelementB)
    ?netrelation a era:NetRelation ;
                 era:elementA ?netelementA ;
                 era:elementB ?netelementB ;
                 era:isOnOriginOfElementA ?isOnOriginOfElementA ;
                 era:isOnOriginOfElementB ?isOnOriginOfElementB ;
                 era:navigability ?navigability ;
                 era:validity/time:hasBeginning/time:inXSDDate ?valid_from_date .
    OPTIONAL {{
      ?netrelation era:validity/time:hasEnd/time:inXSDDate ?valid_to_date .
      FILTER (xsd:date(now()) >= ?valid_to_date)
    }}
    FILTER (xsd:date(now()) >= ?valid_from_date && !BOUND(?valid_to_date))
  }}
}}"#
    )
}

/// Parse a `LINESTRING(...)` WKT into a [`LineString<f64>`].
///
/// Inline parser to avoid pulling a wkt-crate dependency. Accepts upper or
/// lower-case keyword and any whitespace between tokens. Does NOT handle Z/M.
pub fn parse_wkt_linestring(wkt: &str) -> Result<LineString<f64>, ProjectionError> {
    let trimmed = wkt.trim();
    let upper = trimmed.to_ascii_uppercase();
    let body = if let Some(rest) = upper.strip_prefix("LINESTRING") {
        rest
    } else {
        return Err(ProjectionError::RinfIncompleteTopology(format!(
            "WKT is not a LINESTRING: {trimmed}"
        )));
    };
    // Use the original (un-uppercased) string for coordinate parsing — but
    // since `body` was sliced from `upper`, recompute the same slice on the
    // original. Easiest: just lowercase numbers are identical in either case.
    let body = body.trim();
    let inner = body
        .strip_prefix('(')
        .and_then(|s| s.strip_suffix(')'))
        .ok_or_else(|| {
            ProjectionError::RinfIncompleteTopology(format!(
                "Malformed LINESTRING parentheses: {trimmed}"
            ))
        })?;
    let mut coords: Vec<(f64, f64)> = Vec::new();
    for pair in inner.split(',') {
        let mut nums = pair.split_whitespace();
        let lon = nums
            .next()
            .and_then(|s| s.parse::<f64>().ok())
            .ok_or_else(|| {
                ProjectionError::RinfIncompleteTopology(format!(
                    "Missing or invalid longitude in WKT: {trimmed}"
                ))
            })?;
        let lat = nums
            .next()
            .and_then(|s| s.parse::<f64>().ok())
            .ok_or_else(|| {
                ProjectionError::RinfIncompleteTopology(format!(
                    "Missing or invalid latitude in WKT: {trimmed}"
                ))
            })?;
        coords.push((lon, lat));
    }
    if coords.len() < 2 {
        return Err(ProjectionError::RinfIncompleteTopology(format!(
            "LINESTRING needs >=2 points: {trimmed}"
        )));
    }
    Ok(LineString::from(coords))
}

/// Approximate length of a WGS84 LineString in meters (great-circle, equirectangular).
pub fn linestring_length_meters(ls: &LineString<f64>) -> f64 {
    let pts: Vec<Point<f64>> = ls.points().collect();
    let mut total = 0.0;
    for w in pts.windows(2) {
        let (a, b) = (w[0], w[1]);
        let lat_mid = (a.y() + b.y()) / 2.0;
        let dx = (b.x() - a.x()) * 111_320.0 * lat_mid.to_radians().cos();
        let dy = (b.y() - a.y()) * 111_320.0;
        total += (dx * dx + dy * dy).sqrt();
    }
    total
}

/// Extract the IRI tail to use as a stable id.
fn iri_to_id(iri: &str) -> String {
    iri.rsplit(['/', '#']).next().unwrap_or(iri).to_string()
}

fn binding_value<'a>(row: &'a Value, key: &str) -> Option<&'a str> {
    row.get(key)?.get("value")?.as_str()
}

fn parse_bool(s: &str) -> bool {
    matches!(s.trim().to_ascii_lowercase().as_str(), "true" | "1")
}

fn parse_navigability(iri_or_label: &str) -> RinfNavigability {
    let tail = iri_to_id(iri_or_label).to_ascii_lowercase();
    match tail.as_str() {
        "both" => RinfNavigability::Both,
        "ab" | "atob" | "anbi" => RinfNavigability::AB,
        "ba" | "btoa" | "binba" => RinfNavigability::BA,
        "none" | "non-navigable" => RinfNavigability::None,
        _ => RinfNavigability::Both,
    }
}

/// Parse the SPARQL-JSON response for the netelements query.
pub fn parse_netelements_response(json: &Value) -> Result<Vec<RinfNetelementRow>, ProjectionError> {
    let bindings = json
        .get("results")
        .and_then(|r| r.get("bindings"))
        .and_then(|b| b.as_array())
        .ok_or_else(|| {
            ProjectionError::RinfRetrievalFailed(
                "Netelements response missing results.bindings array".to_string(),
            )
        })?;

    let mut out = Vec::with_capacity(bindings.len());
    for row in bindings {
        let iri = binding_value(row, "netelement").ok_or_else(|| {
            ProjectionError::RinfRetrievalFailed("Missing ?netelement binding".to_string())
        })?;
        let wkt = binding_value(row, "netelement_wkt").ok_or_else(|| {
            ProjectionError::RinfRetrievalFailed("Missing ?netelement_wkt binding".to_string())
        })?;
        let ls = parse_wkt_linestring(wkt)?;
        let count = ls.coords().count();
        let length = linestring_length_meters(&ls);
        out.push(RinfNetelementRow {
            netelement_iri: iri.to_string(),
            netelement_id: iri_to_id(iri),
            wkt: wkt.to_string(),
            geometry_point_count: count,
            length_meters: length,
        });
    }
    Ok(out)
}

/// Parse the SPARQL-JSON response for the netrelations query.
pub fn parse_netrelations_response(
    json: &Value,
) -> Result<Vec<RinfNetrelationRow>, ProjectionError> {
    let bindings = json
        .get("results")
        .and_then(|r| r.get("bindings"))
        .and_then(|b| b.as_array())
        .ok_or_else(|| {
            ProjectionError::RinfRetrievalFailed(
                "Netrelations response missing results.bindings array".to_string(),
            )
        })?;
    let today = chrono::Utc::now().date_naive();
    let mut out = Vec::with_capacity(bindings.len());
    for row in bindings {
        let iri = binding_value(row, "netrelation").ok_or_else(|| {
            ProjectionError::RinfRetrievalFailed("Missing ?netrelation binding".to_string())
        })?;
        let a = binding_value(row, "netelementA").ok_or_else(|| {
            ProjectionError::RinfRetrievalFailed("Missing ?netelementA binding".to_string())
        })?;
        let b = binding_value(row, "netelementB").ok_or_else(|| {
            ProjectionError::RinfRetrievalFailed("Missing ?netelementB binding".to_string())
        })?;
        let on_a = binding_value(row, "isOnOriginOfElementA")
            .map(parse_bool)
            .unwrap_or(false);
        let on_b = binding_value(row, "isOnOriginOfElementB")
            .map(parse_bool)
            .unwrap_or(false);
        let nav = binding_value(row, "navigability")
            .map(parse_navigability)
            .unwrap_or(RinfNavigability::Both);
        let valid_on_date = binding_value(row, "valid_from_date")
            .and_then(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d").ok())
            .unwrap_or(today);
        out.push(RinfNetrelationRow {
            netrelation_iri: iri.to_string(),
            element_a_id: iri_to_id(a),
            element_b_id: iri_to_id(b),
            is_on_origin_of_element_a: on_a,
            is_on_origin_of_element_b: on_b,
            navigability: nav,
            valid_on_date,
        });
    }
    Ok(out)
}

/// Map parsed netelement rows to core [`Netelement`] structs.
///
/// Returns the netelements plus a parallel `(id, length_meters, point_count)`
/// vector used by the validator to detect coarse geometries.
#[allow(clippy::type_complexity)]
pub fn map_netelements_to_core(
    rows: &[RinfNetelementRow],
) -> Result<(Vec<Netelement>, Vec<(String, f64, usize)>), ProjectionError> {
    let mut nes = Vec::with_capacity(rows.len());
    let mut lengths = Vec::with_capacity(rows.len());
    for r in rows {
        let ls = parse_wkt_linestring(&r.wkt)?;
        let length = linestring_length_meters(&ls);
        let count = ls.coords().count();
        let ne = Netelement::new(r.netelement_id.clone(), ls, "EPSG:4326".to_string())?;
        lengths.push((r.netelement_id.clone(), length, count));
        nes.push(ne);
    }
    Ok((nes, lengths))
}

/// Map parsed netrelation rows to core [`NetRelation`] structs.
///
/// Drops rows whose endpoints don't reference loaded netelements.
pub fn map_netrelations_to_core(
    rows: &[RinfNetrelationRow],
    netelements: &[Netelement],
) -> Result<Vec<NetRelation>, ProjectionError> {
    use std::collections::HashSet;
    let known: HashSet<&str> = netelements.iter().map(|n| n.id.as_str()).collect();
    let mut out = Vec::with_capacity(rows.len());
    for r in rows {
        if !known.contains(r.element_a_id.as_str()) || !known.contains(r.element_b_id.as_str()) {
            continue;
        }
        if r.element_a_id == r.element_b_id {
            continue;
        }
        let (fwd, bwd) = match r.navigability {
            RinfNavigability::Both => (true, true),
            RinfNavigability::AB => (true, false),
            RinfNavigability::BA => (false, true),
            RinfNavigability::None => (false, false),
        };
        let pos_a: u8 = if r.is_on_origin_of_element_a { 0 } else { 1 };
        let pos_b: u8 = if r.is_on_origin_of_element_b { 0 } else { 1 };
        let id = iri_to_id(&r.netrelation_iri);
        let nr = NetRelation::new(
            id,
            r.element_a_id.clone(),
            r.element_b_id.clone(),
            pos_a,
            pos_b,
            fwd,
            bwd,
        )?;
        out.push(nr);
    }
    Ok(out)
}

/// High-level helper: fetch + parse netelements for a search polygon.
pub fn fetch_netelements(
    client: &dyn SparqlClient,
    endpoint_url: &str,
    polygon_wkt: &str,
) -> Result<Vec<RinfNetelementRow>, ProjectionError> {
    let query = build_netelements_query(polygon_wkt);
    let json = client.query(endpoint_url, &query)?;
    parse_netelements_response(&json)
}

/// High-level helper: fetch + parse netrelations for the given seed IRIs.
pub fn fetch_netrelations(
    client: &dyn SparqlClient,
    endpoint_url: &str,
    seed_iris: &[String],
) -> Result<Vec<RinfNetrelationRow>, ProjectionError> {
    if seed_iris.is_empty() {
        return Ok(Vec::new());
    }
    let query = build_netrelations_query(seed_iris);
    let json = client.query(endpoint_url, &query)?;
    parse_netrelations_response(&json)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wkt_linestring_parses_basic() {
        let ls = parse_wkt_linestring("LINESTRING(11.0 60.0, 11.1 60.1)").unwrap();
        assert_eq!(ls.coords().count(), 2);
    }

    #[test]
    fn wkt_linestring_rejects_single_point() {
        assert!(parse_wkt_linestring("LINESTRING(11.0 60.0)").is_err());
    }

    #[test]
    fn iri_tail_is_used_as_id() {
        assert_eq!(
            iri_to_id("http://data.europa.eu/949/linearElement/SMOKE-A"),
            "SMOKE-A"
        );
    }

    #[test]
    fn netelements_query_contains_polygon() {
        let q = build_netelements_query("POLYGON((1 2, 3 4))");
        assert!(q.contains("POLYGON((1 2, 3 4))"));
        assert!(q.contains("sfIntersects"));
    }
}
