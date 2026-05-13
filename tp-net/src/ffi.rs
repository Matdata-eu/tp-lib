//! FFI primitives: heap byte buffers and flat `#[repr(C)]` config structs.
//!
//! All data crossing the boundary is either a raw byte slice (JSON text) or a
//! flat config struct. Strings/collections are never passed in `#[repr(C)]`.

/// Heap-allocated byte buffer owned by the Rust side; freed by the caller via
/// [`tp_net_free_byte_buffer`].
///
/// `len == cap` after allocation. A null `ptr` paired with `len == -1`
/// signals an FFI error (the C# layer raises a generic exception).
#[repr(C)]
pub struct ByteBuffer {
    pub ptr: *mut u8,
    pub len: i32,
    pub cap: i32,
}

impl ByteBuffer {
    pub(crate) fn from_vec(mut v: Vec<u8>) -> Self {
        v.shrink_to_fit();
        let len = v.len() as i32;
        let cap = v.capacity() as i32;
        let ptr = v.as_mut_ptr();
        std::mem::forget(v);
        Self { ptr, len, cap }
    }

    pub(crate) fn null_error() -> Self {
        Self {
            ptr: std::ptr::null_mut(),
            len: -1,
            cap: 0,
        }
    }
}

/// Free a [`ByteBuffer`] previously returned by any `tp_net_*` function.
///
/// # Safety
/// The buffer must have been produced by this library and not yet freed.
#[no_mangle]
pub unsafe extern "C" fn tp_net_free_byte_buffer(buf: ByteBuffer) {
    if !buf.ptr.is_null() && buf.cap > 0 {
        let _ = Vec::from_raw_parts(buf.ptr, buf.len.max(0) as usize, buf.cap as usize);
    }
}

/// Flat mirror of `tp_lib_core::ProjectionConfig` for `#[repr(C)]` transport.
///
/// `max_search_radius_meters` is reserved for the public contract but is not
/// currently consumed by `tp-lib-core`.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ProjectionConfigFfi {
    pub max_search_radius_meters: f64,
    pub projection_distance_warning_threshold: f64,
    pub suppress_warnings: u8,
}

impl From<ProjectionConfigFfi> for tp_lib_core::ProjectionConfig {
    fn from(c: ProjectionConfigFfi) -> Self {
        tp_lib_core::ProjectionConfig {
            projection_distance_warning_threshold: c.projection_distance_warning_threshold,
            suppress_warnings: c.suppress_warnings != 0,
        }
    }
}

/// Flat mirror of `tp_lib_core::PathConfig` for `#[repr(C)]` transport.
///
/// `anchors` are not transmitted via this struct; they arrive on the
/// `PreparedDetections` JSON payload of `tp_net_calculate_train_path`.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct PathConfigFfi {
    pub distance_scale: f64,
    pub heading_scale: f64,
    pub cutoff_distance: f64,
    pub heading_cutoff: f64,
    pub probability_threshold: f64,
    pub resampling_distance: f64,
    pub has_resampling_distance: u8,
    pub max_candidates: u64,
    pub path_only: u8,
    pub debug_mode: u8,
    pub beta: f64,
    pub edge_zone_distance: f64,
    pub turn_scale: f64,
    pub detection_cutoff_distance: f64,
}

impl From<PathConfigFfi> for tp_lib_core::PathConfig {
    fn from(c: PathConfigFfi) -> Self {
        tp_lib_core::PathConfig {
            distance_scale: c.distance_scale,
            heading_scale: c.heading_scale,
            cutoff_distance: c.cutoff_distance,
            heading_cutoff: c.heading_cutoff,
            probability_threshold: c.probability_threshold,
            resampling_distance: if c.has_resampling_distance != 0 {
                Some(c.resampling_distance)
            } else {
                None
            },
            max_candidates: c.max_candidates as usize,
            path_only: c.path_only != 0,
            debug_mode: c.debug_mode != 0,
            beta: c.beta,
            edge_zone_distance: c.edge_zone_distance,
            turn_scale: c.turn_scale,
            detection_cutoff_distance: c.detection_cutoff_distance,
            ..Default::default()
        }
    }
}
