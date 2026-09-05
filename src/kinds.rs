//! The kinds: what each layer bears, named by where its capability goes.

use crate::anatomy::{Delineation, Extent, Integer, Situated, Situation};

/// The kind whose capability yields its one glyph.
pub trait Glyphing {
    /// The glyph.
    fn glyph(&self) -> char;
}

/// The kind whose capabilities yield its opening and closing glyphs.
pub trait Delimiting {
    /// The glyph that opens it.
    fn opener(&self) -> char;
    /// The glyph that closes it.
    fn closer(&self) -> char;
}

/// The kind whose variants can be walked in order: the first, and the one after each.
pub trait Serial: Sized + Copy {
    /// The first variant.
    fn first() -> Self;
    /// The variant after this one, if any.
    fn after(self) -> Option<Self>;
}

/// The kind whose capability writes its canonical text.
pub trait Textualizable {
    /// The canonical text.
    fn textualize(&self) -> String;
}

/// The kind whose capability writes its canonical text and the situation of every structure in it, in one pass.
pub trait Situating {
    /// The canonical text, situated.
    fn situate(&self) -> Situated<String>;
}

/// The kind whose capability yields the delineation: text finds it, a concept computes it.
pub trait Protosizable {
    /// What can go wrong.
    type Fault;
    /// The delineation.
    fn protosize(&self) -> Result<Delineation, Self::Fault>;
}

/// The kind whose capability conceives a situated concept `C` from situated structure.
pub trait Conceivable<C> {
    /// What can go wrong.
    type Fault;
    /// The concept, situated as the structure was.
    fn conceive(&self) -> Result<Situated<C>, Self::Fault>;
}

/// The kind whose capability incorporates a corporate `T` from a concept at its situation.
pub trait Incorporable<T> {
    /// What can go wrong.
    type Fault;
    /// The corporate value.
    fn incorporate(&self, at: &Situation) -> Result<T, Self::Fault>;
}

/// The kind whose capability actualizes a potential `T`: the whole descent.
pub trait Actualizable<T> {
    /// What can go wrong.
    type Fault;
    /// The value, if the text matches its anatomy.
    fn actualize(&self) -> Result<T, Self::Fault>;
}

/// The kind whose capability yields the path of a fault, and places it under a parent.
///
/// The path convention every dialect shares: indices from the root structure
/// down. A headed structure's children are its head at 0 and its body at 1; an
/// enclosure's children are the enclosed structures in order; a qualified head's
/// constraints are the head's children; opaque content and a bare symbol have
/// none. A concept keeps the same rule for the structures it was conceived from.
pub trait Pathed {
    /// The path from the root to the fault.
    fn path(&self) -> &[Integer];
    /// The same fault seen from the parent, under the given child index.
    fn within(self, index: Integer) -> Self;
}

/// The kind whose capabilities look a situation up by path or by child index.
pub trait Locating {
    /// The extent at the path, if the path exists.
    fn locate(&self, path: &[Integer]) -> Option<Extent>;
    /// The child situation at the index; nowhere, if the index does not exist.
    fn part(&self, index: Integer) -> &Situation;
}

/// The kind whose capability yields the text it holds.
pub trait Texted {
    /// The text.
    fn text(&self) -> &str;
}
