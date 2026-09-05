//! Protos: the universal structural substrate.
//!
//! Protos is only about structure. It owns the one character reader and the
//! one character writer every dialect shares: the delimiters, the heads, the
//! recursive structure, and the situation of every structure in its text. What
//! a structure means is said by the dialect, never by protos.
//!
//! Four layers: Text, Protoform, Concept, Corporate. Text arrives as a
//! [`Potential`] value and descends — protosize, conceive, incorporate — and
//! may fault on the way; a corporate value ascends — conceive, protosize,
//! textualize — and cannot. Extents are found on the way in and computed on
//! the way out, and they live beside the tree they describe, in a
//! [`Situation`], never inside it.

mod actualization;
mod anatomy;
mod deep;
mod delineation;
mod dropping;
mod glyph;
mod kinds;
mod opaque;
mod run;
mod situation;
mod text;
mod textualization;

pub use anatomy::{
    Bare, BareRefusal, Boolean, Boundary, Decimal, DecimalRefusal, Delineation, Enclosure, Extent,
    Fault, Head, Integer, Opaque, Path, Potential, Problem, Protoform, Refusal, Separator,
    Situated, Situation, Symbol, Text, Word,
};
pub use glyph::{Classifying, Glyph};
pub use kinds::{
    Actualizable, Conceivable, Delimiting, Glyphing, Incorporable, Locating, Pathed, Protosizable,
    Route, Serial, Situating, Texted, Textualizable,
};
