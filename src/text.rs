//! Text: the string that can be quoted, refused at construction if it cannot.

use std::fmt;

use crate::anatomy::{Boundary, Integer, Refusal, Text};
use crate::kinds::Delimiting;

impl TryFrom<String> for Text {
    type Error = Refusal;

    fn try_from(string: String) -> Result<Self, Refusal> {
        let closer = Boundary::CurlyQuotes.closer();
        for (offset, glyph) in string.char_indices() {
            if glyph == closer {
                return Err(Refusal {
                    glyph,
                    offset: offset as Integer,
                });
            }
        }
        Ok(Text(string))
    }
}

impl TryFrom<&str> for Text {
    type Error = Refusal;

    fn try_from(string: &str) -> Result<Self, Refusal> {
        Text::try_from(string.to_owned())
    }
}

impl AsRef<str> for Text {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl std::ops::Deref for Text {
    type Target = str;

    fn deref(&self) -> &str {
        &self.0
    }
}

impl From<Text> for String {
    fn from(text: Text) -> String {
        text.0
    }
}

impl fmt::Display for Text {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl fmt::Display for Refusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "glyph {:?} at offset {} cannot be carried by Text",
            self.glyph, self.offset
        )
    }
}

impl std::error::Error for Refusal {}
