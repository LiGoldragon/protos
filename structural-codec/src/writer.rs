//! Canonical Block→text emission through the same sealed lexical data used by
//! raw recognition.

use raw_discovery::{Block, CarrierBody, Delimiter, TokenProfile};

use crate::error::TextualProfileError;

/// Render a discovered block under the compatibility profile or under explicit
/// sealed language data.
pub trait CanonicalText {
    fn canonical_text(&self) -> String;

    fn canonical_text_with(
        &self,
        profile: &TokenProfile,
    ) -> Result<String, TextualProfileError>;
}

impl CanonicalText for Block {
    fn canonical_text(&self) -> String {
        self.canonical_text_with(&TokenProfile::standard())
            .expect("the standard profile renders every Protos block")
    }

    fn canonical_text_with(
        &self,
        profile: &TokenProfile,
    ) -> Result<String, TextualProfileError> {
        CanonicalEmitter::new(profile).emit(self)
    }
}

/// The data-bearing emission capability. It holds the sealed profile that
/// decides every token spelling and attachment.
struct CanonicalEmitter<'profile> {
    profile: &'profile TokenProfile,
}

impl<'profile> CanonicalEmitter<'profile> {
    fn new(profile: &'profile TokenProfile) -> Self {
        Self { profile }
    }

    fn emit(&self, block: &Block) -> Result<String, TextualProfileError> {
        match block {
            Block::Atom(atom) => Ok(atom.text().to_owned()),
            Block::PipeText(pipe) => self.emit_content_carrier(pipe.text()),
            Block::Application { head, payload } => Ok(format!(
                "{}{}{}",
                self.emit(head)?,
                self.profile.spec().application.text,
                self.emit(payload)?
            )),
            Block::Delimited {
                delimiter,
                root_objects,
            } => self.emit_delimited(*delimiter, root_objects),
        }
    }

    fn emit_content_carrier(&self, content: &str) -> Result<String, TextualProfileError> {
        let carrier = self
            .profile
            .content_carrier()
            .ok_or(TextualProfileError::MissingContentCarrier)?;
        let CarrierBody::Delimited { closing, escape } = &carrier.body else {
            return Err(TextualProfileError::InvalidContentCarrier);
        };
        let escaped = match escape {
            Some(escape) => {
                let escaped_escape = format!("{escape}{escape}");
                let escaped_close = format!("{escape}{closing}");
                content
                    .replace(escape, &escaped_escape)
                    .replace(closing, &escaped_close)
            }
            None => content.to_owned(),
        };
        Ok(format!("{}{escaped}{closing}", carrier.prefix))
    }

    fn emit_delimited(
        &self,
        delimiter: Delimiter,
        children: &[Block],
    ) -> Result<String, TextualProfileError> {
        let tokens = self.profile.delimiter(delimiter);
        let mut rendered = String::new();
        rendered.push_str(&tokens.opening);
        for (index, child) in children.iter().enumerate() {
            if index > 0 && self.separates(&children[index - 1], child) {
                rendered.push(' ');
            }
            rendered.push_str(&self.emit(child)?);
        }
        rendered.push_str(&tokens.closing);
        Ok(rendered)
    }

    fn separates(&self, left: &Block, right: &Block) -> bool {
        let left_attaches = left
            .atom()
            .and_then(|atom| self.profile.punctuation(atom.text()))
            .is_some_and(|punctuation| punctuation.attach_right);
        let right_attaches = right
            .atom()
            .and_then(|atom| self.profile.punctuation(atom.text()))
            .is_some_and(|punctuation| punctuation.attach_left);
        !left_attaches && !right_attaches
    }
}
