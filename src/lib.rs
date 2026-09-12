//! The structural layer shared by every Protos dialect.
mod core;
mod rendering;
mod traversing;
pub use core::{
    Boundary, BoundedProtosizable, Canonicalizable, Enclosure, Error, Extent, Problem, Protos,
    Protosizable, ReaderBudget, Separator, Symbol, Textualizable,
};
