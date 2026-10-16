//! The vertical layout's facts: where the current line stands, how an
//! enclosure's children group into elements, and which elements have a next
//! layer.

use crate::{Enclosure, Protos};

/// Where the current line stands: the glyphs written since its start.
pub(crate) struct Column {
    pub(crate) glyphs: usize,
}
pub(crate) trait Advancing {
    fn pass(&mut self, text: &str);
    fn pass_glyph(&mut self, glyph: char);
}
impl Advancing for Column {
    fn pass(&mut self, text: &str) {
        match text.rfind('\n') {
            Some(at) => self.glyphs = text[at + 1..].chars().count(),
            None => self.glyphs += text.chars().count(),
        }
    }
    fn pass_glyph(&mut self, glyph: char) {
        if glyph == '\n' {
            self.glyphs = 0;
        } else {
            self.glyphs += 1;
        }
    }
}

/// An element of an enclosure: a node and the angled enclosures that follow
/// it tight.
pub(crate) trait Grouping {
    fn elements(&self) -> Vec<Vec<&Protos>>;
}
impl Grouping for [Protos] {
    fn elements(&self) -> Vec<Vec<&Protos>> {
        let mut elements: Vec<Vec<&Protos>> = Vec::new();
        for child in self {
            let angled = matches!(
                child,
                Protos::Enclosed {
                    enclosure: Enclosure::Angled,
                    ..
                }
            );
            match elements.last_mut() {
                Some(element) if angled && element.holds_tight() => element.push(child),
                _ => elements.push(vec![child]),
            }
        }
        elements
    }
}

/// What an element is, for the layout.
pub(crate) trait Layered {
    /// The element has a next layer: its node is headed or a non-empty brace
    /// or bracket enclosure.
    fn layered(&self) -> bool;
    /// An angled enclosure written tight after the element reads back as its
    /// sibling: the element's last leaf does not end on a separator, which
    /// would make the angles the constraints of a head.
    fn holds_tight(&self) -> bool;
}
impl Layered for [&Protos] {
    fn layered(&self) -> bool {
        match self.first() {
            Some(Protos::Headed { .. }) => true,
            Some(Protos::Enclosed {
                enclosure: Enclosure::Braced | Enclosure::Bracketed | Enclosure::Vectored,
                children,
                ..
            }) => !children.is_empty(),
            _ => false,
        }
    }
    fn holds_tight(&self) -> bool {
        let mut last = self.last().copied();
        while let Some(Protos::Headed { body, .. }) = last {
            last = Some(body);
        }
        match last {
            Some(Protos::Bare { text, .. }) => !text.ends_with(['.', '!', ':']),
            Some(Protos::Enclosed { .. }) | Some(Protos::Opaque { .. }) => true,
            Some(Protos::Headed { .. }) | None => false,
        }
    }
}
