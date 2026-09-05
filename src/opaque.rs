//! The opaque regions: curly quotes read to the closing quote, parentheses read by balance.

use crate::anatomy::{Boundary, Fault, Problem};
use crate::delineation::{Faulting, Peeking, Reader};
use crate::glyph::Mark;
use crate::kinds::{Delimiting, Glyphing};

/// The kind whose capabilities read an opaque region from the offset to its terminator.
pub(crate) trait Bounding {
    /// The content between curly quotes.
    fn read_quoted(&mut self) -> Result<String, Fault>;
    /// The content between parentheses, unescaped.
    fn read_parenthesized(&mut self) -> Result<String, Fault>;
}

impl Bounding for Reader<'_> {
    fn read_quoted(&mut self) -> Result<String, Fault> {
        let opened = self.offset;
        let closer = Boundary::CurlyQuotes.closer();
        let mut here = opened + Boundary::CurlyQuotes.opener().len_utf8();
        let content_start = here;
        while let Some(glyph) = self.glyph_at(here) {
            if glyph == closer {
                self.offset = here + closer.len_utf8();
                return Ok(self.text[content_start..here].to_owned());
            }
            here += glyph.len_utf8();
        }
        Err(self.fault(
            opened,
            self.text.len(),
            Problem::Unterminated(Boundary::CurlyQuotes),
        ))
    }

    fn read_parenthesized(&mut self) -> Result<String, Fault> {
        let opened = self.offset;
        let opener = Boundary::Parentheses.opener();
        let closer = Boundary::Parentheses.closer();
        let escape = Mark::Escape.glyph();
        let mut here = opened + opener.len_utf8();
        let mut depth = 0usize;
        let mut content = String::new();
        while let Some(glyph) = self.glyph_at(here) {
            here += glyph.len_utf8();
            if glyph == escape {
                match self.glyph_at(here) {
                    Some(next) if next == opener || next == closer || next == escape => {
                        content.push(next);
                        here += next.len_utf8();
                    }
                    Some(_) => content.push(glyph),
                    None => break,
                }
            } else if glyph == opener {
                depth += 1;
                content.push(glyph);
            } else if glyph == closer {
                if depth == 0 {
                    self.offset = here;
                    return Ok(content);
                }
                depth -= 1;
                content.push(glyph);
            } else {
                content.push(glyph);
            }
        }
        Err(self.fault(
            opened,
            self.text.len(),
            Problem::Unterminated(Boundary::Parentheses),
        ))
    }
}
