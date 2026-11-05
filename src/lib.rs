//! The structural layer shared by every Protos dialect.
mod core;
mod layout;
mod rendering;
mod traversing;
pub use core::{
    Boundary, BoundedProtosizable, BudgetAccess, Canonicalizable, Compactable, Enclosure, Error,
    Extent, ExtentAccess, OutOfRange, Position, PositionAccess, Problem, Protos, Protosizable,
    ReaderBudget, ReversedExtent, Separator, Spendable, Symbol, Textualizable,
};
