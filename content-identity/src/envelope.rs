//! The content-addressed envelope.
//!
//! An [`Envelope<Domain>`] wraps stored canonical bytes with the layout version
//! and the content hash of the payload. It is the shape sema-engine's stored
//! records already have (payload plus its identity) given one typed home; a
//! consumer seals bytes once and later verifies that the stored identity still
//! matches the payload.

use rkyv::Deserialize as RkyvDeserialize;
use rkyv::api::high::HighDeserializer;
use rkyv::bytecheck::CheckBytes;
use rkyv::rancor::{self, Strategy};
use rkyv::util::AlignedVec;
use rkyv::validation::Validator;
use rkyv::validation::archive::ArchiveValidator;
use rkyv::validation::shared::SharedValidator;

use crate::domain::{HashDomain, LayoutVersion};
use crate::error::ArchiveError;
use crate::hash::ContentHash;
use crate::portable::PortableArchive;

/// A payload sealed under a domain: its canonical bytes, the domain's layout
/// version, and the content hash that addresses it.
pub struct Envelope<Domain: HashDomain> {
    layout: LayoutVersion,
    identity: ContentHash<Domain>,
    payload: AlignedVec,
}

impl<Domain: HashDomain> Envelope<Domain> {
    /// Seal already-canonical payload bytes: derive the identity under `Domain`
    /// and stamp the domain's layout version.
    pub fn seal(payload: AlignedVec) -> Self {
        let identity = ContentHash::derive(payload.as_ref());
        Self {
            layout: Domain::layout_version(),
            identity,
            payload,
        }
    }

    /// Seal a stringless Core value: serialize to its canonical rkyv bytes, then
    /// seal them.
    pub fn of_core<Value>(value: &Value) -> Result<Self, ArchiveError>
    where
        Value: PortableArchive,
        Value::Archived: RkyvDeserialize<Value, HighDeserializer<rancor::Error>>
            + for<'validation> CheckBytes<
                Strategy<Validator<ArchiveValidator<'validation>, SharedValidator>, rancor::Error>,
            >,
    {
        Ok(Self::seal(value.to_archive_bytes()?))
    }

    /// The layout version stamped when the payload was sealed.
    pub const fn layout(&self) -> LayoutVersion {
        self.layout
    }

    /// The content address of the payload.
    pub const fn identity(&self) -> &ContentHash<Domain> {
        &self.identity
    }

    /// The sealed canonical bytes.
    pub fn payload(&self) -> &[u8] {
        self.payload.as_ref()
    }

    /// Whether the stored identity still matches a fresh derivation over the
    /// payload — the envelope's self-consistency check.
    pub fn verify(&self) -> bool {
        self.identity == ContentHash::derive(self.payload.as_ref())
    }
}
