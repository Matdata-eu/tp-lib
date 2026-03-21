//! Origin classification for train path segments

use serde::{Deserialize, Serialize};

/// Indicates whether a path segment was selected by the algorithm or manually added by a user.
///
/// When deserialising older path files that were produced before this field existed,
/// the `#[default]` attribute ensures backward-compatibility: missing `origin` values
/// are treated as [`PathOrigin::Algorithm`].
///
/// # Serialisation
///
/// The enum is serialised in lowercase: `"algorithm"` or `"manual"`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "lowercase")]
pub enum PathOrigin {
    /// Segment was selected by the path calculation algorithm.
    /// This is the default, ensuring backward compatibility with older path files.
    #[default]
    Algorithm,

    /// Segment was manually added by a user in the webapp review interface.
    Manual,
}
