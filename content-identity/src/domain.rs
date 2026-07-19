//! Typed, layout-versioned hash domains.
//!
//! A [`HashDomain`] is a marker type that names one closed hashing context. It
//! reports a [`DomainSeparation`] — the reconciliation of the stack's two blake3
//! conventions behind one type — and a [`LayoutVersion`]. "Which layout" lives in
//! the type, never in a hand-remembered string suffix.

use crate::hasher::IdentityHasher;

/// The layout revision of the bytes a domain hashes over. A structured field of
/// the domain, not a manual string suffix — so a layout change is a typed,
/// reviewable version bump.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct LayoutVersion(u16);

impl LayoutVersion {
    pub const fn new(value: u16) -> Self {
        Self(value)
    }

    pub const fn value(self) -> u16 {
        self.0
    }

    /// The little-endian bytes folded as the layout preamble of a contextual
    /// derivation.
    pub const fn little_endian(self) -> [u8; 2] {
        self.0.to_le_bytes()
    }
}

/// How a hash domain primes its blake3 pre-image. Two disciplines, one type — the
/// storage-safe reconciliation of the stack's two blake3 conventions.
///
/// `Contextual` is the going-forward discipline: blake3's derive-key context plus
/// an explicit, structured layout-version preamble. `FrozenMagic` is the
/// storage-frozen discipline: a plain hasher whose first fold is a length-prefixed
/// magic string that already encodes its own version in the string, reproducing
/// sema-engine's exact on-disk domain strings so stored digests never move.
pub enum DomainSeparation {
    /// Derive-key context plus a structured layout-version preamble.
    Contextual {
        context: &'static str,
        layout: LayoutVersion,
    },
    /// A length-prefixed magic string on a plain hasher; the layout is carried in
    /// the string and reported here for inspection, never double-folded.
    FrozenMagic {
        magic: &'static [u8],
        layout: LayoutVersion,
    },
}

impl DomainSeparation {
    /// The layout revision this separation reports.
    pub const fn layout_version(&self) -> LayoutVersion {
        match self {
            Self::Contextual { layout, .. } | Self::FrozenMagic { layout, .. } => *layout,
        }
    }

    /// A hasher primed with this domain's separation, ready for the pre-image
    /// bytes. For `Contextual`, the layout preamble is already folded; for
    /// `FrozenMagic`, the magic string is already folded length-prefixed.
    pub fn begin(&self) -> IdentityHasher {
        match self {
            Self::Contextual { context, layout } => {
                let mut hasher = IdentityHasher::keyed(context);
                hasher.update_length_prefixed(&layout.little_endian());
                hasher
            }
            Self::FrozenMagic { magic, .. } => {
                let mut hasher = IdentityHasher::unprimed();
                hasher.update_length_prefixed(magic);
                hasher
            }
        }
    }
}

/// A typed, closed, layout-versioned hash domain. A trait, not one enum, so each
/// crate owns its own closed domain set while sharing the primitive: sema-engine
/// keeps its exact existing domain strings as `FrozenMagic` variants (byte-stable
/// on-disk digests), and each Core crate defines fresh `Contextual` domains
/// without content-identity knowing about them.
pub trait HashDomain {
    /// This domain's separation discipline.
    fn separation() -> DomainSeparation;

    /// This domain's layout revision, read from its separation.
    fn layout_version() -> LayoutVersion {
        Self::separation().layout_version()
    }
}
