//! Portable content identity for stringless Core values.
//!
//! This is the dependency-graph leaf of the shared-codec language family: it
//! depends only on `rkyv` and `blake3`, holds no strings in any Core-facing
//! surface, and depends on no other stack crate. It carries the machinery every
//! layer above reinvented independently:
//!
//! - [`PortableArchive`] — the one rkyv portable-archive discipline, lifted
//!   verbatim from sema-engine's `EngineStoredValue` bound so the round-trip
//!   contract lives in one place.
//! - [`ContentHash`] — one generic 32-byte digest newtype, parameterized by a
//!   typed [`HashDomain`], replacing the five duplicate digest newtypes.
//! - [`HashDomain`] / [`DomainSeparation`] / [`LayoutVersion`] — typed,
//!   layout-versioned hash domains that reconcile the stack's two blake3
//!   conventions storage-safely: existing sema-engine domain strings become
//!   `FrozenMagic` variants (byte-stable on-disk digests), new Core derivations
//!   use layout-tagged `Contextual` domains.
//! - [`IdentityHasher`] — the shared blake3 folding primitive, one home for the
//!   length-prefix convention.
//! - [`Envelope`] — a content-addressed wrapper of stored bytes.
//!
//! The identity ruling this crate embodies: content identity is blake3 over
//! stringless Core rkyv bytes, NameTable excluded, domain-separated and
//! layout-version-tagged, so a rename is hash-stable by construction.

mod domain;
mod envelope;
mod error;
mod hash;
mod hasher;
mod portable;

pub use domain::{DomainSeparation, HashDomain, LayoutVersion};
pub use envelope::Envelope;
pub use error::ArchiveError;
pub use hash::ContentHash;
pub use hasher::IdentityHasher;
pub use portable::PortableArchive;
