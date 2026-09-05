//! Textualization: Protoform to Text (the printing pass).
//!
//! Textualize yields the canonical text of a value and cannot fault.
//! Parentheses escape `(`, `)`, and `\` unconditionally so that any
//! content round-trips. Curly quotes have no escape: the closing
//! curly quote U+201D is unrepresentable in curly-quoted content.

use crate::{
    Boundary, Delimiting, Delineation, Enclosure, Glyphing, Head, Protoform, Text, Textualizable,
};

// ---------------------------------------------------------------------------
// Escaping: the kind whose capability escapes content for parentheses
// ---------------------------------------------------------------------------

/// The kind whose capability escapes content for parenthesized printing.
trait Escaping {
    fn escape_for_parentheses(&self) -> String;
}

impl Escaping for str {
    fn escape_for_parentheses(&self) -> String {
        let opener = Boundary::Parentheses.opener();
        let closer = Boundary::Parentheses.closer();
        let mut result = String::with_capacity(self.len());
        for c in self.chars() {
            if c == opener || c == closer || c == '\\' {
                result.push('\\');
            }
            result.push(c);
        }
        result
    }
}

// ---------------------------------------------------------------------------
// Textualizable implementations
// ---------------------------------------------------------------------------

/// The kind whose capability yields the spacing rule for an enclosure.
trait Spacing {
    fn is_spaced(&self) -> bool;
}

impl Spacing for Enclosure {
    fn is_spaced(&self) -> bool {
        match self {
            Self::Braced | Self::Bracketed => true,
            Self::Angled => false,
        }
    }
}

impl Textualizable for Head {
    fn textualize(&self) -> Text {
        match self {
            Head::Bare(symbol) => symbol.clone(),
            Head::Qualified(symbol, constraints) => {
                let opener = Enclosure::Angled.opener();
                let closer = Enclosure::Angled.closer();
                let mut parts = Vec::with_capacity(constraints.len());
                for c in constraints {
                    parts.push(c.textualize());
                }
                let joined = parts.join(" ");
                format!("{symbol}{opener}{joined}{closer}")
            }
        }
    }
}

/// Textualize a non-Headed protoform leaf.
trait LeafTextualizing {
    fn textualize_leaf(&self) -> Text;
}

impl LeafTextualizing for Protoform {
    fn textualize_leaf(&self) -> Text {
        match self {
            Protoform::Enclosed(enclosure, children) => {
                let open = enclosure.opener();
                let close = enclosure.closer();
                if children.is_empty() {
                    return format!("{open}{close}");
                }
                let mut parts = Vec::with_capacity(children.len());
                for child in children {
                    parts.push(child.textualize());
                }
                let joined = parts.join(" ");
                if enclosure.is_spaced() {
                    format!("{open} {joined} {close}")
                } else {
                    format!("{open}{joined}{close}")
                }
            }
            Protoform::Opaque(boundary, content) => {
                let open = boundary.opener();
                let close = boundary.closer();
                match boundary {
                    Boundary::CurlyQuotes => format!("{open}{content}{close}"),
                    Boundary::Parentheses => {
                        let escaped = content.escape_for_parentheses();
                        format!("{open}{escaped}{close}")
                    }
                }
            }
            Protoform::Bare(head) => head.textualize(),
            Protoform::Headed(_, _, _) => unreachable!(),
        }
    }
}

impl Textualizable for Protoform {
    fn textualize(&self) -> Text {
        // Iterative for Headed chains to avoid stack overflow on deep chains
        let mut result = String::new();
        let mut current = self;
        loop {
            match current {
                Protoform::Headed(head, sep, body) => {
                    result.push_str(&head.textualize());
                    result.push(sep.glyph());
                    current = body.as_ref();
                }
                leaf => {
                    result.push_str(&leaf.textualize_leaf());
                    break;
                }
            }
        }
        result
    }
}

impl Textualizable for Delineation {
    fn textualize(&self) -> Text {
        let mut parts = Vec::with_capacity(self.protoforms.len());
        for pf in &self.protoforms {
            parts.push(pf.textualize());
        }
        parts.join(" ")
    }
}
