//! The crate-boundary error type.

use thiserror::Error;

/// A failure crossing the portable-archive boundary. Rendering carries the
/// underlying rkyv message so callers see the concrete cause without depending
/// on rkyv's own error type at this crate's edge.
#[derive(Debug, Clone, Error)]
pub enum ArchiveError {
    /// Serializing a value to its canonical rkyv bytes failed.
    #[error("portable serialization failed: {0}")]
    Serialize(String),

    /// Deserializing (with validation-on-read) a value from rkyv bytes failed.
    #[error("portable deserialization failed: {0}")]
    Deserialize(String),
}
