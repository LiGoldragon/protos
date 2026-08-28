//! The universal structural boundary for Protos dialects.
//!
//! A dialect receives `Portion` values and supplies its own type anatomy. This
//! crate owns the sole character reader and, in the next slice, the sole
//! character writer.

use std::fmt;

pub struct Text {
    normalized: String,
    content_hash: ContentHash,
    delineation: Option<Delineation>,
}

pub struct ContentHash(u64);

pub struct Symbol(String);

pub struct Extent {
    pub start: usize,
    pub end: usize,
}

pub enum Separator {
    Period,
    Exclamation,
    Colon,
}

pub enum Enclosure {
    Braced,
    Bracketed,
    Guillemets,
    Angled,
    CurlyQuote,
}

/// A dialect-owned delimiter recognized by the common reader without becoming
/// one of Protos's five universal enclosures.
pub enum Boundary {
    Universal(Enclosure),
    Dialect(DialectBoundary),
}

pub enum DialectBoundary {
    Parentheses,
}

/// One structural value. Its variant is its inline anatomy and carries the
/// value's one half-open UTF-8 byte extent.
pub enum Portion {
    Headed(Extent, Headed),
    Enclosed(Extent, Enclosed),
    Bare(Extent, Bare),
}

pub struct Headed {
    pub head: Symbol,
    pub separator: Separator,
    pub body: Box<Portion>,
}

pub struct Enclosed {
    pub boundary: Boundary,
    pub arity: usize,
    pub contents: EnclosedContents,
}

pub enum EnclosedContents {
    Portions(Vec<Portion>),
    Opaque(String),
}

pub struct Bare {
    pub symbol: Symbol,
}

pub struct Delineation {
    pub portions: Vec<Portion>,
}

pub struct Prospective<T> {
    pub text: Text,
    target: std::marker::PhantomData<fn() -> T>,
}

pub struct Fault {
    pub extent: Extent,
    pub problem: FaultProblem,
}

pub enum FaultProblem {
    UnexpectedCloser,
    UnclosedDelimiter,
    MissingHead,
    MissingBody,
    ExpectedOnePortion,
    ExpectedShape,
}

pub enum Layout {
    Flat,
}

pub trait Delineatable {
    type Delineation;

    fn delineate(&self) -> Result<Self::Delineation, Fault>;
}

pub trait Embodiable {
    type Embodied: Embodied;

    fn embody(&self) -> Result<Self::Embodied, Fault>;
}

/// A final Rust type owns its inbound Portion anatomy.
pub trait Embodied: Sized {
    fn from_portion(portion: &Portion) -> Result<Self, Fault>;
}

/// A final Rust type owns its outbound Portion anatomy; Protos prints it.
pub trait Textualizable: Embodied {
    fn to_portion(&self) -> Portion;

    fn textualize(&self) -> Text {
        self.to_portion().print(Layout::Flat)
    }
}

/// A shape predicate used by a dialect to select an anatomy, never a parser.
pub trait ShapeDefined: Embodied {
    fn matches(portion: &Portion) -> bool;
}

pub trait ContentHashable {
    fn content_hash(&self) -> ContentHash;
}

/// The only Protos capability which writes structural characters.
pub trait Printing {
    fn print(&self, layout: Layout) -> Text;
}

/// Lets callers retain the extents computed by the writer without re-reading
/// the text it just wrote.
pub trait DelineatedText {
    fn delineation(&self) -> Option<&Delineation>;
}

impl From<&str> for Text {
    fn from(value: &str) -> Self {
        let mut normalizer = Normalizer {
            input: value,
            cursor: 0,
            output: String::new(),
        };
        let normalized = normalizer.normalize();
        let content_hash = TextHasher.hash(&normalized);
        Self {
            normalized,
            content_hash,
            delineation: None,
        }
    }
}

impl From<String> for Text {
    fn from(value: String) -> Self {
        Self::from(value.as_str())
    }
}

impl AsRef<str> for Text {
    fn as_ref(&self) -> &str {
        &self.normalized
    }
}

impl From<&str> for Symbol {
    fn from(value: &str) -> Self {
        Self(value.to_owned())
    }
}

impl AsRef<str> for Symbol {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl AsRef<Extent> for Portion {
    fn as_ref(&self) -> &Extent {
        match self {
            Self::Headed(extent, _) | Self::Enclosed(extent, _) | Self::Bare(extent, _) => extent,
        }
    }
}

impl Delineatable for Text {
    type Delineation = Delineation;

    fn delineate(&self) -> Result<Self::Delineation, Fault> {
        let mut parser = Parser {
            input: self.as_ref(),
            cursor: 0,
        };
        parser.delineate_document()
    }
}

impl<T> From<Text> for Prospective<T> {
    fn from(text: Text) -> Self {
        Self {
            text,
            target: std::marker::PhantomData,
        }
    }
}

impl<T> Delineatable for Prospective<T> {
    type Delineation = Delineation;

    fn delineate(&self) -> Result<Self::Delineation, Fault> {
        self.text.delineate()
    }
}

impl<T: Embodied> Embodiable for Prospective<T> {
    type Embodied = T;

    fn embody(&self) -> Result<Self::Embodied, Fault> {
        let delineation = self.delineate()?;
        if delineation.portions.len() != 1 {
            return Err(Fault {
                extent: Extent {
                    start: 0,
                    end: self.text.as_ref().len(),
                },
                problem: FaultProblem::ExpectedOnePortion,
            });
        }
        T::from_portion(&delineation.portions[0])
    }
}

impl ContentHashable for Text {
    fn content_hash(&self) -> ContentHash {
        ContentHash(self.content_hash.0)
    }
}

impl DelineatedText for Text {
    fn delineation(&self) -> Option<&Delineation> {
        self.delineation.as_ref()
    }
}

impl Printing for Delineation {
    fn print(&self, layout: Layout) -> Text {
        let mut printer = Printer {
            output: String::new(),
        };
        let delineation = printer.delineation(self, layout);
        let content_hash = TextHasher.hash(&printer.output);
        Text {
            normalized: printer.output,
            content_hash,
            delineation: Some(delineation),
        }
    }
}

impl Printing for Portion {
    fn print(&self, layout: Layout) -> Text {
        let mut printer = Printer {
            output: String::new(),
        };
        let portion = printer.portion(self, layout);
        let content_hash = TextHasher.hash(&printer.output);
        Text {
            normalized: printer.output,
            content_hash,
            delineation: Some(Delineation {
                portions: vec![portion],
            }),
        }
    }
}

struct Printer {
    output: String,
}

trait Rendering {
    fn delineation(&mut self, delineation: &Delineation, layout: Layout) -> Delineation;
    fn portion(&mut self, portion: &Portion, layout: Layout) -> Portion;
    fn headed(&mut self, headed: &Headed, layout: Layout) -> Headed;
    fn enclosed(&mut self, enclosed: &Enclosed, layout: Layout) -> Enclosed;
    fn bare(&mut self, bare: &Bare) -> Bare;
    fn emit_parenthetical_payload(&mut self, payload: &str);
    fn delimiter(&self, boundary: Boundary) -> &'static DelimiterSpec;
    fn emit(&mut self, text: &str);
}

impl Rendering for Printer {
    fn delineation(&mut self, delineation: &Delineation, layout: Layout) -> Delineation {
        let mut portions = Vec::with_capacity(delineation.portions.len());
        for portion in &delineation.portions {
            if !self.output.is_empty() {
                match layout {
                    Layout::Flat => self.emit(" "),
                }
            }
            portions.push(self.portion(portion, layout));
        }
        Delineation { portions }
    }

    fn portion(&mut self, portion: &Portion, layout: Layout) -> Portion {
        let start = self.output.len();
        match portion {
            Portion::Headed(_, headed) => {
                let anatomy = self.headed(headed, layout);
                Portion::Headed(
                    Extent {
                        start,
                        end: self.output.len(),
                    },
                    anatomy,
                )
            }
            Portion::Enclosed(_, enclosed) => {
                let anatomy = self.enclosed(enclosed, layout);
                Portion::Enclosed(
                    Extent {
                        start,
                        end: self.output.len(),
                    },
                    anatomy,
                )
            }
            Portion::Bare(_, bare) => {
                let anatomy = self.bare(bare);
                Portion::Bare(
                    Extent {
                        start,
                        end: self.output.len(),
                    },
                    anatomy,
                )
            }
        }
    }

    fn headed(&mut self, headed: &Headed, layout: Layout) -> Headed {
        self.emit(headed.head.as_ref());
        self.emit(match headed.separator {
            Separator::Period => ".",
            Separator::Exclamation => "!",
            Separator::Colon => ":",
        });
        Headed {
            head: Symbol::from(headed.head.as_ref()),
            separator: headed.separator,
            body: Box::new(self.portion(&headed.body, layout)),
        }
    }

    fn enclosed(&mut self, enclosed: &Enclosed, layout: Layout) -> Enclosed {
        let delimiter = self.delimiter(enclosed.boundary);
        self.emit(delimiter.opening);
        let contents = match &enclosed.contents {
            EnclosedContents::Portions(portions) => {
                let mut printed = Vec::with_capacity(portions.len());
                for portion in portions {
                    if !printed.is_empty() {
                        match layout {
                            Layout::Flat => self.emit(" "),
                        }
                    }
                    printed.push(self.portion(portion, layout));
                }
                EnclosedContents::Portions(printed)
            }
            EnclosedContents::Opaque(value) => {
                match enclosed.boundary {
                    Boundary::Dialect(DialectBoundary::Parentheses) => {
                        self.emit_parenthetical_payload(value)
                    }
                    Boundary::Universal(_) => self.emit(value),
                }
                EnclosedContents::Opaque(value.to_owned())
            }
        };
        self.emit(delimiter.closing);
        Enclosed {
            boundary: enclosed.boundary,
            arity: enclosed.arity,
            contents,
        }
    }

    fn bare(&mut self, bare: &Bare) -> Bare {
        self.emit(bare.symbol.as_ref());
        Bare {
            symbol: Symbol::from(bare.symbol.as_ref()),
        }
    }

    fn emit_parenthetical_payload(&mut self, payload: &str) {
        let mut unmatched_openings = Vec::new();
        for character in payload.chars() {
            match character {
                '\\' => self.emit("\\\\"),
                '(' => {
                    unmatched_openings.push(self.output.len());
                    self.emit("(");
                }
                ')' => {
                    if unmatched_openings.pop().is_some() {
                        self.emit(")");
                    } else {
                        self.emit("\\)");
                    }
                }
                _ => self.emit(&character.to_string()),
            }
        }
        for position in unmatched_openings.into_iter().rev() {
            self.output.insert(position, '\\');
        }
    }

    fn delimiter(&self, boundary: Boundary) -> &'static DelimiterSpec {
        DELIMITERS
            .iter()
            .find(|delimiter| delimiter.boundary == boundary)
            .expect("every parsed boundary has one universal delimiter specification")
    }

    fn emit(&mut self, text: &str) {
        self.output.push_str(text);
    }
}

struct DelimiterSpec {
    boundary: Boundary,
    opening: &'static str,
    closing: &'static str,
    handling: DelimiterHandling,
}

enum DelimiterHandling {
    Structural,
    Opaque,
    BalancedOpaque,
}

static DELIMITERS: [DelimiterSpec; 6] = [
    DelimiterSpec {
        boundary: Boundary::Universal(Enclosure::Braced),
        opening: "{",
        closing: "}",
        handling: DelimiterHandling::Structural,
    },
    DelimiterSpec {
        boundary: Boundary::Universal(Enclosure::Bracketed),
        opening: "[",
        closing: "]",
        handling: DelimiterHandling::Structural,
    },
    DelimiterSpec {
        boundary: Boundary::Universal(Enclosure::Guillemets),
        opening: "«",
        closing: "»",
        handling: DelimiterHandling::Structural,
    },
    DelimiterSpec {
        boundary: Boundary::Universal(Enclosure::Angled),
        opening: "<",
        closing: ">",
        handling: DelimiterHandling::Structural,
    },
    DelimiterSpec {
        boundary: Boundary::Universal(Enclosure::CurlyQuote),
        opening: "“",
        closing: "”",
        handling: DelimiterHandling::Opaque,
    },
    DelimiterSpec {
        boundary: Boundary::Dialect(DialectBoundary::Parentheses),
        opening: "(",
        closing: ")",
        handling: DelimiterHandling::BalancedOpaque,
    },
];

struct TextHasher;

trait Hashing {
    fn hash(&self, text: &str) -> ContentHash;
}

impl Hashing for TextHasher {
    fn hash(&self, text: &str) -> ContentHash {
        let mut value = 0xcbf29ce484222325_u64;
        for byte in text.bytes() {
            value ^= u64::from(byte);
            value = value.wrapping_mul(0x100000001b3);
        }
        ContentHash(value)
    }
}

struct Normalizer<'input> {
    input: &'input str,
    cursor: usize,
    output: String,
}

trait Normalizing {
    fn normalize(&mut self) -> String;
    fn copy_opaque(&mut self, delimiter: &DelimiterSpec);
    fn copy_balanced_opaque(&mut self, delimiter: &DelimiterSpec);
    fn skip_whitespace(&mut self);
    fn needs_structural_space(&self) -> bool;
    fn opening(&self) -> Option<&'static DelimiterSpec>;
    fn next_character(&self) -> Option<char>;
    fn advance_character(&mut self);
    fn emit(&mut self, text: &str);
}

impl Normalizing for Normalizer<'_> {
    fn normalize(&mut self) -> String {
        let mut pending_whitespace = false;
        while self.cursor < self.input.len() {
            if self.next_character().is_some_and(char::is_whitespace) {
                self.skip_whitespace();
                pending_whitespace = true;
                continue;
            }
            if pending_whitespace && self.needs_structural_space() {
                self.emit(" ");
            }
            pending_whitespace = false;
            if let Some(delimiter) = self.opening() {
                match delimiter.handling {
                    DelimiterHandling::Structural => {
                        self.emit(delimiter.opening);
                        self.cursor += delimiter.opening.len();
                    }
                    DelimiterHandling::Opaque => self.copy_opaque(delimiter),
                    DelimiterHandling::BalancedOpaque => self.copy_balanced_opaque(delimiter),
                }
            } else if let Some(character) = self.next_character() {
                self.emit(&self.input[self.cursor..self.cursor + character.len_utf8()]);
                self.advance_character();
            }
        }
        self.output.to_owned()
    }

    fn copy_opaque(&mut self, delimiter: &DelimiterSpec) {
        self.emit(delimiter.opening);
        self.cursor += delimiter.opening.len();
        let content_start = self.cursor;
        while self.cursor < self.input.len()
            && !self.input[self.cursor..].starts_with(delimiter.closing)
        {
            self.advance_character();
        }
        self.emit(&self.input[content_start..self.cursor]);
        if self.cursor < self.input.len() {
            self.emit(delimiter.closing);
            self.cursor += delimiter.closing.len();
        }
    }

    fn copy_balanced_opaque(&mut self, delimiter: &DelimiterSpec) {
        let opaque_start = self.cursor;
        self.cursor += delimiter.opening.len();
        let mut depth = 1_usize;
        while self.cursor < self.input.len() && depth > 0 {
            if self.input[self.cursor..].starts_with(delimiter.opening) {
                depth += 1;
                self.cursor += delimiter.opening.len();
            } else if self.input[self.cursor..].starts_with(delimiter.closing) {
                depth -= 1;
                self.cursor += delimiter.closing.len();
            } else {
                self.advance_character();
            }
        }
        self.emit(&self.input[opaque_start..self.cursor]);
    }

    fn skip_whitespace(&mut self) {
        while self.next_character().is_some_and(char::is_whitespace) {
            self.advance_character();
        }
    }

    fn needs_structural_space(&self) -> bool {
        let previous = self.output.chars().last();
        let following = self.next_character();
        match (previous, following) {
            (Some(previous), Some(following)) => {
                !matches!(previous, '{' | '[' | '«' | '<' | '(' | '.' | '!' | ':')
                    && !matches!(following, '}' | ']' | '»' | '>' | ')' | '.' | '!' | ':')
            }
            _ => false,
        }
    }

    fn opening(&self) -> Option<&'static DelimiterSpec> {
        DELIMITERS
            .iter()
            .find(|delimiter| self.input[self.cursor..].starts_with(delimiter.opening))
    }

    fn next_character(&self) -> Option<char> {
        self.input[self.cursor..].chars().next()
    }

    fn advance_character(&mut self) {
        if let Some(character) = self.next_character() {
            self.cursor += character.len_utf8();
        }
    }

    fn emit(&mut self, text: &str) {
        self.output.push_str(text);
    }
}

struct Parser<'input> {
    input: &'input str,
    cursor: usize,
}

trait Parsing {
    fn delineate_document(&mut self) -> Result<Delineation, Fault>;
    fn portions_until(
        &mut self,
        closing: Option<(&'static str, usize)>,
    ) -> Result<Vec<Portion>, Fault>;
    fn portion(&mut self) -> Result<Portion, Fault>;
    fn enclosed(&mut self, start: usize, delimiter: &DelimiterSpec) -> Result<Portion, Fault>;
    fn opaque(&mut self, start: usize, delimiter: &DelimiterSpec) -> Result<Portion, Fault>;
    fn balanced_opaque(
        &mut self,
        start: usize,
        delimiter: &DelimiterSpec,
    ) -> Result<Portion, Fault>;
    fn bare_or_headed(&mut self) -> Result<Portion, Fault>;
    fn skip_whitespace(&mut self);
    fn matching_opening(&self) -> Option<&'static DelimiterSpec>;
    fn matching_closer(&self) -> Option<&'static DelimiterSpec>;
    fn next_character(&self) -> Option<char>;
    fn advance_character(&mut self);
    fn separator_from(&self, character: char) -> Option<Separator>;
    fn fault(&self, start: usize, problem: FaultProblem) -> Fault;
}

impl Parsing for Parser<'_> {
    fn delineate_document(&mut self) -> Result<Delineation, Fault> {
        self.portions_until(None)
            .map(|portions| Delineation { portions })
    }

    fn portions_until(
        &mut self,
        closing: Option<(&'static str, usize)>,
    ) -> Result<Vec<Portion>, Fault> {
        let mut portions = Vec::new();
        loop {
            self.skip_whitespace();
            if let Some((expected, _)) = closing {
                if self.input[self.cursor..].starts_with(expected) {
                    self.cursor += expected.len();
                    return Ok(portions);
                }
            }
            if self.cursor == self.input.len() {
                return match closing {
                    Some((_, start)) => Err(self.fault(start, FaultProblem::UnclosedDelimiter)),
                    None => Ok(portions),
                };
            }
            if let Some(closer) = self.matching_closer() {
                return Err(Fault {
                    extent: Extent {
                        start: self.cursor,
                        end: self.cursor + closer.closing.len(),
                    },
                    problem: FaultProblem::UnexpectedCloser,
                });
            }
            portions.push(self.portion()?);
        }
    }

    fn portion(&mut self) -> Result<Portion, Fault> {
        let start = self.cursor;
        match self.matching_opening() {
            Some(delimiter) => {
                self.cursor += delimiter.opening.len();
                match delimiter.handling {
                    DelimiterHandling::Structural => self.enclosed(start, delimiter),
                    DelimiterHandling::Opaque => self.opaque(start, delimiter),
                    DelimiterHandling::BalancedOpaque => self.balanced_opaque(start, delimiter),
                }
            }
            None => self.bare_or_headed(),
        }
    }

    fn enclosed(&mut self, start: usize, delimiter: &DelimiterSpec) -> Result<Portion, Fault> {
        let portions = self.portions_until(Some((delimiter.closing, start)))?;
        let arity = portions.len();
        Ok(Portion::Enclosed(
            Extent {
                start,
                end: self.cursor,
            },
            Enclosed {
                boundary: delimiter.boundary,
                arity,
                contents: EnclosedContents::Portions(portions),
            },
        ))
    }

    fn opaque(&mut self, start: usize, delimiter: &DelimiterSpec) -> Result<Portion, Fault> {
        let content_start = self.cursor;
        while self.cursor < self.input.len()
            && !self.input[self.cursor..].starts_with(delimiter.closing)
        {
            self.advance_character();
        }
        if self.cursor == self.input.len() {
            return Err(self.fault(start, FaultProblem::UnclosedDelimiter));
        }
        let content = self.input[content_start..self.cursor].to_owned();
        self.cursor += delimiter.closing.len();
        Ok(Portion::Enclosed(
            Extent {
                start,
                end: self.cursor,
            },
            Enclosed {
                boundary: delimiter.boundary,
                arity: 0,
                contents: EnclosedContents::Opaque(content),
            },
        ))
    }

    fn balanced_opaque(
        &mut self,
        start: usize,
        delimiter: &DelimiterSpec,
    ) -> Result<Portion, Fault> {
        let mut content = String::new();
        let mut depth = 1_usize;
        while self.cursor < self.input.len() {
            if self.input[self.cursor..].starts_with('\\') {
                self.cursor += 1;
                if let Some(character) = self.next_character() {
                    content.push(character);
                    self.advance_character();
                } else {
                    return Err(self.fault(start, FaultProblem::UnclosedDelimiter));
                }
            } else if self.input[self.cursor..].starts_with(delimiter.opening) {
                depth += 1;
                content.push_str(delimiter.opening);
                self.cursor += delimiter.opening.len();
            } else if self.input[self.cursor..].starts_with(delimiter.closing) {
                depth -= 1;
                if depth == 0 {
                    self.cursor += delimiter.closing.len();
                    return Ok(Portion::Enclosed(
                        Extent {
                            start,
                            end: self.cursor,
                        },
                        Enclosed {
                            boundary: delimiter.boundary,
                            arity: 0,
                            contents: EnclosedContents::Opaque(content),
                        },
                    ));
                }
                content.push_str(delimiter.closing);
                self.cursor += delimiter.closing.len();
            } else {
                content.push(self.next_character().expect("cursor is in bounds"));
                self.advance_character();
            }
        }
        Err(self.fault(start, FaultProblem::UnclosedDelimiter))
    }

    fn bare_or_headed(&mut self) -> Result<Portion, Fault> {
        let start = self.cursor;
        let mut separator = None;
        while let Some(character) = self.next_character() {
            if character.is_whitespace()
                || self.matching_opening().is_some()
                || self.matching_closer().is_some()
            {
                break;
            }
            if let Some(found) = self.separator_from(character) {
                separator = Some(found);
                break;
            }
            self.advance_character();
        }
        if self.cursor == start {
            let end = self
                .next_character()
                .map_or(start, |character| start + character.len_utf8());
            return Err(Fault {
                extent: Extent { start, end },
                problem: FaultProblem::MissingHead,
            });
        }
        let symbol = Symbol::from(&self.input[start..self.cursor]);
        match separator {
            Some(separator) => {
                self.advance_character();
                if self.cursor == self.input.len()
                    || self.next_character().is_some_and(char::is_whitespace)
                    || self.matching_closer().is_some()
                {
                    return Err(self.fault(self.cursor, FaultProblem::MissingBody));
                }
                let body = self.portion()?;
                let end = body.as_ref().end;
                Ok(Portion::Headed(
                    Extent { start, end },
                    Headed {
                        head: symbol,
                        separator,
                        body: Box::new(body),
                    },
                ))
            }
            None => Ok(Portion::Bare(
                Extent {
                    start,
                    end: self.cursor,
                },
                Bare { symbol },
            )),
        }
    }

    fn skip_whitespace(&mut self) {
        while self.next_character().is_some_and(char::is_whitespace) {
            self.advance_character();
        }
    }

    fn matching_opening(&self) -> Option<&'static DelimiterSpec> {
        DELIMITERS
            .iter()
            .find(|delimiter| self.input[self.cursor..].starts_with(delimiter.opening))
    }

    fn matching_closer(&self) -> Option<&'static DelimiterSpec> {
        DELIMITERS
            .iter()
            .find(|delimiter| self.input[self.cursor..].starts_with(delimiter.closing))
    }

    fn next_character(&self) -> Option<char> {
        self.input[self.cursor..].chars().next()
    }

    fn advance_character(&mut self) {
        if let Some(character) = self.next_character() {
            self.cursor += character.len_utf8();
        }
    }

    fn separator_from(&self, character: char) -> Option<Separator> {
        match character {
            '.' => Some(Separator::Period),
            '!' => Some(Separator::Exclamation),
            ':' => Some(Separator::Colon),
            _ => None,
        }
    }

    fn fault(&self, start: usize, problem: FaultProblem) -> Fault {
        Fault {
            extent: Extent {
                start,
                end: self.cursor,
            },
            problem,
        }
    }
}

impl Copy for Separator {}

impl Clone for Separator {
    fn clone(&self) -> Self {
        *self
    }
}

impl Copy for Enclosure {}

impl Clone for Enclosure {
    fn clone(&self) -> Self {
        *self
    }
}

impl Copy for Boundary {}

impl Clone for Boundary {
    fn clone(&self) -> Self {
        *self
    }
}

impl Copy for Layout {}

impl Clone for Layout {
    fn clone(&self) -> Self {
        *self
    }
}

impl PartialEq for Text {
    fn eq(&self, other: &Self) -> bool {
        self.normalized == other.normalized
    }
}

impl Eq for Text {}

impl PartialEq for ContentHash {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl Eq for ContentHash {}

impl PartialEq for Symbol {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl Eq for Symbol {}

impl PartialEq for Extent {
    fn eq(&self, other: &Self) -> bool {
        self.start == other.start && self.end == other.end
    }
}

impl Eq for Extent {}

impl PartialEq for Separator {
    fn eq(&self, other: &Self) -> bool {
        matches!(
            (self, other),
            (Self::Period, Self::Period)
                | (Self::Exclamation, Self::Exclamation)
                | (Self::Colon, Self::Colon)
        )
    }
}

impl Eq for Separator {}

impl PartialEq for Enclosure {
    fn eq(&self, other: &Self) -> bool {
        matches!(
            (self, other),
            (Self::Braced, Self::Braced)
                | (Self::Bracketed, Self::Bracketed)
                | (Self::Guillemets, Self::Guillemets)
                | (Self::Angled, Self::Angled)
                | (Self::CurlyQuote, Self::CurlyQuote)
        )
    }
}

impl Eq for Enclosure {}

impl PartialEq for Boundary {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Universal(left), Self::Universal(right)) => left == right,
            (Self::Dialect(left), Self::Dialect(right)) => left == right,
            _ => false,
        }
    }
}

impl Eq for Boundary {}

impl Copy for DialectBoundary {}

impl Clone for DialectBoundary {
    fn clone(&self) -> Self {
        *self
    }
}

impl PartialEq for DialectBoundary {
    fn eq(&self, other: &Self) -> bool {
        matches!((self, other), (Self::Parentheses, Self::Parentheses))
    }
}

impl Eq for DialectBoundary {}

impl PartialEq for Portion {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Headed(left_extent, left), Self::Headed(right_extent, right)) => {
                left_extent == right_extent && left == right
            }
            (Self::Enclosed(left_extent, left), Self::Enclosed(right_extent, right)) => {
                left_extent == right_extent && left == right
            }
            (Self::Bare(left_extent, left), Self::Bare(right_extent, right)) => {
                left_extent == right_extent && left == right
            }
            _ => false,
        }
    }
}

impl Eq for Portion {}

impl PartialEq for Headed {
    fn eq(&self, other: &Self) -> bool {
        self.head == other.head && self.separator == other.separator && self.body == other.body
    }
}

impl Eq for Headed {}

impl PartialEq for Enclosed {
    fn eq(&self, other: &Self) -> bool {
        self.boundary == other.boundary
            && self.arity == other.arity
            && self.contents == other.contents
    }
}

impl Eq for Enclosed {}

impl PartialEq for EnclosedContents {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Portions(left), Self::Portions(right)) => left == right,
            (Self::Opaque(left), Self::Opaque(right)) => left == right,
            _ => false,
        }
    }
}

impl Eq for EnclosedContents {}

impl PartialEq for Bare {
    fn eq(&self, other: &Self) -> bool {
        self.symbol == other.symbol
    }
}

impl Eq for Bare {}

impl PartialEq for Delineation {
    fn eq(&self, other: &Self) -> bool {
        self.portions == other.portions
    }
}

impl Eq for Delineation {}

impl PartialEq for Fault {
    fn eq(&self, other: &Self) -> bool {
        self.extent == other.extent && self.problem == other.problem
    }
}

impl Eq for Fault {}

impl PartialEq for FaultProblem {
    fn eq(&self, other: &Self) -> bool {
        matches!(
            (self, other),
            (Self::UnexpectedCloser, Self::UnexpectedCloser)
                | (Self::UnclosedDelimiter, Self::UnclosedDelimiter)
                | (Self::MissingHead, Self::MissingHead)
                | (Self::MissingBody, Self::MissingBody)
                | (Self::ExpectedOnePortion, Self::ExpectedOnePortion)
                | (Self::ExpectedShape, Self::ExpectedShape)
        )
    }
}

impl Eq for FaultProblem {}

macro_rules! debug_as_display {
    ($($type:ty),+ $(,)?) => {
        $(impl fmt::Debug for $type {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(formatter, "{}", stringify!($type))
            }
        })+
    };
}

debug_as_display!(
    Text,
    ContentHash,
    Symbol,
    Extent,
    Separator,
    Enclosure,
    Boundary,
    DialectBoundary,
    Portion,
    Headed,
    Enclosed,
    EnclosedContents,
    Bare,
    Delineation,
    Fault,
    FaultProblem,
    Layout,
);
