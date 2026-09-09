//! The structural layer shared by every Protos dialect.
mod core;
pub use core::{
    Boundary, Enclosure, Error, Extent, Protos, Protosizable, Separator, Symbol, Textualizable,
};
