//! The glyphs: each delimiter and separator names its own, and a character is
//! classified by walking the variants and asking.

use crate::anatomy::{Boundary, Enclosure, Separator};
use crate::kinds::{Delimiting, Glyphing, Serial};

/// The two marks that are neither delimiter nor separator: the comment opener and the escape.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Mark {
    /// `;` opens a comment to the end of the line.
    Comment,
    /// `\` escapes the next glyph inside parentheses.
    Escape,
}

impl Glyphing for Mark {
    fn glyph(&self) -> char {
        match self {
            Self::Comment => ';',
            Self::Escape => '\\',
        }
    }
}

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

impl Serial for Separator {
    fn first() -> Self {
        Self::Period
    }

    fn after(self) -> Option<Self> {
        match self {
            Self::Period => Some(Self::Exclamation),
            Self::Exclamation => Some(Self::Colon),
            Self::Colon => None,
        }
    }
}

impl Serial for Enclosure {
    fn first() -> Self {
        Self::Braced
    }

    fn after(self) -> Option<Self> {
        match self {
            Self::Braced => Some(Self::Bracketed),
            Self::Bracketed => Some(Self::Angled),
            Self::Angled => None,
        }
    }
}

impl Serial for Boundary {
    fn first() -> Self {
        Self::CurlyQuotes
    }

    fn after(self) -> Option<Self> {
        match self {
            Self::CurlyQuotes => Some(Self::Parentheses),
            Self::Parentheses => None,
        }
    }
}

/// The kind whose static capabilities find the variant that a glyph opens or closes, by walking the variants.
pub(crate) trait Recognizing: Serial + Delimiting {
    /// The variant the glyph opens.
    fn opening(glyph: char) -> Option<Self> {
        let mut variant = Self::first();
        loop {
            if variant.opener() == glyph {
                return Some(variant);
            }
            variant = variant.after()?;
        }
    }

    /// The variant the glyph closes.
    fn closing(glyph: char) -> Option<Self> {
        let mut variant = Self::first();
        loop {
            if variant.closer() == glyph {
                return Some(variant);
            }
            variant = variant.after()?;
        }
    }
}

impl<D: Serial + Delimiting> Recognizing for D {}

/// The kind whose static capability finds the variant a glyph is, by walking the variants.
pub(crate) trait Identifying: Serial + Glyphing {
    /// The variant whose glyph this is.
    fn identify(glyph: char) -> Option<Self> {
        let mut variant = Self::first();
        loop {
            if variant.glyph() == glyph {
                return Some(variant);
            }
            variant = variant.after()?;
        }
    }
}

impl<G: Serial + Glyphing> Identifying for G {}

/// What a character is to protos.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Glyph {
    /// Whitespace between structures.
    Space,
    /// The comment opener.
    Comment,
    /// The opener of an enclosure.
    Open(Enclosure),
    /// The closer of an enclosure.
    Close(Enclosure),
    /// The opener of a boundary.
    Bound(Boundary),
    /// The closer of a boundary.
    Unbound(Boundary),
    /// A separator.
    Separate(Separator),
    /// Any other character: part of a bare run.
    Plain,
}

/// The kind whose capability classifies a character: what it is to protos.
pub trait Classifying {
    /// What the character is.
    fn classify(self) -> Glyph;
}

impl Classifying for char {
    fn classify(self) -> Glyph {
        if self.is_whitespace() {
            Glyph::Space
        } else if self == Mark::Comment.glyph() {
            Glyph::Comment
        } else if let Some(enclosure) = Enclosure::opening(self) {
            Glyph::Open(enclosure)
        } else if let Some(enclosure) = Enclosure::closing(self) {
            Glyph::Close(enclosure)
        } else if let Some(boundary) = Boundary::opening(self) {
            Glyph::Bound(boundary)
        } else if let Some(boundary) = Boundary::closing(self) {
            Glyph::Unbound(boundary)
        } else if let Some(separator) = Separator::identify(self) {
            Glyph::Separate(separator)
        } else {
            Glyph::Plain
        }
    }
}
