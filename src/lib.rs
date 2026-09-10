//! The structural layer shared by every Protos dialect.
mod core;
mod dropping;
pub use core::{
    Boundary, BoundedProtosizable, Canonicalizing, Enclosure, Error, Extent, Problem, Protos,
    Protosizable, ReaderBudget, Separator, Symbol, Textualizable,
};
