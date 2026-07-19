//! The blake3 folding primitive, one home for the length-prefix convention.
//!
//! [`IdentityHasher`] wraps a `blake3::Hasher` and owns the two folding
//! disciplines the stack already relied on: raw folding (tag bytes, little-endian
//! counts) and length-prefixed folding. The length-prefix method is
//! sema-engine's `update_bytes` (`versioning.rs:299-302`) given a single home, so
//! concatenations of variable-length fields cannot silently collide.

use crate::domain::HashDomain;
use crate::hash::ContentHash;

/// A blake3 hasher that speaks the stack's folding conventions. It is the shared
/// primitive underneath both blake3 domain disciplines: schema's derive-key form
/// and sema-engine's plain-hasher-with-magic-prefix form both build on it.
pub struct IdentityHasher {
    inner: blake3::Hasher,
}

impl IdentityHasher {
    /// A plain, un-primed hasher. sema-engine's composite digests (record-key,
    /// commit-log entry, store-schema) build on this: their domain separation is
    /// folded in as data, not through blake3's derive-key mechanism.
    pub fn unprimed() -> Self {
        Self {
            inner: blake3::Hasher::new(),
        }
    }

    /// A hasher primed with blake3's native derive-key context — schema's
    /// convention (`schema/src/identity.rs:69-73`).
    pub fn keyed(context: &str) -> Self {
        Self {
            inner: blake3::Hasher::new_derive_key(context),
        }
    }

    /// Fold bytes verbatim, with no framing. sema-engine folds discriminator tag
    /// bytes and raw little-endian counts this way.
    pub fn update_raw(&mut self, bytes: &[u8]) -> &mut Self {
        self.inner.update(bytes);
        self
    }

    /// Fold bytes length-prefixed: the `u64` little-endian length, then the bytes.
    /// This is sema-engine's `update_bytes` primitive, the single home for the
    /// convention so variable-length fields cannot collide across a concatenation.
    pub fn update_length_prefixed(&mut self, bytes: &[u8]) -> &mut Self {
        self.inner.update(&(bytes.len() as u64).to_le_bytes());
        self.inner.update(bytes);
        self
    }

    /// The raw 32-byte digest.
    pub fn finalize_bytes(self) -> [u8; 32] {
        *self.inner.finalize().as_bytes()
    }

    /// The digest as a domain-typed content hash.
    pub fn finalize<Domain: HashDomain>(self) -> ContentHash<Domain> {
        ContentHash::from_bytes(self.finalize_bytes())
    }
}
