//! Input/output module for CSV, GeoJSON, and Arrow formats

pub mod arrow;
pub mod csv;
pub mod geojson;
pub mod rinf;

pub use csv::{
    parse_gnss_csv, parse_gnss_csv_str, parse_trainpath_csv, write_csv, write_trainpath_csv,
};
pub use geojson::{
    parse_gnss_geojson, parse_gnss_geojson_str, parse_netrelations_geojson, parse_network_geojson,
    parse_network_geojson_str, parse_trainpath_geojson, write_geojson, write_trainpath_geojson,
};
pub use rinf::{build_netelements_query, build_netrelations_query, SparqlClient, UreqSparqlClient};
