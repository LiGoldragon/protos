//! Typed errors at the crate boundary (thiserror; no anyhow). Each operation owns a
//! focused error enum: disjointness validation, decoding, encoding, and table
//! identity.

use content_identity::ArchiveError;
use name_table::NameTableError;

use crate::ids::ScopedEncodedTypeId;

/// A structural table failed conservative disjointness validation: two accepted
/// decode forms could not be PROVEN structurally distinct, so one might silently
/// shadow the other. Conservative-safe: unprovable disjointness is an error.
#[derive(Debug, Clone, thiserror::Error)]
pub enum DisjointnessError {
    #[error(
        "core type {core_type:?}: decode forms {first} and {second} are not provably disjoint ({reason})"
    )]
    NotProvablyDisjoint {
        core_type: ScopedEncodedTypeId,
        first: usize,
        second: usize,
        reason: &'static str,
    },
}

/// Decoding a raw block under an expected type failed. A failed decode leaves the
/// NameTable byte-for-byte unchanged (interning atomicity, law 3).
#[derive(Debug, Clone, thiserror::Error)]
pub enum DecodeError {
    #[error("no structural entry for expected type {0:?}")]
    UnknownType(ScopedEncodedTypeId),
    #[error("expected {expected} block, found {found}")]
    BlockKindMismatch {
        expected: &'static str,
        found: &'static str,
    },
    #[error("atom case did not match the expected form")]
    CaseMismatch,
    #[error("literal atom did not match the expected interned keyword")]
    LiteralMismatch,
    #[error("delimited sequence held {found} objects, outside the form's bounds")]
    SequenceCardinality { found: u64 },
    #[error("could not flatten the block to a scalar leaf")]
    LeafNotFlattenable,
    #[error("scalar leaf failed to parse: {0}")]
    ScalarParse(String),
    #[error("transparent delegation cycle through type {0:?}")]
    DelegationCycle(ScopedEncodedTypeId),
    #[error("product form arity {form} did not match the {blocks} sibling blocks")]
    ProductArity { form: usize, blocks: usize },
    #[error("no accepted decode form matched under expected type {core_type:?}")]
    NoAlternative { core_type: ScopedEncodedTypeId },
    #[error(transparent)]
    Names(#[from] NameTableError),
}

/// Encoding a structural value to a raw block failed.
#[derive(Debug, Clone, thiserror::Error)]
pub enum EncodeError {
    #[error("no structural entry for expected type {0:?}")]
    UnknownType(ScopedEncodedTypeId),
    #[error("value chose constructor {chosen}, but the entry has {available} constructors")]
    ConstructorOutOfRange { chosen: u32, available: usize },
    #[error("value shape did not fit the canonical encode form: {0}")]
    ShapeMismatch(&'static str),
    #[error(transparent)]
    Names(#[from] NameTableError),
}

/// Computing a table's content identity failed.
#[derive(Debug, Clone, thiserror::Error)]
pub enum TableError {
    #[error(transparent)]
    Archive(#[from] ArchiveError),
}

/// A [`TextualForm`](crate::TextualForm) value did not carry the single text chunk the
/// provided un-view path requires. The trivial single-document case is one chunk; the
/// multi-chunk (filename→text) index is a deferred packaging future, so an un-view of a
/// zero- or many-chunk view is a loud, typed error rather than a silent pick.
#[derive(Debug, Clone, thiserror::Error)]
#[error("the textual form carried {count} chunks; un-view requires exactly one")]
pub struct SingleChunkRequired {
    /// How many chunks the view actually carried.
    pub count: usize,
}

/// A [`TextualForm`](crate::TextualForm) did not contain exactly one chunk under a
/// manifest-selected name. Multi-file textual forms are indexed values, so a caller
/// must never silently choose between duplicate files or invent a missing one.
#[derive(Debug, Clone, thiserror::Error)]
#[error("the textual form carried {count} chunks named {name:?}; exactly one is required")]
pub struct NamedChunkRequired {
    /// The manifest-selected chunk name.
    pub name: String,
    /// How many chunks carried that name.
    pub count: usize,
}
