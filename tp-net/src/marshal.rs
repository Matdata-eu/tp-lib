//! JSON (de)serialization helpers for crossing the FFI boundary.

use crate::ffi::ByteBuffer;
use serde::{Deserialize, Serialize};

/// Serialize a value to JSON and return an owned [`ByteBuffer`]. Serialization
/// failures return an error-sentinel buffer (null ptr, len == -1).
pub(crate) fn to_json_bytes<T: Serialize>(val: &T) -> ByteBuffer {
    match serde_json::to_vec(val) {
        Ok(v) => ByteBuffer::from_vec(v),
        Err(_) => ByteBuffer::null_error(),
    }
}

/// Borrow a JSON byte slice from a raw pointer and deserialize into `T`.
///
/// # Safety
/// `ptr` must be non-null and reference at least `len` valid bytes.
pub(crate) unsafe fn from_json_bytes<'a, T: Deserialize<'a>>(
    ptr: *const u8,
    len: i32,
) -> Result<T, serde_json::Error> {
    if ptr.is_null() || len < 0 {
        return Err(<serde_json::Error as serde::de::Error>::custom(
            "null pointer or negative length",
        ));
    }
    let slice = std::slice::from_raw_parts(ptr, len as usize);
    serde_json::from_slice(slice)
}
