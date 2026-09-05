//! The types of every layer: text, situation, delimiters, protoform, fault.

use std::marker::PhantomData;

/// A signed 64-bit integer.
pub type Integer = i64;

/// A 64-bit floating-point number.
pub type Decimal = f64;

/// A truth value.
pub type Boolean = bool;

/// A symbol: a non-empty run with no whitespace, delimiter or separator glyph.
pub type Symbol = String;

/// Text that can be quoted: a string carrying no closing curly quote (U+201D).
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Text(pub(crate) String);

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
#[derive(Debug, PartialEq, Eq)]
pub struct Situation {
    /// The structure's own span.
    pub extent: Extent,
    /// The situations of its children, in path order.
    pub children: Vec<Situation>,
}

/// A value paired with its situation.
#[derive(Debug, PartialEq, Eq)]
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
#[derive(Debug, PartialEq, Eq)]
pub enum Head {
    /// A symbol alone.
    Symbol(Symbol),
    /// A symbol immediately followed by its constraints: `Vector<Text>`.
    Qualified(Symbol, Vec<Protoform>),
}

/// One unit of structural text.
#[derive(Debug, PartialEq, Eq)]
pub enum Protoform {
    /// A head, a separator and a body.
    Headed(Head, Separator, Box<Protoform>),
    /// Structures between the delimiters of an enclosure.
    Enclosed(Enclosure, Vec<Protoform>),
    /// Content between the delimiters of a boundary.
    Opaque(Boundary, Text),
    /// A head alone.
    Bare(Head),
}

/// The structural survey of a text: its top-level structures, each situated.
#[derive(Debug, PartialEq, Eq)]
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
