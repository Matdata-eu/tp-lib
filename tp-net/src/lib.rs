//! C#/.NET bindings for tp-lib-core.
//!
//! All public FFI symbols are declared with `extern "C"` and `#[no_mangle]`.
//! Data crosses the boundary as JSON in heap-allocated byte buffers; configs
//! are flat `#[repr(C)]` structs.

pub mod ffi;
pub mod marshal;

use ffi::{ByteBuffer, PathConfigFfi, ProjectionConfigFfi};
use marshal::{from_json_bytes, to_json_bytes};
use serde::Deserialize;
use tp_lib_core::{
    calculate_train_path, parse_gnss_csv_str, parse_gnss_geojson_str, parse_network_geojson_str,
    prepare_detections_from_loaded, project_gnss, project_onto_path, DetectionKind, PathConfig,
    RailwayNetwork, ResolvedAnchor, TrainPath,
};

const WGS84: &str = "EPSG:4326";
const CSV_LAT_COL: &str = "latitude";
const CSV_LON_COL: &str = "longitude";
const CSV_TIME_COL: &str = "timestamp";

/// Partial mirror of `tp_lib_core::PreparedDetections` used only to recover the
/// `anchors` for path calculation. Remaining fields (records, warnings) are
/// ignored on input.
#[derive(Deserialize)]
struct PreparedDetectionsInput {
    #[serde(default)]
    anchors: Vec<ResolvedAnchor>,
}

unsafe fn load_network(
    ptr: *const u8,
    len: i32,
) -> Option<(
    RailwayNetwork,
    Vec<tp_lib_core::NetRelation>,
    Vec<tp_lib_core::Netelement>,
)> {
    let bytes = std::slice::from_raw_parts(ptr, len.max(0) as usize);
    let text = std::str::from_utf8(bytes).ok()?;
    let (netelements, netrelations) = parse_network_geojson_str(text).ok()?;
    let net_clone = netelements.clone();
    let network = RailwayNetwork::new(netelements).ok()?;
    Some((network, netrelations, net_clone))
}

unsafe fn load_gnss(ptr: *const u8, len: i32) -> Option<Vec<tp_lib_core::GnssPosition>> {
    let bytes = std::slice::from_raw_parts(ptr, len.max(0) as usize);
    let text = std::str::from_utf8(bytes).ok()?;
    if text.trim_start().starts_with('{') {
        parse_gnss_geojson_str(text, WGS84).ok()
    } else {
        parse_gnss_csv_str(text, WGS84, CSV_LAT_COL, CSV_LON_COL, CSV_TIME_COL).ok()
    }
}

/// Project GNSS positions onto the nearest network segments.
///
/// # Safety
/// All pointers must reference valid UTF-8 byte slices of the indicated length.
#[no_mangle]
pub unsafe extern "C" fn tp_net_project_gnss(
    network_ptr: *const u8,
    network_len: i32,
    gnss_ptr: *const u8,
    gnss_len: i32,
    config: ProjectionConfigFfi,
) -> ByteBuffer {
    let Some((network, _, _)) = load_network(network_ptr, network_len) else {
        return ByteBuffer::null_error();
    };
    let Some(gnss) = load_gnss(gnss_ptr, gnss_len) else {
        return ByteBuffer::null_error();
    };
    let core_config: tp_lib_core::ProjectionConfig = config.into();
    match project_gnss(&gnss, &network, &core_config) {
        Ok(projected) => to_json_bytes(&projected),
        Err(_) => ByteBuffer::null_error(),
    }
}

/// Project GNSS positions onto a previously computed train path.
///
/// # Safety
/// All pointers must reference valid UTF-8 byte slices of the indicated length.
#[no_mangle]
pub unsafe extern "C" fn tp_net_project_onto_path(
    network_ptr: *const u8,
    network_len: i32,
    gnss_ptr: *const u8,
    gnss_len: i32,
    train_path_ptr: *const u8,
    train_path_len: i32,
    config: PathConfigFfi,
) -> ByteBuffer {
    let Some((_, _, netelements)) = load_network(network_ptr, network_len) else {
        return ByteBuffer::null_error();
    };
    let Some(gnss) = load_gnss(gnss_ptr, gnss_len) else {
        return ByteBuffer::null_error();
    };
    let Ok(train_path) = from_json_bytes::<TrainPath>(train_path_ptr, train_path_len) else {
        return ByteBuffer::null_error();
    };
    let core_config: PathConfig = config.into();
    match project_onto_path(&gnss, &train_path, &netelements, &core_config) {
        Ok(projected) => to_json_bytes(&projected),
        Err(_) => ByteBuffer::null_error(),
    }
}

/// Calculate a train path from GNSS positions and a railway network.
///
/// `prepared_detections_ptr` may be null (`prepared_detections_len == 0`).
/// When provided, the JSON must include an `anchors` array of [`ResolvedAnchor`].
///
/// # Safety
/// All non-null pointers must reference valid UTF-8 byte slices of the
/// indicated length.
#[no_mangle]
pub unsafe extern "C" fn tp_net_calculate_train_path(
    network_ptr: *const u8,
    network_len: i32,
    gnss_ptr: *const u8,
    gnss_len: i32,
    prepared_detections_ptr: *const u8,
    prepared_detections_len: i32,
    config: PathConfigFfi,
) -> ByteBuffer {
    let Some((_, netrelations, netelements)) = load_network(network_ptr, network_len) else {
        return ByteBuffer::null_error();
    };
    let Some(gnss) = load_gnss(gnss_ptr, gnss_len) else {
        return ByteBuffer::null_error();
    };
    let mut core_config: PathConfig = config.into();
    if !prepared_detections_ptr.is_null() && prepared_detections_len > 0 {
        match from_json_bytes::<PreparedDetectionsInput>(
            prepared_detections_ptr,
            prepared_detections_len,
        ) {
            Ok(pd) => core_config.anchors = pd.anchors,
            Err(_) => return ByteBuffer::null_error(),
        }
    }
    match calculate_train_path(&gnss, &netelements, &netrelations, &core_config) {
        Ok(result) => to_json_bytes(&result),
        Err(_) => ByteBuffer::null_error(),
    }
}

/// Validate, time-filter and resolve detections into [`ResolvedAnchor`]s for
/// path calculation.
///
/// `kind_is_linear == 0` ⇒ `Punctual`; non-zero ⇒ `Linear`.
///
/// # Safety
/// All pointers must reference valid UTF-8 byte slices of the indicated length.
#[no_mangle]
pub unsafe extern "C" fn tp_net_prepare_detections(
    network_ptr: *const u8,
    network_len: i32,
    gnss_ptr: *const u8,
    gnss_len: i32,
    detections_geojson_ptr: *const u8,
    detections_geojson_len: i32,
    kind_is_linear: u8,
    cutoff_distance_meters: f64,
) -> ByteBuffer {
    let Some((_, _, netelements)) = load_network(network_ptr, network_len) else {
        return ByteBuffer::null_error();
    };
    let Some(gnss) = load_gnss(gnss_ptr, gnss_len) else {
        return ByteBuffer::null_error();
    };
    let det_bytes = std::slice::from_raw_parts(
        detections_geojson_ptr,
        detections_geojson_len.max(0) as usize,
    );
    let Ok(det_text) = std::str::from_utf8(det_bytes) else {
        return ByteBuffer::null_error();
    };
    let kind = if kind_is_linear != 0 {
        DetectionKind::Linear
    } else {
        DetectionKind::Punctual
    };
    let detections =
        match tp_lib_core::io::geojson::detections::load_str(det_text, "<memory>", kind) {
            Ok(d) => d,
            Err(_) => return ByteBuffer::null_error(),
        };
    let prepared = match prepare_detections_from_loaded(
        detections,
        &gnss,
        &netelements,
        cutoff_distance_meters,
    ) {
        Ok(p) => p,
        Err(_) => return ByteBuffer::null_error(),
    };
    #[derive(serde::Serialize)]
    struct PreparedDetectionsDto<'a> {
        anchors: &'a [ResolvedAnchor],
        records: &'a [tp_lib_core::DetectionRecord],
        warnings: &'a [String],
    }
    let dto = PreparedDetectionsDto {
        anchors: &prepared.anchors,
        records: &prepared.records,
        warnings: &prepared.warnings,
    };
    to_json_bytes(&dto)
}
