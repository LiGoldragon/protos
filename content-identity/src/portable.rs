//! The one portable-archive discipline.
//!
//! [`PortableArchive`] is the canonical bound lifted verbatim from sema-engine's
//! `EngineStoredValue`/`EngineStoredRecord` (`sema-engine/src/record.rs:153-198`).
//! A type that is `PortableArchive` round-trips through rkyv with
//! validation-on-read, in the fixed little-endian / 32-bit-pointer / unaligned
//! layout, with no engine-specific type in the bound. It is blanket-implemented,
//! so every consumer names the discipline once here instead of restating the
//! 34-plus per-crate copies the shared-codec survey found.

use rkyv::api::high::HighDeserializer;
use rkyv::bytecheck::CheckBytes;
use rkyv::rancor::{self, Strategy};
use rkyv::ser::Serializer;
use rkyv::ser::allocator::ArenaHandle;
use rkyv::ser::sharing::Share;
use rkyv::util::AlignedVec;
use rkyv::validation::Validator;
use rkyv::validation::archive::ArchiveValidator;
use rkyv::validation::shared::SharedValidator;
use rkyv::{Archive, Deserialize as RkyvDeserialize, Serialize as RkyvSerialize};

use crate::error::ArchiveError;

/// The portable-archive round-trip discipline. The bound is lifted verbatim from
/// sema-engine's `EngineStoredValue`; the two serialization methods make the
/// canonical bytes reachable so content identity derives from one place.
pub trait PortableArchive:
    Archive
    + Clone
    + for<'serialize> RkyvSerialize<
        Strategy<Serializer<AlignedVec, ArenaHandle<'serialize>, Share>, rancor::Error>,
    >
where
    Self::Archived: RkyvDeserialize<Self, HighDeserializer<rancor::Error>>
        + for<'validation> CheckBytes<
            Strategy<Validator<ArchiveValidator<'validation>, SharedValidator>, rancor::Error>,
        >,
{
    /// The value's canonical rkyv bytes, in the fixed portable layout.
    fn to_archive_bytes(&self) -> Result<AlignedVec, ArchiveError> {
        rkyv::to_bytes::<rancor::Error>(self)
            .map_err(|error| ArchiveError::Serialize(error.to_string()))
    }

    /// Reconstruct a value from portable rkyv bytes, validating the archive on
    /// read before any access.
    fn from_archive_bytes(bytes: &[u8]) -> Result<Self, ArchiveError> {
        rkyv::from_bytes::<Self, rancor::Error>(bytes)
            .map_err(|error| ArchiveError::Deserialize(error.to_string()))
    }
}

impl<Value> PortableArchive for Value
where
    Value: Archive
        + Clone
        + for<'serialize> RkyvSerialize<
            Strategy<Serializer<AlignedVec, ArenaHandle<'serialize>, Share>, rancor::Error>,
        >,
    Value::Archived: RkyvDeserialize<Value, HighDeserializer<rancor::Error>>
        + for<'validation> CheckBytes<
            Strategy<Validator<ArchiveValidator<'validation>, SharedValidator>, rancor::Error>,
        >,
{
}
