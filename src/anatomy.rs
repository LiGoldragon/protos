//! The types of every layer: text, situation, delimiters, protoform, fault.

use std::{
    fmt,
    hash::{Hash, Hasher},
    marker::PhantomData,
};

/// A signed 64-bit integer.
pub type Integer = i64;

/// A finite 64-bit decimal value.
///
/// A corporate value must have canonical readable text, so IEEE infinities and
/// NaN never inhabit this type.
#[derive(Clone, Copy, Default)]
pub struct Decimal(f64);

/// Why a floating-point value cannot become a [`Decimal`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DecimalRefusal {
    /// The value is NaN or an infinity.
    Nonfinite,
}

impl TryFrom<f64> for Decimal {
    type Error = DecimalRefusal;

    fn try_from(value: f64) -> Result<Self, Self::Error> {
        value
            .is_finite()
            .then_some(Self(value))
            .ok_or(DecimalRefusal::Nonfinite)
    }
}

impl From<Decimal> for f64 {
    fn from(decimal: Decimal) -> f64 {
        decimal.0
    }
}

impl PartialEq for Decimal {
    fn eq(&self, other: &Self) -> bool {
        self.0.to_bits() == other.0.to_bits()
    }
}

impl Eq for Decimal {}

impl Hash for Decimal {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.to_bits().hash(state);
    }
}

impl fmt::Debug for Decimal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl fmt::Display for Decimal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// A truth value.
pub type Boolean = bool;

/// A head symbol: a non-empty run of plain glyphs.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Symbol(pub(crate) String);

/// A structural bare run: non-empty plain glyphs and separators that the
/// reader itself leaves bare rather than delineating as a headed chain.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Bare(pub(crate) String);

/// A datom word run: non-empty plain glyphs and separators, whose meaning is
/// supplied by the position that carries it.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Word(pub(crate) String);

/// Why text cannot become a structural bare run or symbol.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct BareRefusal {
    /// The glyph that does not belong to the requested structural form.
    pub glyph: char,
    /// Its byte offset in the refused text.
    pub offset: Integer,
}

/// Text that can be quoted: a string carrying no closing curly quote (U+201D).
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Text(pub(crate) String);

/// Content held by an opaque boundary.
///
/// The boundary that contains it determines its terminator and escaping rules,
/// so it has no delimiter-independent exclusion.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Opaque(pub(crate) String);

/// The refusal of a string as [`Text`]: the glyph that cannot be carried, at its byte offset.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Refusal {
    /// The refused glyph.
    pub glyph: char,
    /// Its byte offset in the refused string.
    pub offset: Integer,
}

/// A span of text: start and end byte offsets.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Extent(pub Integer, pub Integer);

/// A path from a root structure down: the index of each child taken.
pub type Path = Vec<Integer>;

/// Where a structure and each of its children lie in the text: a tree parallel to the structure.
pub struct Situation {
    /// The structure's own span.
    pub extent: Extent,
    /// The situations of its children, in path order.
    pub children: Vec<Situation>,
}

/// A value paired with its situation.
pub struct Situated<T>(pub Situation, pub T);

/// A separator between a head and its body.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Separator {
    /// The period `.`.
    Period,
    /// The exclamation mark `!`.
    Exclamation,
    /// The colon `:`.
    Colon,
}

/// A structural enclosure: what it holds is read as structure.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Enclosure {
    /// Braces `{ }`.
    Braced,
    /// Brackets `[ ]`.
    Bracketed,
    /// Angle brackets `< >`.
    Angled,
}

/// An opaque boundary: what it holds is content.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Boundary {
    /// Curly quotes `“ ”`: every glyph inside is content until the closing quote.
    CurlyQuotes,
    /// Parentheses `( )`: read by balance, with `\(` `\)` `\\` escapes.
    Parentheses,
}

/// The head of a structure: a symbol, possibly qualified by constraints in angle brackets.
pub enum Head {
    /// A symbol alone.
    Symbol(Symbol),
    /// A symbol immediately followed by its constraints: `Vector<Text>`.
    Qualified(Symbol, Vec<Protoform>),
}

/// One unit of structural text.
pub enum Protoform {
    /// A head, a separator and a body.
    Headed(Head, Separator, Box<Protoform>),
    /// Structures between the delimiters of an enclosure.
    Enclosed(Enclosure, Vec<Protoform>),
    /// Text between curly quotes. [`Text`] excludes the closing quote.
    Quoted(Text),
    /// Content between balanced parentheses.
    Parenthesized(Opaque),
    /// A bare run whose separators are data in its position.
    Bare(Bare),
    /// A qualified head with no body.
    Qualified(Symbol, Vec<Protoform>),
}

/// The structural survey of a text: its top-level structures, each situated.
pub struct Delineation(pub Vec<Situated<Protoform>>);

/// What can go wrong in the structure of a text.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Problem {
    /// An enclosure opened and never closed.
    Unclosed(Enclosure),
    /// A closing enclosure glyph with no open enclosure to close.
    Unopened(Enclosure),
    /// A boundary opened and never terminated.
    Unterminated(Boundary),
    /// A closing boundary glyph with no open boundary to terminate.
    Stray(Boundary),
}

/// A structural fault: a problem at an extent.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Fault {
    /// Where the problem is.
    pub extent: Extent,
    /// What the problem is.
    pub problem: Problem,
}

/// Text that may become a `T` through the concept `C`: potential until it matches its anatomy.
pub struct Potential<T, C = ()>(pub(crate) String, pub(crate) PhantomData<fn() -> (T, C)>);
