//! Protos: the universal structural substrate.
//!
//! Every dialect shares the context-switching parse, the delimiters,
//! the heads, and the recursive structure. Protos is only about
//! structure: anatomy, not interpretation.

use std::collections::BTreeMap;
use std::marker::PhantomData;

// ---------------------------------------------------------------------------
// Primitive type aliases
// ---------------------------------------------------------------------------

/// The textual layer's type.
pub type Text = String;

/// A signed 64-bit integer.
pub type Integer = i64;

/// A 64-bit floating-point number.
pub type Decimal = f64;

/// A truth value.
pub type Boolean = bool;

/// A qualified string used as a name.
pub type Symbol = Text;

// ---------------------------------------------------------------------------
// Situation types
// ---------------------------------------------------------------------------

/// A span in the text: start and end byte positions.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Extent(pub Integer, pub Integer);

/// A path in the protoform tree: a sequence of child indices.
pub type Path = Vec<Integer>;

/// A mapping from paths to extents.
pub type Situation = BTreeMap<Path, Extent>;

// ---------------------------------------------------------------------------
// Delimiter and separator types
// ---------------------------------------------------------------------------

/// A separator between a head and its body.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Separator {
    /// The period `.` separator.
    Period,
    /// The exclamation `!` separator.
    Exclamation,
    /// The colon `:` separator.
    Colon,
}

/// A structural enclosure: parsed recursively, children are protoforms.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Enclosure {
    /// Braces `{ }`.
    Braced,
    /// Brackets `[ ]`.
    Bracketed,
    /// Angle brackets `< >`.
    Angled,
}

/// An opaque boundary: content is read literally until the closer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Boundary {
    /// Curly quotes: no escape, every glyph inside is content.
    CurlyQuotes,
    /// Parentheses: read by balance with backslash escapes for `(`, `)`, `\`.
    Parentheses,
}

// ---------------------------------------------------------------------------
// Protoform layer types
// ---------------------------------------------------------------------------

/// The name of a headed structure.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Head {
    /// A bare symbol.
    Bare(Symbol),
    /// A symbol with constraints in angle brackets.
    Qualified(Symbol, Vec<Protoform>),
}

/// One unit of structural text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Protoform {
    /// A head, a separator, and a body.
    Headed(Head, Separator, Box<Protoform>),
    /// Content between structural delimiters.
    Enclosed(Enclosure, Vec<Protoform>),
    /// Opaque content between boundary delimiters.
    Opaque(Boundary, Text),
    /// A standalone head with no body.
    Bare(Head),
}

/// The structural survey of a text: protoforms and their situation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Delineation {
    /// The top-level protoforms.
    pub protoforms: Vec<Protoform>,
    /// Paths to extents in the source text.
    pub situation: Situation,
}

// ---------------------------------------------------------------------------
// Fault types
// ---------------------------------------------------------------------------

/// A structural problem found during delineation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Problem {
    /// An enclosure was opened but never closed.
    Unclosed(Enclosure),
    /// A boundary was opened but never terminated.
    UnclosedBoundary(Boundary),
    /// A closer was found without a matching opener.
    Unopened,
}

/// A structural fault: a problem at an extent.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Fault {
    /// Where the fault is.
    pub extent: Extent,
    /// What the fault is.
    pub problem: Problem,
}

// ---------------------------------------------------------------------------
// Universal types
// ---------------------------------------------------------------------------

/// A potential value: text that may become a value when actualized.
pub struct Potential<T, C = ()>(Text, PhantomData<fn() -> (T, C)>);

/// A situated value: a value with an optional extent from the text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Situated<F>(pub Option<Extent>, pub F);

// ---------------------------------------------------------------------------
// Kinds (traits)
// ---------------------------------------------------------------------------

/// The kind whose capability yields the glyph character.
pub trait Glyphing {
    /// The character glyph.
    fn glyph(&self) -> char;
}

/// The kind whose capability yields a delimiter pair's characters.
pub trait Delimiting {
    /// The opening character.
    fn opener(&self) -> char;
    /// The closing character.
    fn closer(&self) -> char;
}

/// The kind whose static capability identifies a separator from its character.
pub trait Identifying: Sized {
    /// Identify the variant from a character.
    fn identify(c: char) -> Option<Self>;
}

/// The kind whose static capability recognizes a delimiter from its opener or closer.
pub trait Recognizing: Sized {
    /// Identify from an opening character.
    fn from_opener(c: char) -> Option<Self>;
    /// Identify from a closing character.
    fn from_closer(c: char) -> Option<Self>;
}

/// The kind whose capability yields the canonical text of a value.
pub trait Textualizable {
    /// Yield the canonical textual form.
    fn textualize(&self) -> Text;
}

/// The kind whose capability yields a delineation.
pub trait Protosizable {
    /// The fault type.
    type Fault;
    /// Protosize into a delineation.
    fn protosize(&self) -> Result<Delineation, Self::Fault>;
}

/// The kind whose capability conceives a concept from a protoform.
pub trait Conceivable<C> {
    /// The fault type.
    type Fault;
    /// Conceive the concept.
    fn conceive(&self) -> Result<C, Self::Fault>;
}

/// The kind whose capability incorporates a corporate value from a concept.
pub trait Incorporable<T> {
    /// The fault type.
    type Fault;
    /// Incorporate the corporate value, consuming the concept.
    fn incorporate(self) -> Result<T, Self::Fault>;
}

/// The kind whose capability actualizes a potential value.
pub trait Actualizable<T: Sized> {
    /// The fault type.
    type Fault;
    /// Actualize the potential value.
    fn actualize(&self) -> Result<T, Self::Fault>;
}

/// The kind whose capability yields the path of a fault.
pub trait Pathed {
    /// The path in the structure.
    fn path(&self) -> &[Integer];
}

/// The kind whose capability looks up an extent by path.
pub trait Situating {
    /// Look up the extent at the given path.
    fn situate(&self, path: &[Integer]) -> Option<Extent>;
}

/// The kind whose capability yields the text of a potential value.
pub trait Texted {
    /// The underlying text.
    fn text(&self) -> &str;
}

// ---------------------------------------------------------------------------
// Glyph, delimiter, and classification implementations
// ---------------------------------------------------------------------------

impl Glyphing for Separator {
    fn glyph(&self) -> char {
        match self {
            Self::Period => '.',
            Self::Exclamation => '!',
            Self::Colon => ':',
        }
    }
}

impl Delimiting for Enclosure {
    fn opener(&self) -> char {
        match self {
            Self::Braced => '{',
            Self::Bracketed => '[',
            Self::Angled => '<',
        }
    }

    fn closer(&self) -> char {
        match self {
            Self::Braced => '}',
            Self::Bracketed => ']',
            Self::Angled => '>',
        }
    }
}

impl Delimiting for Boundary {
    fn opener(&self) -> char {
        match self {
            Self::CurlyQuotes => '\u{201C}',
            Self::Parentheses => '(',
        }
    }

    fn closer(&self) -> char {
        match self {
            Self::CurlyQuotes => '\u{201D}',
            Self::Parentheses => ')',
        }
    }
}

impl Identifying for Separator {
    fn identify(c: char) -> Option<Self> {
        let variants = [Self::Period, Self::Exclamation, Self::Colon];
        variants.into_iter().find(|v| v.glyph() == c)
    }
}

impl Recognizing for Enclosure {
    fn from_opener(c: char) -> Option<Self> {
        let variants = [Self::Braced, Self::Bracketed, Self::Angled];
        variants.into_iter().find(|v| v.opener() == c)
    }

    fn from_closer(c: char) -> Option<Self> {
        let variants = [Self::Braced, Self::Bracketed, Self::Angled];
        variants.into_iter().find(|v| v.closer() == c)
    }
}

impl Recognizing for Boundary {
    fn from_opener(c: char) -> Option<Self> {
        let variants = [Self::CurlyQuotes, Self::Parentheses];
        variants.into_iter().find(|v| v.opener() == c)
    }

    fn from_closer(c: char) -> Option<Self> {
        let variants = [Self::CurlyQuotes, Self::Parentheses];
        variants.into_iter().find(|v| v.closer() == c)
    }
}

// ---------------------------------------------------------------------------
// Potential and Situated implementations
// ---------------------------------------------------------------------------

impl<T, C> Texted for Potential<T, C> {
    fn text(&self) -> &str {
        &self.0
    }
}

impl<T, C> From<Text> for Potential<T, C> {
    fn from(text: Text) -> Self {
        Self(text, PhantomData)
    }
}

impl<T, C> From<&str> for Potential<T, C> {
    fn from(s: &str) -> Self {
        Self(s.to_owned(), PhantomData)
    }
}

impl<T, C> Clone for Potential<T, C> {
    fn clone(&self) -> Self {
        Self(self.0.clone(), PhantomData)
    }
}

impl<T, C> std::fmt::Debug for Potential<T, C> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Potential").field(&self.0).finish()
    }
}

impl<T, C> PartialEq for Potential<T, C> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl<T, C> Eq for Potential<T, C> {}

// ---------------------------------------------------------------------------
// Modules: implementation below, named for the pass they are
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Iterative Drop for Delineation: prevents stack overflow on deep chains
// ---------------------------------------------------------------------------

impl Drop for Delineation {
    fn drop(&mut self) {
        let mut worklist: Vec<Protoform> = std::mem::take(&mut self.protoforms);
        while let Some(pf) = worklist.pop() {
            match pf {
                Protoform::Headed(_, _, body) => worklist.push(*body),
                Protoform::Enclosed(_, children) => worklist.extend(children),
                _ => {}
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Modules: implementation below, named for the pass they are
// ---------------------------------------------------------------------------

mod actualization;
mod delineation;
mod textualization;
