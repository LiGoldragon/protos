//! The one shared raw recognizer, parameterized entirely by a sealed token profile.

use crate::block::{Atom, Block, Delimiter, PipeText};
use crate::error::{RecognizeError, SourcePosition};
use crate::profile::{
    BareTokenPolicy, CarrierBody, CarrierCapture, CarrierIdentity, CarrierRule, GlyphClassSet,
    RawProfile, TokenBoundary, TokenProfile, TriviaRule,
};

/// The raw-structure entry point. Language-specific lexical behavior is sealed
/// data in its [`TokenProfile`]; this type is the sole execution mechanism.
#[derive(Clone, Debug)]
pub struct Recognizer {
    profile: TokenProfile,
}

impl Recognizer {
    /// Compatibility constructor for the two historic Protos raw profiles.
    pub fn new(profile: RawProfile) -> Self {
        Self::with_token_profile(profile.token_profile())
    }

    pub fn with_token_profile(profile: TokenProfile) -> Self {
        Self { profile }
    }

    pub fn standard() -> Self {
        Self::with_token_profile(TokenProfile::standard())
    }

    pub fn nomos_extended() -> Self {
        Self::with_token_profile(TokenProfile::nomos_extended())
    }

    pub fn profile(&self) -> &TokenProfile {
        &self.profile
    }

    /// Recognize raw structure without assigning semantic meaning to any token.
    pub fn recognize(&self, source: &str) -> Result<Document, RecognizeError> {
        let mut reading = SourceReading::new(source, &self.profile);
        let root_objects = reading.read_document()?;
        Ok(Document::from_root_objects(root_objects))
    }
}

pub use crate::block::Document;

/// The raw-layer boundary. `Foreign` remains a typed placeholder; production
/// textual paths use `Recognizer` with language-supplied sealed data.
#[derive(Clone, Debug)]
pub enum RawLayer {
    Recognizer(Recognizer),
    Foreign(ForeignRawLayer),
}

impl RawLayer {
    pub fn standard() -> Self {
        Self::Recognizer(Recognizer::standard())
    }

    pub fn nomos_extended() -> Self {
        Self::Recognizer(Recognizer::nomos_extended())
    }

    pub fn recognizer(&self) -> Option<&Recognizer> {
        match self {
            Self::Recognizer(recognizer) => Some(recognizer),
            Self::Foreign(_) => None,
        }
    }

    pub fn foreign(&self) -> Option<&ForeignRawLayer> {
        match self {
            Self::Foreign(foreign) => Some(foreign),
            Self::Recognizer(_) => None,
        }
    }
}

/// A typed placeholder naming a surface outside the shared profile mechanism.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ForeignRawLayer {
    language: ForeignLanguage,
}

impl ForeignRawLayer {
    pub fn new(language: ForeignLanguage) -> Self {
        Self { language }
    }

    pub fn language(&self) -> &ForeignLanguage {
        &self.language
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ForeignLanguage {
    name: String,
}

impl ForeignLanguage {
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }

    pub fn rust() -> Self {
        Self::new("rust")
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}

/// A cursor over one source plus the sealed lexical authority that reads it.
struct SourceReading<'source, 'profile> {
    source: &'source str,
    cursor: Cursor,
    profile: &'profile TokenProfile,
}

impl<'source, 'profile> SourceReading<'source, 'profile> {
    fn new(source: &'source str, profile: &'profile TokenProfile) -> Self {
        Self {
            source,
            cursor: Cursor::start(),
            profile,
        }
    }

    fn read_document(&mut self) -> Result<Vec<Block>, RecognizeError> {
        let mut root_objects = Vec::new();
        loop {
            self.skip_trivia()?;
            if self.peek().is_none() {
                return Ok(root_objects);
            }
            if let Some((_, closing)) = self.closing_delimiter() {
                return Err(RecognizeError::UnexpectedClose {
                    found: closing.chars().next().unwrap_or('\0'),
                    position: self.cursor.position(),
                });
            }
            root_objects.push(self.read_object()?);
        }
    }

    fn read_object(&mut self) -> Result<Block, RecognizeError> {
        let head = self.read_primary()?;
        let application = &self.profile.spec().application.text;
        if !self.starts_with(application) {
            return Ok(head);
        }
        let application_position = self.cursor.position();
        self.consume(application);
        if !self.at_primary_start()? {
            return Err(RecognizeError::DanglingApplication {
                position: application_position,
            });
        }
        let payload = self.read_object()?;
        Ok(Block::Application {
            head: Box::new(head),
            payload: Box::new(payload),
        })
    }

    fn read_primary(&mut self) -> Result<Block, RecognizeError> {
        if let Some(carrier) = self.carrier_match()? {
            return Ok(self.consume_carrier(carrier));
        }
        if let Some((delimiter, opening)) = self.opening_delimiter() {
            let opening = opening.to_owned();
            return self.read_delimited(delimiter, &opening);
        }
        if let Some((_, closing)) = self.closing_delimiter() {
            return Err(RecognizeError::UnexpectedClose {
                found: closing.chars().next().unwrap_or('\0'),
                position: self.cursor.position(),
            });
        }
        let application = &self.profile.spec().application.text;
        if self.starts_with(application) {
            let position = self.cursor.position();
            return if application == "." {
                Err(RecognizeError::UnexpectedDot { position })
            } else {
                Err(RecognizeError::UnexpectedApplicationToken {
                    token: application.clone(),
                    position,
                })
            };
        }
        if let Some(punctuation) = self.longest_punctuation() {
            let text = punctuation.to_owned();
            self.consume(&text);
            return Ok(Block::Atom(Atom::new(text)));
        }
        self.read_bare_token()
    }

    fn at_primary_start(&self) -> Result<bool, RecognizeError> {
        if self.peek().is_none()
            || self.starts_trivia()
            || self.closing_delimiter().is_some()
            || self.starts_with(&self.profile.spec().application.text)
        {
            return Ok(false);
        }
        Ok(true)
    }

    fn read_delimited(
        &mut self,
        delimiter: Delimiter,
        opening: &str,
    ) -> Result<Block, RecognizeError> {
        let start = self.cursor.position();
        self.consume(opening);
        let closing = self.profile.delimiter(delimiter).closing.clone();
        let mut root_objects = Vec::new();
        loop {
            self.skip_trivia()?;
            if self.peek().is_none() {
                return Err(RecognizeError::UnclosedDelimiter {
                    delimiter,
                    position: start,
                });
            }
            if self.starts_with(&closing) {
                self.consume(&closing);
                return Ok(Block::Delimited {
                    delimiter,
                    root_objects,
                });
            }
            if let Some((_, found)) = self.closing_delimiter() {
                return Err(RecognizeError::UnexpectedClose {
                    found: found.chars().next().unwrap_or('\0'),
                    position: self.cursor.position(),
                });
            }
            root_objects.push(self.read_object()?);
        }
    }

    fn read_bare_token(&mut self) -> Result<Block, RecognizeError> {
        let start = self.cursor.position();
        while self.peek().is_some()
            && !self.starts_structural_token()?
            && !self.starts_trivia()
        {
            self.bump();
        }
        let end = self.cursor.position();
        let text = self.source[start.byte_offset..end.byte_offset].to_owned();
        if text.is_empty() {
            let glyph = self.peek().unwrap_or('\0');
            self.bump();
            return Err(RecognizeError::UnsupportedGlyph {
                glyph,
                position: start,
            });
        }
        self.validate_bare_token(&text, start)?;
        Ok(Block::Atom(Atom::new(text)))
    }

    fn validate_bare_token(
        &self,
        text: &str,
        position: SourcePosition,
    ) -> Result<(), RecognizeError> {
        match &self.profile.spec().bare_tokens {
            BareTokenPolicy::Unreserved { forbidden_glyphs } => {
                if let Some(glyph) = text.chars().find(|glyph| forbidden_glyphs.contains(*glyph)) {
                    return Err(RecognizeError::UnsupportedGlyph { glyph, position });
                }
            }
            BareTokenPolicy::Classed(boundaries) => {
                if !boundaries
                    .iter()
                    .any(|boundary| Self::boundary_accepts(boundary, text))
                {
                    return Err(RecognizeError::TokenBoundary {
                        token: text.to_owned(),
                        position,
                    });
                }
            }
        }
        Ok(())
    }

    fn boundary_accepts(boundary: &TokenBoundary, text: &str) -> bool {
        let mut characters = text.chars();
        characters
            .next()
            .is_some_and(|first| boundary.first.contains(first))
            && characters.all(|character| boundary.continuation.contains(character))
    }

    fn carrier_match(&self) -> Result<Option<CarrierMatch<'profile>>, RecognizeError> {
        let mut winner: Option<CarrierMatch<'profile>> = None;
        for rule in &self.profile.spec().carriers {
            if let Some(candidate) = self.match_carrier(rule)? {
                let replace = winner.as_ref().is_none_or(|current| {
                    (candidate.end, rule.prefix.len(), rule.identity.value())
                        > (
                            current.end,
                            current.rule.prefix.len(),
                            current.rule.identity.value(),
                        )
                });
                if replace {
                    winner = Some(candidate);
                }
            }
        }
        Ok(winner)
    }

    fn match_carrier(
        &self,
        rule: &'profile CarrierRule,
    ) -> Result<Option<CarrierMatch<'profile>>, RecognizeError> {
        if !self.starts_with(&rule.prefix) {
            return Ok(None);
        }
        let body_start = self.cursor.byte_offset + rule.prefix.len();
        match &rule.body {
            CarrierBody::Delimited { closing, escape } => {
                let (end, content) =
                    self.scan_delimited(body_start, closing, escape.as_deref(), rule.identity)?;
                Ok(Some(CarrierMatch {
                    rule,
                    end,
                    content: Some(content),
                }))
            }
            CarrierBody::Fenced {
                fence,
                opening,
                closing,
                minimum_fences,
                maximum_fences,
            } => {
                let mut cursor = body_start;
                let mut fences = 0u8;
                while fences < *maximum_fences && self.char_at(cursor) == Some(*fence) {
                    cursor += fence.len_utf8();
                    fences += 1;
                }
                if fences < *minimum_fences || !self.source[cursor..].starts_with(opening) {
                    return Ok(None);
                }
                cursor += opening.len();
                let close = format!("{closing}{}", fence.to_string().repeat(fences.into()));
                let Some(relative) = self.source[cursor..].find(&close) else {
                    return Err(RecognizeError::UnclosedCarrier {
                        identity: rule.identity,
                        position: self.cursor.position(),
                    });
                };
                let content_end = cursor + relative;
                Ok(Some(CarrierMatch {
                    rule,
                    end: content_end + close.len(),
                    content: Some(self.source[cursor..content_end].to_owned()),
                }))
            }
            CarrierBody::ClassRun {
                first,
                continuation,
            } => Ok(self
                .scan_class_run(body_start, first, continuation)
                .map(|end| CarrierMatch {
                    rule,
                    end,
                    content: None,
                })),
            CarrierBody::DelimitedOrClassRun {
                closing,
                escape,
                first,
                continuation,
            } => {
                if let Ok((end, content)) =
                    self.scan_delimited(body_start, closing, escape.as_deref(), rule.identity)
                {
                    return Ok(Some(CarrierMatch {
                        rule,
                        end,
                        content: Some(content),
                    }));
                }
                Ok(self
                    .scan_class_run(body_start, first, continuation)
                    .map(|end| CarrierMatch {
                        rule,
                        end,
                        content: None,
                    }))
            }
        }
    }

    fn scan_delimited(
        &self,
        mut cursor: usize,
        closing: &str,
        escape: Option<&str>,
        identity: CarrierIdentity,
    ) -> Result<(usize, String), RecognizeError> {
        let mut content = String::new();
        while cursor < self.source.len() {
            if self.source[cursor..].starts_with(closing) {
                return Ok((cursor + closing.len(), content));
            }
            if let Some(escape) = escape {
                if self.source[cursor..].starts_with(escape) {
                    cursor += escape.len();
                    if let Some(character) = self.char_at(cursor) {
                        content.push(character);
                        cursor += character.len_utf8();
                    } else {
                        content.push_str(escape);
                    }
                    continue;
                }
            }
            let character = self.char_at(cursor).expect("cursor is before source end");
            content.push(character);
            cursor += character.len_utf8();
        }
        let error = if identity == CarrierIdentity::new(0)
            && self.profile.content_carrier().is_some_and(|carrier| carrier.identity == identity)
        {
            RecognizeError::UnclosedPipeText {
                position: self.cursor.position(),
            }
        } else {
            RecognizeError::UnclosedCarrier {
                identity,
                position: self.cursor.position(),
            }
        };
        Err(error)
    }

    fn scan_class_run(
        &self,
        mut cursor: usize,
        first: &GlyphClassSet,
        continuation: &GlyphClassSet,
    ) -> Option<usize> {
        let initial = self.char_at(cursor)?;
        if !first.contains(initial) {
            return None;
        }
        cursor += initial.len_utf8();
        while let Some(character) = self.char_at(cursor) {
            if !continuation.contains(character) {
                break;
            }
            cursor += character.len_utf8();
        }
        Some(cursor)
    }

    fn consume_carrier(&mut self, carrier: CarrierMatch<'profile>) -> Block {
        let start = self.cursor.byte_offset;
        self.consume_through(carrier.end);
        match carrier.rule.capture {
            CarrierCapture::WholeToken => {
                Block::Atom(Atom::new(self.source[start..carrier.end].to_owned()))
            }
            CarrierCapture::Content => Block::PipeText(PipeText::new(
                carrier.content.unwrap_or_else(String::new),
            )),
        }
    }

    fn skip_trivia(&mut self) -> Result<(), RecognizeError> {
        loop {
            if self.has_whitespace_trivia() && self.peek().is_some_and(char::is_whitespace) {
                self.bump();
                continue;
            }
            let Some(rule) = self.longest_comment() else {
                return Ok(());
            };
            match rule {
                TriviaRule::Whitespace => unreachable!(),
                TriviaRule::LineComment { opening } => {
                    let opening = opening.clone();
                    self.consume(&opening);
                    while let Some(character) = self.bump() {
                        if character == '\n' {
                            break;
                        }
                    }
                }
                TriviaRule::BlockComment {
                    opening,
                    closing,
                    nested,
                } => {
                    let opening = opening.clone();
                    let closing = closing.clone();
                    let nested = *nested;
                    let start = self.cursor.position();
                    self.consume(&opening);
                    let mut depth = 1usize;
                    while depth > 0 {
                        if self.peek().is_none() {
                            return Err(RecognizeError::UnclosedBlockComment { position: start });
                        }
                        if nested && self.starts_with(&opening) {
                            self.consume(&opening);
                            depth += 1;
                        } else if self.starts_with(&closing) {
                            self.consume(&closing);
                            depth -= 1;
                        } else {
                            self.bump();
                        }
                    }
                }
            }
        }
    }

    fn starts_trivia(&self) -> bool {
        (self.has_whitespace_trivia() && self.peek().is_some_and(char::is_whitespace))
            || self.longest_comment().is_some()
    }

    fn has_whitespace_trivia(&self) -> bool {
        self.profile
            .spec()
            .trivia
            .iter()
            .any(|rule| matches!(rule, TriviaRule::Whitespace))
    }

    fn longest_comment(&self) -> Option<&TriviaRule> {
        self.profile
            .spec()
            .trivia
            .iter()
            .filter(|rule| match rule {
                TriviaRule::Whitespace => false,
                TriviaRule::LineComment { opening }
                | TriviaRule::BlockComment { opening, .. } => self.starts_with(opening),
            })
            .max_by_key(|rule| match rule {
                TriviaRule::Whitespace => 0,
                TriviaRule::LineComment { opening }
                | TriviaRule::BlockComment { opening, .. } => opening.len(),
            })
    }

    fn starts_structural_token(&self) -> Result<bool, RecognizeError> {
        if self.opening_delimiter().is_some()
            || self.closing_delimiter().is_some()
            || self.starts_with(&self.profile.spec().application.text)
            || self.longest_punctuation().is_some()
        {
            return Ok(true);
        }
        for carrier in &self.profile.spec().carriers {
            if !carrier.prefix.is_empty() && self.match_carrier(carrier)?.is_some() {
                return Ok(true);
            }
        }
        Ok(false)
    }

    fn opening_delimiter(&self) -> Option<(Delimiter, &str)> {
        self.profile
            .spec()
            .delimiters
            .iter()
            .filter(|tokens| self.starts_with(&tokens.opening))
            .max_by_key(|tokens| tokens.opening.len())
            .map(|tokens| (tokens.delimiter, tokens.opening.as_str()))
    }

    fn closing_delimiter(&self) -> Option<(Delimiter, &str)> {
        self.profile
            .spec()
            .delimiters
            .iter()
            .filter(|tokens| self.starts_with(&tokens.closing))
            .max_by_key(|tokens| tokens.closing.len())
            .map(|tokens| (tokens.delimiter, tokens.closing.as_str()))
    }

    fn longest_punctuation(&self) -> Option<&str> {
        self.profile
            .spec()
            .punctuation
            .iter()
            .filter(|punctuation| self.starts_with(&punctuation.text))
            .max_by_key(|punctuation| punctuation.text.len())
            .map(|punctuation| punctuation.text.as_str())
    }

    fn starts_with(&self, text: &str) -> bool {
        self.source[self.cursor.byte_offset..].starts_with(text)
    }

    fn char_at(&self, byte_offset: usize) -> Option<char> {
        self.source.get(byte_offset..)?.chars().next()
    }

    fn peek(&self) -> Option<char> {
        self.char_at(self.cursor.byte_offset)
    }

    fn bump(&mut self) -> Option<char> {
        let character = self.peek()?;
        self.cursor.advance(character);
        Some(character)
    }

    fn consume(&mut self, text: &str) {
        debug_assert!(self.starts_with(text));
        for character in text.chars() {
            let consumed = self.bump();
            debug_assert_eq!(consumed, Some(character));
        }
    }

    fn consume_through(&mut self, end: usize) {
        while self.cursor.byte_offset < end {
            self.bump();
        }
        debug_assert_eq!(self.cursor.byte_offset, end);
    }
}

struct CarrierMatch<'profile> {
    rule: &'profile CarrierRule,
    end: usize,
    content: Option<String>,
}

#[derive(Clone, Copy, Debug)]
struct Cursor {
    byte_offset: usize,
    line: usize,
    column: usize,
}

impl Cursor {
    const fn start() -> Self {
        Self {
            byte_offset: 0,
            line: 1,
            column: 1,
        }
    }

    const fn position(self) -> SourcePosition {
        SourcePosition {
            byte_offset: self.byte_offset,
            line: self.line,
            column: self.column,
        }
    }

    fn advance(&mut self, character: char) {
        self.byte_offset += character.len_utf8();
        if character == '\n' {
            self.line += 1;
            self.column = 1;
        } else {
            self.column += 1;
        }
    }
}
