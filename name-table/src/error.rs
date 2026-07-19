//! The crate-boundary error type.

use thiserror::Error;

use crate::identifier::{Identifier, IdentifierNamespace};
use crate::name::Name;

/// A failure crossing the name-table boundary.
#[derive(Debug, Clone, Error)]
pub enum NameTableError {
    /// A resolve was asked for an identifier the table never interned.
    #[error("no name interned for {0}")]
    UnknownIdentifier(Identifier),

    /// A composed table has no slice for the requested identifier namespace.
    #[error("the composed NameTable does not borrow {0:?}")]
    UnknownNamespace(IdentifierNamespace),

    /// Composition attempted to add a namespace already represented in the
    /// component's single composed NameTable.
    #[error("the composed NameTable already represents {0:?}")]
    DuplicateNamespace(IdentifierNamespace),

    /// An alias must be added by the namespace that owns its target identifier.
    #[error("cannot add an alias to borrowed identifier {0}")]
    BorrowedNamespace(Identifier),

    /// An alias spelling already resolves to a different identifier.
    #[error("name {name:?} already resolves to {existing}")]
    NameAlreadyAssigned { name: Name, existing: Identifier },

    /// Serializing the table's canonical name bytes failed.
    #[error("name-table serialization failed: {0}")]
    Serialize(String),

    /// Deserializing (with validation-on-read) the table's name bytes failed.
    #[error("name-table deserialization failed: {0}")]
    Deserialize(String),
}
