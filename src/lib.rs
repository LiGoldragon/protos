//! The structural layer shared by every Protos dialect.
mod core;
pub use core::{
    Boundary, BoundedProtosizable, Enclosure, Error, Extent, Problem, Protos, Protosizable,
    ReaderBudget, Separator, Symbol, Textualizable,
};
