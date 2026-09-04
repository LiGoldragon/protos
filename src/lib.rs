//! Universal structural substrate for the Protos family.
//!
//! This crate owns the sole character reader (delineation) and the sole
//! character writer (canonical print). All dialects ride on it.

use std::collections::BTreeMap;
use std::fmt;
use std::marker::PhantomData;

// ---------------------------------------------------------------------------
// Intrinsic scalars (type aliases, hand-written)
// ---------------------------------------------------------------------------

pub type Text = String;
pub type Integer = i64;
pub type Decimal = f64;
pub type Boolean = bool;
pub type Symbol = Text;

// ---------------------------------------------------------------------------
// Structural types
// ---------------------------------------------------------------------------

/// Byte offsets into the source text: start, end (half-open).
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Extent(pub Integer, pub Integer);

impl fmt::Debug for Extent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Extent({}, {})", self.0, self.1)
    }
}

/// The position of a protoform: indices from the root.
/// A head's body is at index 0.
pub type Path = Vec<Integer>;

/// Where each protoform of a delineation lies in its text.
pub type Situation = BTreeMap<Path, Extent>;

/// The three separators: `.` `!` `:`
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum Separator {
    Period,
    Exclamation,
    Colon,
}

impl fmt::Debug for Separator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Period => write!(f, "Period"),
            Self::Exclamation => write!(f, "Exclamation"),
            Self::Colon => write!(f, "Colon"),
        }
    }
}

impl Separator {
    /// The glyph character for this separator.
    pub fn glyph(self) -> char {
        match self {
            Self::Period => '.',
            Self::Exclamation => '!',
            Self::Colon => ':',
        }
    }
}

/// Structural enclosures: `{ }` `[ ]` `« »` `< >`
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum Enclosure {
    Braced,
    Bracketed,
    Guillemets,
    Angled,
}

impl fmt::Debug for Enclosure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Braced => write!(f, "Braced"),
            Self::Bracketed => write!(f, "Bracketed"),
            Self::Guillemets => write!(f, "Guillemets"),
            Self::Angled => write!(f, "Angled"),
        }
    }
}

/// Opaque boundaries: `\u{201C} \u{201D}` (curly quotes), `( )` (parentheses)
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum Boundary {
    CurlyQuotes,
    Parentheses,
}

impl fmt::Debug for Boundary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CurlyQuotes => write!(f, "CurlyQuotes"),
            Self::Parentheses => write!(f, "Parentheses"),
        }
    }
}

/// The head of a headed structure: either a bare symbol or a qualified symbol.
#[derive(Clone)]
pub enum Head {
    /// A bare symbol: `Name`
    Bare(Symbol),
    /// A symbol qualified by an angled enclosure: `Name<...>`
    Qualified(Symbol, Vec<Protoform>),
}

impl fmt::Debug for Head {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Bare(s) => f.debug_tuple("Bare").field(s).finish(),
            Self::Qualified(s, c) => f.debug_tuple("Qualified").field(s).field(c).finish(),
        }
    }
}

impl PartialEq for Head {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Bare(a), Self::Bare(b)) => a == b,
            (Self::Qualified(a1, a2), Self::Qualified(b1, b2)) => a1 == b1 && a2 == b2,
            _ => false,
        }
    }
}

impl Eq for Head {}

/// One structural value. Protoform carries no extent; extents are found on the
/// way in and computed when printing.
#[derive(Clone)]
pub enum Protoform {
    /// A head, a separator, and a body: `Head.body`
    Headed(Head, Separator, Box<Protoform>),
    /// A structural enclosure with children: `{ a b }` `[ a b ]` `« a b »` `<a b>`
    Enclosed(Enclosure, Vec<Protoform>),
    /// An opaque boundary with content: `\u{201C}content\u{201D}` or `(content)`
    Opaque(Boundary, Text),
    /// A bare word
    Bare(Symbol),
    /// A symbol qualified by an angled enclosure: `Vector<Text>`
    Qualified(Symbol, Vec<Protoform>),
}

impl fmt::Debug for Protoform {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Headed(h, s, b) => f.debug_tuple("Headed").field(h).field(s).field(b).finish(),
            Self::Enclosed(e, c) => f.debug_tuple("Enclosed").field(e).field(c).finish(),
            Self::Opaque(b, c) => f.debug_tuple("Opaque").field(b).field(c).finish(),
            Self::Bare(s) => f.debug_tuple("Bare").field(s).finish(),
            Self::Qualified(s, c) => f.debug_tuple("Qualified").field(s).field(c).finish(),
        }
    }
}

impl PartialEq for Protoform {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Headed(h1, s1, b1), Self::Headed(h2, s2, b2)) => {
                h1 == h2 && s1 == s2 && b1 == b2
            }
            (Self::Enclosed(e1, c1), Self::Enclosed(e2, c2)) => e1 == e2 && c1 == c2,
            (Self::Opaque(b1, c1), Self::Opaque(b2, c2)) => b1 == b2 && c1 == c2,
            (Self::Bare(s1), Self::Bare(s2)) => s1 == s2,
            (Self::Qualified(s1, c1), Self::Qualified(s2, c2)) => s1 == s2 && c1 == c2,
            _ => false,
        }
    }
}

impl Eq for Protoform {}

/// The structures of a text and where they lie.
#[derive(Clone)]
pub struct Delineation {
    pub protoforms: Vec<Protoform>,
    pub situation: Situation,
}

impl fmt::Debug for Delineation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Delineation")
            .field("protoforms", &self.protoforms)
            .field("situation", &self.situation)
            .finish()
    }
}

impl PartialEq for Delineation {
    fn eq(&self, other: &Self) -> bool {
        self.protoforms == other.protoforms
    }
}

impl Eq for Delineation {}

/// A structural fault, situated.
#[derive(Clone, PartialEq, Eq)]
pub struct Fault {
    pub extent: Extent,
    pub problem: Problem,
}

impl fmt::Debug for Fault {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Fault")
            .field("extent", &self.extent)
            .field("problem", &self.problem)
            .finish()
    }
}

/// The structural fault taxonomy.
#[derive(Clone, PartialEq, Eq)]
pub enum Problem {
    Unclosed(Enclosure),
    UnclosedBoundary(Boundary),
    Unopened,
    MissingBody,
    MissingHead,
    EmptyInput,
}

impl fmt::Debug for Problem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unclosed(e) => write!(f, "Unclosed({e:?})"),
            Self::UnclosedBoundary(b) => write!(f, "UnclosedBoundary({b:?})"),
            Self::Unopened => write!(f, "Unopened"),
            Self::MissingBody => write!(f, "MissingBody"),
            Self::MissingHead => write!(f, "MissingHead"),
            Self::EmptyInput => write!(f, "EmptyInput"),
        }
    }
}

/// Text taken as a would-be T, untrusted until actualized.
/// The second type parameter C is the concept type of the dialect;
/// it defaults to () when no dialect is in scope.
pub struct Potential<T, C = ()>(Text, PhantomData<fn() -> (T, C)>);

impl<T, C> Potential<T, C> {
    /// The raw text.
    pub fn text(&self) -> &str {
        &self.0
    }
}

impl<T, C> From<Text> for Potential<T, C> {
    fn from(text: Text) -> Self {
        Self(text, PhantomData)
    }
}

impl<T, C> From<&str> for Potential<T, C> {
    fn from(s: &str) -> Self {
        Self(s.to_owned(), PhantomData)
    }
}

impl<T, C> Clone for Potential<T, C> {
    fn clone(&self) -> Self {
        Self(self.0.clone(), PhantomData)
    }
}

impl<T, C> fmt::Debug for Potential<T, C> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Potential").field(&self.0).finish()
    }
}

impl<T, C> PartialEq for Potential<T, C> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl<T, C> Eq for Potential<T, C> {}

/// A fault joined to its extent by actualize. Generic over the dialect's fault.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Situated<F>(pub Option<Extent>, pub F);

// ---------------------------------------------------------------------------
// Traits (kinds)
// ---------------------------------------------------------------------------

/// Borne by Text: delineate text into protoforms.
pub trait Structural {
    fn delineate(&self) -> Result<Delineation, Fault>;
}

/// Borne by Protoform and Delineation: canonical text.
pub trait Printing {
    fn print(&self) -> Text;
}

/// Borne by every concept: yield the protoform.
pub trait Protosizable {
    fn protosize(&self) -> Protoform;
}

/// Borne by Protoform, once per dialect: conceive a concept from a protoform.
pub trait Conceptual<C: Protosizable> {
    type Fault;
    fn conceive(&self) -> Result<C, Self::Fault>;
}

/// The corporal kind: a value that can be incorporated from a concept.
/// The concept type parameter C is the dialect's concept (e.g. Datom).
pub trait Corporal<C: Protosizable>: Embodied {
    type Fault;
    fn incorporate(concept: C) -> Result<Self, Self::Fault>;
}

/// Borne by `Potential<T, C>`: actualize a value from text.
pub trait Actualizable<T: Embodied> {
    type Fault;
    fn actualize(&self) -> Result<T, Self::Fault>;
}

/// A fault that knows where it is by path.
pub trait Pathed {
    fn path(&self) -> &[Integer];
}

/// Look up where a protoform was in the source text.
pub trait Situating {
    fn situate(&self, path: &[Integer]) -> Option<Extent>;
}

/// The bound: an alias of Sized, blanket-implemented.
pub trait Embodied: Sized {}
impl<T: Sized> Embodied for T {}

// ---------------------------------------------------------------------------
// Blanket Actualizable: delineate -> conceive -> incorporate
// ---------------------------------------------------------------------------

impl<C, T> Actualizable<T> for Potential<T, C>
where
    C: Protosizable,
    T: Corporal<C>,
    Delineation: Conceptual<C>,
    T::Fault: From<Fault> + From<<Delineation as Conceptual<C>>::Fault> + Pathed,
{
    type Fault = Situated<T::Fault>;

    fn actualize(&self) -> Result<T, Situated<T::Fault>> {
        let delineation = self.text().to_owned().delineate().map_err(|f| {
            let extent = Some(f.extent);
            Situated(extent, T::Fault::from(f))
        })?;

        let concept = delineation.conceive().map_err(|f| {
            let fault = T::Fault::from(f);
            let extent = delineation.situate(fault.path());
            Situated(extent, fault)
        })?;

        T::incorporate(concept).map_err(|f| {
            let extent = delineation.situate(f.path());
            Situated(extent, f)
        })
    }
}

// ---------------------------------------------------------------------------
// Structural for Text (the delineator / parser)
// ---------------------------------------------------------------------------

/// Delimiter characters.
const OPEN_BRACE: char = '{';
const CLOSE_BRACE: char = '}';
const OPEN_BRACKET: char = '[';
const CLOSE_BRACKET: char = ']';
const OPEN_GUILLEMET: char = '\u{00AB}'; // «
const CLOSE_GUILLEMET: char = '\u{00BB}'; // »
const OPEN_ANGLE: char = '<';
const CLOSE_ANGLE: char = '>';
const OPEN_CURLY_QUOTE: char = '\u{201C}'; // "
const CLOSE_CURLY_QUOTE: char = '\u{201D}'; // "
const OPEN_PAREN: char = '(';
const CLOSE_PAREN: char = ')';
const COMMENT_CHAR: char = ';';
const ESCAPE_CHAR: char = '\\';

fn is_delimiter(c: char) -> bool {
    matches!(
        c,
        OPEN_BRACE
            | CLOSE_BRACE
            | OPEN_BRACKET
            | CLOSE_BRACKET
            | OPEN_GUILLEMET
            | CLOSE_GUILLEMET
            | OPEN_ANGLE
            | CLOSE_ANGLE
            | OPEN_CURLY_QUOTE
            | CLOSE_CURLY_QUOTE
            | OPEN_PAREN
            | CLOSE_PAREN
    )
}

fn is_separator(c: char) -> bool {
    matches!(c, '.' | '!' | ':')
}

fn separator_from_char(c: char) -> Separator {
    match c {
        '.' => Separator::Period,
        '!' => Separator::Exclamation,
        ':' => Separator::Colon,
        _ => unreachable!(),
    }
}

fn enclosure_for_opener(c: char) -> Option<Enclosure> {
    match c {
        OPEN_BRACE => Some(Enclosure::Braced),
        OPEN_BRACKET => Some(Enclosure::Bracketed),
        OPEN_GUILLEMET => Some(Enclosure::Guillemets),
        OPEN_ANGLE => Some(Enclosure::Angled),
        _ => None,
    }
}

fn closer_for_enclosure(e: Enclosure) -> char {
    match e {
        Enclosure::Braced => CLOSE_BRACE,
        Enclosure::Bracketed => CLOSE_BRACKET,
        Enclosure::Guillemets => CLOSE_GUILLEMET,
        Enclosure::Angled => CLOSE_ANGLE,
    }
}

fn is_closer(c: char) -> bool {
    matches!(
        c,
        CLOSE_BRACE
            | CLOSE_BRACKET
            | CLOSE_GUILLEMET
            | CLOSE_ANGLE
            | CLOSE_CURLY_QUOTE
            | CLOSE_PAREN
    )
}

/// Parse a bare run (word with separators) into a Protoform.
/// The run contains no whitespace and no delimiter glyphs.
/// Returns (protoform, [path -> (byte_start, byte_end)] entries).
fn parse_bare_run(
    run: &str,
    run_start: Integer,
    base_path: &[Integer],
) -> Result<(Protoform, Vec<(Path, Extent)>), Fault> {
    // Check for leading separator (MissingHead)
    if let Some(first_char) = run.chars().next() {
        if is_separator(first_char) {
            return Err(Fault {
                extent: Extent(run_start, run_start + first_char.len_utf8() as Integer),
                problem: Problem::MissingHead,
            });
        }
    }

    // Find the first separator that has a non-whitespace, non-closing, non-delimiter
    // character following it
    let mut char_iter = run.char_indices().peekable();
    while let Some((byte_offset, ch)) = char_iter.next() {
        if is_separator(ch) {
            // Check what follows
            if let Some(&(next_offset, next_ch)) = char_iter.peek() {
                if !next_ch.is_whitespace() && !is_closer(next_ch) && !is_delimiter(next_ch) {
                    // Split: head is run[..byte_offset], separator is ch, body is run[next_offset..]
                    let head = run[..byte_offset].to_owned();
                    let sep = separator_from_char(ch);
                    let body_str = &run[next_offset..];
                    let body_start = run_start + next_offset as Integer;

                    let mut situations = Vec::new();
                    let body_path: Path = base_path
                        .iter()
                        .copied()
                        .chain(std::iter::once(0))
                        .collect();

                    let (body, body_situations) = parse_bare_run(body_str, body_start, &body_path)?;

                    let head_extent = Extent(run_start, run_start + run.len() as Integer);
                    situations.push((base_path.to_vec(), head_extent));
                    situations.extend(body_situations);

                    return Ok((
                        Protoform::Headed(Head::Bare(head), sep, Box::new(body)),
                        situations,
                    ));
                } else {
                    // Separator at end of run or followed by whitespace/closer -> MissingBody
                    return Err(Fault {
                        extent: Extent(
                            run_start + byte_offset as Integer,
                            run_start + byte_offset as Integer + ch.len_utf8() as Integer,
                        ),
                        problem: Problem::MissingBody,
                    });
                }
            } else {
                // Separator at end of string -> MissingBody
                return Err(Fault {
                    extent: Extent(
                        run_start + byte_offset as Integer,
                        run_start + byte_offset as Integer + ch.len_utf8() as Integer,
                    ),
                    problem: Problem::MissingBody,
                });
            }
        }
    }

    // No separator found: just a bare symbol
    let extent = Extent(run_start, run_start + run.len() as Integer);
    let situations = vec![(base_path.to_vec(), extent)];
    Ok((Protoform::Bare(run.to_owned()), situations))
}

/// The core delineation parser.
struct Delineator<'a> {
    source: &'a str,
    pos: usize,
}

impl<'a> Delineator<'a> {
    fn new(source: &'a str) -> Self {
        Self { source, pos: 0 }
    }

    fn remaining(&self) -> &'a str {
        &self.source[self.pos..]
    }

    fn peek_char(&self) -> Option<char> {
        self.remaining().chars().next()
    }

    fn advance_char(&mut self) -> Option<char> {
        let c = self.remaining().chars().next()?;
        self.pos += c.len_utf8();
        Some(c)
    }

    fn skip_whitespace_and_comments(&mut self) {
        loop {
            // Skip whitespace
            let before = self.pos;
            while let Some(c) = self.peek_char() {
                if c.is_whitespace() {
                    self.advance_char();
                } else {
                    break;
                }
            }

            // Check for comment: `;` opens a comment to end of line
            if self.peek_char() == Some(COMMENT_CHAR) {
                while let Some(c) = self.advance_char() {
                    if c == '\n' {
                        break;
                    }
                }
                continue;
            }

            if self.pos == before {
                break;
            }
        }
    }

    /// Parse the contents of a structural enclosure (or the top level).
    /// `closer` is None for top level, Some(ch) for an enclosure.
    #[allow(clippy::type_complexity)]
    fn parse_contents(
        &mut self,
        closer: Option<char>,
        base_path: &[Integer],
    ) -> Result<(Vec<Protoform>, Vec<(Path, Extent)>), Fault> {
        let mut protoforms = Vec::new();
        let mut situations = Vec::new();
        let mut child_index: Integer = 0;

        loop {
            self.skip_whitespace_and_comments();

            if self.pos >= self.source.len() {
                if closer.is_some() {
                    // We never found the closer
                    return Err(Fault {
                        extent: Extent(0, 0),                          // Will be set by caller
                        problem: Problem::Unclosed(Enclosure::Braced), // Will be set by caller
                    });
                }
                break;
            }

            let c = self.peek_char().unwrap();

            // Check for closer
            if let Some(expected_closer) = closer {
                if c == expected_closer {
                    self.advance_char();
                    break;
                }
            }

            // Check for unexpected closer
            if is_closer(c) {
                let start = self.pos as Integer;
                self.advance_char();
                return Err(Fault {
                    extent: Extent(start, self.pos as Integer),
                    problem: Problem::Unopened,
                });
            }

            let child_path: Path = base_path
                .iter()
                .copied()
                .chain(std::iter::once(child_index))
                .collect();

            let (pf, pf_situations) = self.parse_one(&child_path)?;
            protoforms.push(pf);
            situations.extend(pf_situations);
            child_index += 1;
        }

        Ok((protoforms, situations))
    }

    /// Parse one protoform at the current position.
    fn parse_one(&mut self, path: &[Integer]) -> Result<(Protoform, Vec<(Path, Extent)>), Fault> {
        let c = self.peek_char().unwrap();

        // Structural enclosure opener
        if let Some(enclosure) = enclosure_for_opener(c) {
            let start = self.pos as Integer;
            self.advance_char(); // consume opener
            let children_path = path;
            match self.parse_contents(Some(closer_for_enclosure(enclosure)), children_path) {
                Ok((children, mut child_situations)) => {
                    let end = self.pos as Integer;
                    let extent = Extent(start, end);
                    child_situations.push((path.to_vec(), extent));
                    Ok((Protoform::Enclosed(enclosure, children), child_situations))
                }
                Err(_) => Err(Fault {
                    extent: Extent(start, self.source.len() as Integer),
                    problem: Problem::Unclosed(enclosure),
                }),
            }
        }
        // Curly quote opener
        else if c == OPEN_CURLY_QUOTE {
            let start = self.pos as Integer;
            self.advance_char(); // consume opener
            let content_start = self.pos;
            // Read until closing curly quote; everything inside is content
            loop {
                match self.advance_char() {
                    Some(CLOSE_CURLY_QUOTE) => break,
                    Some(_) => continue,
                    None => {
                        return Err(Fault {
                            extent: Extent(start, self.source.len() as Integer),
                            problem: Problem::UnclosedBoundary(Boundary::CurlyQuotes),
                        });
                    }
                }
            }
            let content =
                self.source[content_start..self.pos - CLOSE_CURLY_QUOTE.len_utf8()].to_owned();
            let end = self.pos as Integer;
            let extent = Extent(start, end);
            let situations = vec![(path.to_vec(), extent)];
            Ok((
                Protoform::Opaque(Boundary::CurlyQuotes, content),
                situations,
            ))
        }
        // Parenthesis opener
        else if c == OPEN_PAREN {
            let start = self.pos as Integer;
            self.advance_char(); // consume opener
            let mut content = String::new();
            let mut depth = 1u32;
            // Read by balance, building content with unescaping
            loop {
                match self.advance_char() {
                    Some(ESCAPE_CHAR) => {
                        // The next char is escaped (e.g., `\)` -> `)`)
                        if let Some(escaped) = self.advance_char() {
                            content.push(escaped);
                        }
                    }
                    Some(OPEN_PAREN) => {
                        depth += 1;
                        content.push(OPEN_PAREN);
                    }
                    Some(CLOSE_PAREN) => {
                        depth -= 1;
                        if depth == 0 {
                            break;
                        }
                        content.push(CLOSE_PAREN);
                    }
                    Some(ch) => content.push(ch),
                    None => {
                        return Err(Fault {
                            extent: Extent(start, self.source.len() as Integer),
                            problem: Problem::UnclosedBoundary(Boundary::Parentheses),
                        });
                    }
                }
            }
            let end = self.pos as Integer;
            let extent = Extent(start, end);
            let situations = vec![(path.to_vec(), extent)];
            Ok((
                Protoform::Opaque(Boundary::Parentheses, content),
                situations,
            ))
        }
        // Bare run: accumulate characters until whitespace or delimiter
        else {
            let start = self.pos;
            while let Some(c) = self.peek_char() {
                if c.is_whitespace() || is_delimiter(c) || c == COMMENT_CHAR {
                    break;
                }
                self.advance_char();
            }
            let run = &self.source[start..self.pos];

            // Check if the bare run is immediately followed by `<` (no whitespace)
            // AND the run does NOT end with a separator (that case is handled below):
            // this creates a Qualified structure, e.g. `Vector<Text>`
            let run_ends_with_sep =
                !run.is_empty() && run.chars().next_back().is_some_and(is_separator);
            let run_has_internal_sep = run.chars().any(is_separator);
            if !run.is_empty()
                && !run_ends_with_sep
                && !run_has_internal_sep
                && self.peek_char() == Some(OPEN_ANGLE)
            {
                let angle_start = self.pos as Integer;
                self.advance_char(); // consume `<`
                match self.parse_contents(Some(CLOSE_ANGLE), path) {
                    Ok((children, mut child_situations)) => {
                        let end = self.pos as Integer;
                        let extent = Extent(start as Integer, end);
                        child_situations.push((path.to_vec(), extent));

                        // Parse the bare run for internal separators to get the symbol
                        // The run itself is a bare word (no separators expected before <)
                        let symbol = run.to_owned();

                        // Check if a separator follows the `>` (creating a Headed with Qualified head)
                        if let Some(next_c) = self.peek_char() {
                            if is_separator(next_c) {
                                let sep = separator_from_char(next_c);
                                self.advance_char(); // consume separator
                                // Check what follows the separator
                                if let Some(after_sep) = self.peek_char() {
                                    if !after_sep.is_whitespace()
                                        && !is_closer(after_sep)
                                        && after_sep != COMMENT_CHAR
                                    {
                                        let body_path: Path = path
                                            .iter()
                                            .copied()
                                            .chain(std::iter::once(0))
                                            .collect();
                                        let (body, body_situations) = self.parse_one(&body_path)?;
                                        child_situations.extend(body_situations);
                                        // Update extent
                                        if let Some(entry) =
                                            child_situations.iter_mut().find(|(p, _)| p == path)
                                        {
                                            entry.1 = Extent(start as Integer, self.pos as Integer);
                                        }
                                        return Ok((
                                            Protoform::Headed(
                                                Head::Qualified(symbol, children),
                                                sep,
                                                Box::new(body),
                                            ),
                                            child_situations,
                                        ));
                                    }
                                }
                                // Separator with nothing after -> MissingBody
                                return Err(Fault {
                                    extent: Extent(self.pos as Integer - 1, self.pos as Integer),
                                    problem: Problem::MissingBody,
                                });
                            }
                        }

                        // Standalone Qualified
                        return Ok((Protoform::Qualified(symbol, children), child_situations));
                    }
                    Err(_) => {
                        return Err(Fault {
                            extent: Extent(angle_start, self.source.len() as Integer),
                            problem: Problem::Unclosed(Enclosure::Angled),
                        });
                    }
                }
            }

            // If the run has internal separators AND `<` follows, the separator
            // rule must apply first: `LockPaths.Vector<LockPath>` becomes
            // Headed(Bare(LockPaths), Period, Qualified(Vector, [Bare(LockPath)])).
            // Rewind past the body portion so parse_one sees `Vector<LockPath>`.
            if run_has_internal_sep && self.peek_char() == Some(OPEN_ANGLE) {
                // Find the first separator in the run
                for (byte_offset, ch) in run.char_indices() {
                    if is_separator(ch) {
                        let head_str = &run[..byte_offset];
                        if head_str.is_empty() {
                            return Err(Fault {
                                extent: Extent(
                                    start as Integer,
                                    start as Integer + ch.len_utf8() as Integer,
                                ),
                                problem: Problem::MissingHead,
                            });
                        }
                        let sep = separator_from_char(ch);
                        let body_text_start = start + byte_offset + ch.len_utf8();
                        // Rewind self.pos to the body start so parse_one picks up the body
                        self.pos = body_text_start;
                        let body_path: Path =
                            path.iter().copied().chain(std::iter::once(0)).collect();
                        let (body, body_situations) = self.parse_one(&body_path)?;
                        let (head_pf, mut head_situations) =
                            parse_bare_run(head_str, start as Integer, path)?;
                        let result = attach_body_to_deepest(head_pf, sep, body);
                        head_situations.extend(body_situations);
                        if let Some(entry) = head_situations.iter_mut().find(|(p, _)| p == path) {
                            entry.1 = Extent(start as Integer, self.pos as Integer);
                        }
                        return Ok((result, head_situations));
                    }
                }
            }

            // Check if the run ends with a separator and the next char is an opener
            if !run.is_empty() {
                let last_char = run.chars().next_back().unwrap();
                if is_separator(last_char) {
                    // Check what follows
                    if let Some(next_c) = self.peek_char() {
                        if enclosure_for_opener(next_c).is_some()
                            || next_c == OPEN_CURLY_QUOTE
                            || next_c == OPEN_PAREN
                        {
                            let sep_byte_offset = run.len() - last_char.len_utf8();
                            let head_str = &run[..sep_byte_offset];
                            if head_str.is_empty() {
                                return Err(Fault {
                                    extent: Extent(
                                        start as Integer,
                                        start as Integer + last_char.len_utf8() as Integer,
                                    ),
                                    problem: Problem::MissingHead,
                                });
                            }
                            let sep = separator_from_char(last_char);

                            let body_path: Path =
                                path.iter().copied().chain(std::iter::once(0)).collect();
                            let (body, body_situations) = self.parse_one(&body_path)?;

                            let (head_pf, mut head_situations) =
                                parse_bare_run(head_str, start as Integer, path)?;

                            let result = attach_body_to_deepest(head_pf, sep, body);
                            head_situations.extend(body_situations);
                            if let Some(entry) = head_situations.iter_mut().find(|(p, _)| p == path)
                            {
                                entry.1 = Extent(start as Integer, self.pos as Integer);
                            }
                            return Ok((result, head_situations));
                        }
                    }
                }
            }

            // Standard bare run parsing
            parse_bare_run(run, start as Integer, path)
        }
    }
}

/// Attach a body to the deepest rightmost position in a headed chain.
/// e.g., for head=Headed(A, Period, Bare(B)), sep=Colon, body=Enclosed(...)
/// -> Headed(A, Period, Headed(B, Colon, Enclosed(...)))
fn attach_body_to_deepest(head: Protoform, sep: Separator, body: Protoform) -> Protoform {
    match head {
        Protoform::Bare(symbol) => Protoform::Headed(Head::Bare(symbol), sep, Box::new(body)),
        Protoform::Qualified(symbol, quals) => {
            Protoform::Headed(Head::Qualified(symbol, quals), sep, Box::new(body))
        }
        Protoform::Headed(h, s, inner) => {
            Protoform::Headed(h, s, Box::new(attach_body_to_deepest(*inner, sep, body)))
        }
        other => {
            // Shouldn't happen in normal parsing, but handle gracefully
            Protoform::Headed(Head::Bare(String::new()), sep, Box::new(other))
        }
    }
}

impl Structural for Text {
    fn delineate(&self) -> Result<Delineation, Fault> {
        let mut delineator = Delineator::new(self);
        let (protoforms, situation_entries) = delineator.parse_contents(None, &[])?;
        let mut situation = Situation::new();
        for (path, extent) in situation_entries {
            situation.insert(path, extent);
        }
        Ok(Delineation {
            protoforms,
            situation,
        })
    }
}

impl Structural for Potential<()> {
    fn delineate(&self) -> Result<Delineation, Fault> {
        self.0.delineate()
    }
}

// ---------------------------------------------------------------------------
// Printing for Protoform and Delineation
// ---------------------------------------------------------------------------

impl Printing for Head {
    fn print(&self) -> Text {
        match self {
            Head::Bare(symbol) => symbol.clone(),
            Head::Qualified(symbol, children) => {
                let inner: Vec<String> = children.iter().map(|c| c.print()).collect();
                let joined = inner.join(" ");
                format!("{symbol}<{joined}>")
            }
        }
    }
}

impl Printing for Protoform {
    fn print(&self) -> Text {
        match self {
            Protoform::Headed(head, sep, body) => {
                let mut result = head.print();
                result.push(sep.glyph());
                result.push_str(&body.print());
                result
            }
            Protoform::Enclosed(enclosure, children) => {
                let (open, close) = match enclosure {
                    Enclosure::Braced => ("{", "}"),
                    Enclosure::Bracketed => ("[", "]"),
                    Enclosure::Guillemets => ("\u{00AB}", "\u{00BB}"),
                    Enclosure::Angled => ("<", ">"),
                };
                if children.is_empty() {
                    // Empty enclosures are tight
                    format!("{open}{close}")
                } else {
                    let inner: Vec<String> = children.iter().map(|c| c.print()).collect();
                    let joined = inner.join(" ");
                    match enclosure {
                        // Angled: always tight (no space inside)
                        Enclosure::Angled => format!("{open}{joined}{close}"),
                        // Others: space inside at both ends
                        _ => format!("{open} {joined} {close}"),
                    }
                }
            }
            Protoform::Opaque(boundary, content) => match boundary {
                Boundary::CurlyQuotes => {
                    format!("\u{201C}{content}\u{201D}")
                }
                Boundary::Parentheses => {
                    // Escape unbalanced `)` as `\)` in content
                    let escaped = escape_parens_for_print(content);
                    format!("({escaped})")
                }
            },
            Protoform::Bare(symbol) => symbol.clone(),
            Protoform::Qualified(symbol, children) => {
                Head::Qualified(symbol.clone(), children.clone()).print()
            }
        }
    }
}

/// Escape unbalanced `)` in parenthesized content for printing.
fn escape_parens_for_print(content: &str) -> String {
    let mut result = String::with_capacity(content.len());
    let mut depth: i32 = 0;

    // First pass: find which `)` are unbalanced.
    // Actually, the content stores the raw content between the opening `(` and closing `)`.
    // When printing, we need to ensure that reading it back by balance produces the same content.
    // An unbalanced `)` in the content would close the outer parenthesis prematurely.
    // So we escape any `)` that would cause depth to go below 0.
    for c in content.chars() {
        match c {
            '(' => {
                depth += 1;
                result.push(c);
            }
            ')' => {
                if depth > 0 {
                    depth -= 1;
                    result.push(c);
                } else {
                    result.push('\\');
                    result.push(')');
                }
            }
            _ => result.push(c),
        }
    }
    result
}

impl Printing for Delineation {
    fn print(&self) -> Text {
        let parts: Vec<String> = self.protoforms.iter().map(|p| p.print()).collect();
        parts.join(" ")
    }
}

// ---------------------------------------------------------------------------
// Situating for Delineation
// ---------------------------------------------------------------------------

impl Situating for Delineation {
    fn situate(&self, path: &[Integer]) -> Option<Extent> {
        self.situation.get(path).copied()
    }
}
