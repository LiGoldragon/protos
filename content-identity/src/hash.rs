//! The one generic content hash.
//!
//! [`ContentHash<Domain>`] is a single 32-byte digest newtype parameterized by a
//! typed [`HashDomain`]. It collapses sema-engine's five identical `[u8; 32]`
//! digest newtypes and schema's `ContentHash` into one type; the domain carries
//! the layout-version tag, so "which layout" lives in the type rather than a
//! hand-remembered `&'static [u8]` suffix.
//!
//! rkyv storage support is deliberately elided here (the authoritative design
//! marks it "rkyv derive elided"): the leaf crate ships the identity machinery,
//! and a consumer that stores `ContentHash` in an archived record adds the rkyv
//! surface when it migrates in a later release-train slice.

use std::cmp::Ordering;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;

use rkyv::Deserialize as RkyvDeserialize;
use rkyv::api::high::HighDeserializer;
use rkyv::bytecheck::CheckBytes;
use rkyv::rancor::{self, Strategy};
use rkyv::validation::Validator;
use rkyv::validation::archive::ArchiveValidator;
use rkyv::validation::shared::SharedValidator;

use crate::domain::HashDomain;
use crate::error::ArchiveError;
use crate::portable::PortableArchive;

/// A domain-separated, layout-versioned blake3 content address over canonical
/// bytes. The domain is a compile-time marker (zero bytes at runtime), so two
/// digests derived under different domains cannot be compared or confused.
pub struct ContentHash<Domain: HashDomain> {
    bytes: [u8; 32],
    domain: PhantomData<Domain>,
}

impl<Domain: HashDomain> ContentHash<Domain> {
    /// Wrap raw digest bytes already known to belong to this domain.
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self {
            bytes,
            domain: PhantomData,
        }
    }

    /// The raw 32-byte digest.
    pub const fn bytes(&self) -> &[u8; 32] {
        &self.bytes
    }

    /// blake3 over the given canonical bytes under this domain's separation: the
    /// domain primes and preambles the pre-image, then the bytes are folded
    /// length-prefixed so a caller cannot forge a collision by re-slicing.
    pub fn derive(bytes: &[u8]) -> Self {
        let mut hasher = Domain::separation().begin();
        hasher.update_length_prefixed(bytes);
        hasher.finalize()
    }

    /// Content identity of a stringless Core value over its canonical rkyv bytes.
    ///
    /// The NameTable is not in the pre-image (it is not in the Core value), so a
    /// rename is hash-stable by construction: the identity depends only on the
    /// structural, name-free bytes.
    pub fn of_core<Value>(value: &Value) -> Result<Self, ArchiveError>
    where
        Value: PortableArchive,
        Value::Archived: RkyvDeserialize<Value, HighDeserializer<rancor::Error>>
            + for<'validation> CheckBytes<
                Strategy<Validator<ArchiveValidator<'validation>, SharedValidator>, rancor::Error>,
            >,
    {
        Ok(Self::derive(value.to_archive_bytes()?.as_ref()))
    }

    /// The digest as a lowercase hexadecimal string.
    pub fn to_hexadecimal(&self) -> String {
        self.bytes
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect()
    }
}

// Value-trait implementations written by hand so the phantom `Domain` marker does
// not leak a spurious `Domain: Clone`/`Domain: Eq`/… bound onto every hash.

impl<Domain: HashDomain> Clone for ContentHash<Domain> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<Domain: HashDomain> Copy for ContentHash<Domain> {}

impl<Domain: HashDomain> PartialEq for ContentHash<Domain> {
    fn eq(&self, other: &Self) -> bool {
        self.bytes == other.bytes
    }
}

impl<Domain: HashDomain> Eq for ContentHash<Domain> {}

impl<Domain: HashDomain> PartialOrd for ContentHash<Domain> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<Domain: HashDomain> Ord for ContentHash<Domain> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.bytes.cmp(&other.bytes)
    }
}

impl<Domain: HashDomain> Hash for ContentHash<Domain> {
    fn hash<State: Hasher>(&self, state: &mut State) {
        self.bytes.hash(state);
    }
}

impl<Domain: HashDomain> fmt::Display for ContentHash<Domain> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.to_hexadecimal())
    }
}

impl<Domain: HashDomain> fmt::Debug for ContentHash<Domain> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "ContentHash({})", self.to_hexadecimal())
    }
}
