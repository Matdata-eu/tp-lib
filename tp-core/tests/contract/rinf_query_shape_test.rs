//! T013 - Contract assertions for RINF SPARQL query shape and relation mapping.

use tp_lib_core::{build_netelements_query, build_netrelations_query};

#[test]
fn netelements_query_targets_linear_elements_with_polygon_filter() {
    let polygon = "POLYGON((4.0 50.0, 4.1 50.0, 4.1 50.1, 4.0 50.1, 4.0 50.0))";
    let q = build_netelements_query(polygon);
    assert!(q.contains("PREFIX era:"), "missing era prefix: {q}");
    assert!(q.contains("PREFIX gsp:"), "missing gsp prefix: {q}");
    assert!(q.contains("era:LinearElement"), "must select LinearElement");
    assert!(q.contains("gsp:hasGeometry"), "must traverse hasGeometry");
    assert!(q.contains("geof:sfIntersects"), "must intersect polygon");
    assert!(q.contains(polygon), "must embed polygon WKT literal");
    assert!(q.contains("gsp:wktLiteral"), "polygon must be wktLiteral");
}

#[test]
fn netrelations_query_filters_by_seed_iris_and_navigability() {
    let seeds = vec![
        "http://data.europa.eu/949/functionalInfrastructure/netElements/A".to_string(),
        "http://data.europa.eu/949/functionalInfrastructure/netElements/B".to_string(),
    ];
    let q = build_netrelations_query(&seeds);
    assert!(q.contains("era:NetRelation"), "must select NetRelation");
    assert!(q.contains("era:elementA"), "must include elementA");
    assert!(q.contains("era:elementB"), "must include elementB");
    assert!(q.contains("era:navigability"), "must include navigability");
    assert!(q.contains("VALUES ?seed_element"), "must use VALUES clause");
    for iri in &seeds {
        assert!(q.contains(iri), "must embed seed IRI {iri}");
    }
}

#[test]
fn netrelations_query_handles_empty_seed_list() {
    let q = build_netrelations_query(&[]);
    assert!(
        q.contains("VALUES ?seed_element"),
        "must still have VALUES clause"
    );
}
