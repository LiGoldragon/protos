//! The structural layer shared by every Protos dialect.
mod core;
mod layout;
mod rendering;
mod traversing;
pub use core::{
    Boundary, BoundedProtosizable, BuildsReaderBudget, Canonicalizable, Compactable, Enclosure,
    Error, Extent, LexicalCursor, LexicallyReadable, LexicallyTraversable, Problem, Protos,
    Protosizable, ReaderBudget, ReaderBudgetConstructionError, ReaderBudgetConstructionReason,
    ReaderBudgetField, ReadsReaderBudget, Separator, Spendable, Symbol, Textualizable,
};
