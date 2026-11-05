use std::collections::BTreeSet;
use std::convert::TryFrom;
use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct OutOfRange(pub usize);

impl fmt::Display for OutOfRange {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "value {} exceeds i64::MAX", self.0)
    }
}

impl std::error::Error for OutOfRange {}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(
    feature = "rkyv",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)
)]
pub struct Position(usize);

pub trait PositionAccess {
    fn get(self) -> usize;
}

impl PositionAccess for Position {
    fn get(self) -> usize {
        self.0
    }
}

impl TryFrom<usize> for Position {
    type Error = OutOfRange;

    fn try_from(value: usize) -> Result<Self, Self::Error> {
        (value <= i64::MAX as usize)
            .then_some(Self(value))
            .ok_or(OutOfRange(value))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ReversedExtent {
    pub start: Position,
    pub end: Position,
}

impl fmt::Display for ReversedExtent {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "extent starts at {} after {}",
            self.start.0, self.end.0
        )
    }
}

impl std::error::Error for ReversedExtent {}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(
    feature = "rkyv",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)
)]
pub struct Extent {
    start: Position,
    end: Position,
}

#[allow(clippy::new_ret_no_self)]
pub trait ExtentAccess {
    fn new(start: Position, end: Position) -> Result<Extent, ReversedExtent>;
    fn start(self) -> Position;
    fn end(self) -> Position;
    fn from_offsets(start: usize, end: usize) -> Extent;
}

impl ExtentAccess for Extent {
    fn new(start: Position, end: Position) -> Result<Self, ReversedExtent> {
        if start.0 <= end.0 {
            Ok(Self { start, end })
        } else {
            Err(ReversedExtent { start, end })
        }
    }
    fn start(self) -> Position {
        self.start
    }
    fn end(self) -> Position {
        self.end
    }
    fn from_offsets(start: usize, end: usize) -> Extent {
        Self::new(
            Position::try_from(start).expect("extent start exceeds position range"),
            Position::try_from(end).expect("extent end exceeds position range"),
        )
        .expect("canonical extent ordering")
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(
    feature = "rkyv",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)
)]
pub enum Separator {
    Period,
    Exclamation,
    Colon,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Enclosure {
    Braced,
    Bracketed,
    Angled,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Boundary {
    Guillemets,
    Parentheses,
}
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Symbol(pub String);
pub enum Protos {
    Headed {
        extent: Extent,
        head: Symbol,
        constraints: Option<Box<Protos>>,
        separator: Separator,
        body: Box<Protos>,
    },
    Enclosed {
        extent: Extent,
        enclosure: Enclosure,
        children: Vec<Protos>,
    },
    Opaque {
        extent: Extent,
        boundary: Boundary,
        content: String,
    },
    Bare {
        extent: Extent,
        text: String,
    },
}
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(
    feature = "rkyv",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)
)]
pub struct Error {
    pub extent: Extent,
    pub problem: Problem,
}
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(
    feature = "rkyv",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)
)]
pub enum Problem {
    Empty,
    Multiple,
    Unclosed(char),
    Unexpected(char),
    MissingHead,
    MissingBody,
    Budget,
    Depth,
}
/// The number of structural nodes one reading act may visit.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ReaderBudget {
    remaining: usize,
}

pub trait BudgetAccess {
    fn remaining(&self) -> usize;
}
impl BudgetAccess for ReaderBudget {
    fn remaining(&self) -> usize {
        self.remaining
    }
}

impl TryFrom<usize> for ReaderBudget {
    type Error = OutOfRange;

    fn try_from(value: usize) -> Result<Self, Self::Error> {
        (value <= i64::MAX as usize)
            .then_some(Self { remaining: value })
            .ok_or(OutOfRange(value))
    }
}
/// A budget a reading spends from, one structural node at a time.
pub trait Spendable {
    fn spend(&mut self) -> bool;
}
impl Spendable for ReaderBudget {
    fn spend(&mut self) -> bool {
        if self.remaining == 0 {
            false
        } else {
            self.remaining -= 1;
            true
        }
    }
}
pub trait BoundedProtosizable {
    fn protosize_with(&self, budget: &mut ReaderBudget) -> Result<Protos, Error>;
}
pub trait Protosizable {
    type Output;
    fn protosize(&self) -> Self::Output;
}
/// The canonical text, expanded vertically: nothing with a next layer
/// shares one line with a sibling that has one too.
pub trait Textualizable {
    fn textualize(&self) -> String;
}
/// The same text on one line, for a reader that takes one line at a time.
pub trait Compactable {
    fn compact(&self) -> String;
}
/// Assign the UTF-8 byte extents of this tree's canonical textual form.
///
/// Parsing retains the source extents it read. This capability is for a tree
/// built structurally, whose context is the canonical text it will print.
pub trait Canonicalizable {
    fn canonicalize(&mut self);
}

pub(crate) trait Glyphing {
    fn opener(self) -> char;
    fn closer(self) -> char;
}
impl Glyphing for Enclosure {
    fn opener(self) -> char {
        match self {
            Self::Braced => '{',
            Self::Bracketed => '[',
            Self::Angled => '<',
        }
    }
    fn closer(self) -> char {
        match self {
            Self::Braced => '}',
            Self::Bracketed => ']',
            Self::Angled => '>',
        }
    }
}
impl Glyphing for Boundary {
    fn opener(self) -> char {
        match self {
            Self::Guillemets => '«',
            Self::Parentheses => '(',
        }
    }
    fn closer(self) -> char {
        match self {
            Self::Guillemets => '»',
            Self::Parentheses => ')',
        }
    }
}
/// The escape rule of an opaque boundary.
///
/// Inside an opaque region a backslash escapes the boundary's own glyphs and
/// itself, and nothing else: `\X` for any other X is a literal backslash
/// followed by X. Escaping is minimal — the writer prefixes a backslash only
/// where leaving the glyph bare would read back as something else — so an
/// opaque region stays as close to verbatim as the boundary allows.
pub(crate) trait Escaping {
    /// The glyphs a backslash may escape, the backslash itself included.
    fn escapes(self) -> &'static [char];
    /// The content indices whose glyph the writer must escape.
    fn forced(self, content: &str) -> BTreeSet<usize>;
    /// Write `content` between this boundary's glyphs.
    fn print_opaque(self, content: &str, out: &mut String);
}
impl Escaping for Boundary {
    fn escapes(self) -> &'static [char] {
        match self {
            Self::Guillemets => &['\\', '»'],
            Self::Parentheses => &['\\', '(', ')'],
        }
    }
    fn forced(self, content: &str) -> BTreeSet<usize> {
        match self {
            // Guillemets do not nest: every closer inside is content.
            Self::Guillemets => content
                .char_indices()
                .filter_map(|(index, glyph)| (glyph == '»').then_some(index))
                .collect(),
            // Parentheses are read by balance, so a balanced pair is content
            // and only an unmatched parenthesis needs the backslash.
            Self::Parentheses => {
                let mut opens = Vec::new();
                let mut forced = BTreeSet::new();
                for (index, glyph) in content.char_indices() {
                    if glyph == '(' {
                        opens.push(index);
                    } else if glyph == ')' && opens.pop().is_none() {
                        forced.insert(index);
                    }
                }
                forced.extend(opens);
                forced
            }
        }
    }
    fn print_opaque(self, content: &str, out: &mut String) {
        let escapes = self.escapes();
        let forced = self.forced(content);
        out.push(self.opener());
        let mut glyphs = content.char_indices().peekable();
        while let Some((index, glyph)) = glyphs.next() {
            let next = glyphs.peek().map(|(_, glyph)| *glyph);
            let ambiguous = glyph == '\\' && next.is_none_or(|glyph| escapes.contains(&glyph));
            if ambiguous || forced.contains(&index) {
                out.push('\\');
            }
            out.push(glyph);
        }
        out.push(self.closer());
    }
}
pub(crate) trait Separating {
    fn glyph(self) -> char;
}
impl Separating for Separator {
    fn glyph(self) -> char {
        match self {
            Self::Period => '.',
            Self::Exclamation => '!',
            Self::Colon => ':',
        }
    }
}
trait Extenting {
    fn extent(&self) -> Extent;
}
impl Extenting for Protos {
    fn extent(&self) -> Extent {
        match self {
            Self::Headed { extent, .. }
            | Self::Enclosed { extent, .. }
            | Self::Opaque { extent, .. }
            | Self::Bare { extent, .. } => *extent,
        }
    }
}

struct Reader<'a> {
    text: &'a str,
    offset: usize,
    budget: &'a mut ReaderBudget,
    depth: usize,
}
const MAXIMUM_READER_DEPTH: usize = 256;
trait Reading {
    fn whole(&mut self) -> Result<Protos, Error>;
    fn node(&mut self) -> Result<Protos, Error>;
    fn enclosed(&mut self, enclosure: Enclosure) -> Result<Protos, Error>;
    fn opaque(&mut self, boundary: Boundary) -> Result<Protos, Error>;
    fn bare_or_headed(&mut self) -> Result<Protos, Error>;
    fn headed(
        &mut self,
        head: Symbol,
        constraints: Option<Protos>,
        start: usize,
        separator: Separator,
    ) -> Result<Protos, Error>;
    fn space(&mut self);
    fn glyph(&self) -> Option<char>;
    fn step(&mut self);
    fn failure(&self, problem: Problem, start: usize) -> Error;
}
impl Reading for Reader<'_> {
    fn whole(&mut self) -> Result<Protos, Error> {
        self.space();
        if self.glyph().is_none() {
            return Err(self.failure(Problem::Empty, self.offset));
        }
        let form = self.node()?;
        self.space();
        if self.glyph().is_some() {
            Err(self.failure(Problem::Multiple, self.offset))
        } else {
            Ok(form)
        }
    }
    fn node(&mut self) -> Result<Protos, Error> {
        if !self.budget.spend() {
            return Err(self.failure(Problem::Budget, self.offset));
        }
        if self.depth == MAXIMUM_READER_DEPTH {
            return Err(self.failure(Problem::Depth, self.offset));
        }
        self.depth += 1;
        self.space();
        let result = match self.glyph() {
            Some('{') => self.enclosed(Enclosure::Braced),
            Some('[') => self.enclosed(Enclosure::Bracketed),
            Some('<') => self.enclosed(Enclosure::Angled),
            Some('«') => self.opaque(Boundary::Guillemets),
            Some('(') => self.opaque(Boundary::Parentheses),
            Some('}' | ']' | '>' | '»' | ')') => Err(self.failure(
                Problem::Unexpected(self.glyph().expect("matched")),
                self.offset,
            )),
            Some(_) => self.bare_or_headed(),
            None => Err(self.failure(Problem::MissingBody, self.offset)),
        };
        self.depth -= 1;
        result
    }
    fn enclosed(&mut self, enclosure: Enclosure) -> Result<Protos, Error> {
        let start = self.offset;
        self.step();
        let mut children = Vec::new();
        loop {
            self.space();
            match self.glyph() {
                Some(glyph) if glyph == enclosure.closer() => {
                    self.step();
                    return Ok(Protos::Enclosed {
                        extent: Extent::from_offsets(start, self.offset),
                        enclosure,
                        children,
                    });
                }
                None => return Err(self.failure(Problem::Unclosed(enclosure.opener()), start)),
                _ => children.push(self.node()?),
            }
        }
    }
    fn opaque(&mut self, boundary: Boundary) -> Result<Protos, Error> {
        let start = self.offset;
        self.step();
        let mut content = String::new();
        let mut depth = 1usize;
        loop {
            let Some(glyph) = self.glyph() else {
                return Err(self.failure(Problem::Unclosed(boundary.opener()), start));
            };
            if glyph == '\\' {
                self.step();
                let Some(escaped) = self.glyph() else {
                    return Err(self.failure(Problem::Unclosed(boundary.opener()), start));
                };
                if !boundary.escapes().contains(&escaped) {
                    content.push('\\');
                }
                content.push(escaped);
                self.step();
                continue;
            }
            if boundary == Boundary::Parentheses && glyph == '(' {
                depth += 1;
                content.push(glyph);
                self.step();
                continue;
            }
            if glyph == boundary.closer() {
                depth -= 1;
                if depth == 0 {
                    self.step();
                    return Ok(Protos::Opaque {
                        extent: Extent::from_offsets(start, self.offset),
                        boundary,
                        content,
                    });
                }
                content.push(glyph);
                self.step();
                continue;
            }
            content.push(glyph);
            self.step();
        }
    }
    fn bare_or_headed(&mut self) -> Result<Protos, Error> {
        let start = self.offset;
        let run_end = self.text[start..]
            .char_indices()
            .find_map(|(relative, glyph)| {
                (glyph.is_whitespace()
                    || matches!(
                        glyph,
                        '{' | '}' | '[' | ']' | '<' | '>' | '«' | '»' | '(' | ')' | ';'
                    ))
                .then_some(start + relative)
            })
            .unwrap_or(self.text.len());
        let whole_run = &self.text[start..run_end];
        if let Some(dot) = whole_run.find('.') {
            let head = &whole_run[..dot];
            let body_start = start + dot + 1;
            let body_begins = self.text[body_start..]
                .chars()
                .next()
                .is_some_and(|glyph| !glyph.is_whitespace() && glyph != ';');
            if head.chars().next().is_some_and(char::is_uppercase)
                && head.chars().all(|glyph| !matches!(glyph, '.' | '!' | ':'))
                && body_begins
            {
                self.offset = body_start;
                return self.headed(Symbol(head.to_owned()), None, start, Separator::Period);
            }
        }
        let trailing_separators = whole_run
            .chars()
            .rev()
            .take_while(|glyph| matches!(glyph, '.' | '!' | ':'))
            .count();
        let segments_are_symbols = whole_run
            .split(['.', '!', ':'])
            .all(|segment| !segment.is_empty());
        let opens_body = trailing_separators == 1
            && whole_run[..whole_run.len() - 1]
                .split(['.', '!', ':'])
                .all(|segment| !segment.is_empty())
            && self.text[run_end..]
                .chars()
                .next()
                .is_some_and(|glyph| matches!(glyph, '{' | '[' | '<' | '«' | '('));
        if !segments_are_symbols && !opens_body {
            self.offset = run_end;
            return Ok(Protos::Bare {
                extent: Extent::from_offsets(start, self.offset),
                text: whole_run.to_owned(),
            });
        }
        let mut run = String::new();
        while let Some(glyph) = self.glyph() {
            if glyph.is_whitespace()
                || matches!(
                    glyph,
                    '{' | '}' | '[' | ']' | '>' | '«' | '»' | '(' | ')' | ';'
                )
            {
                break;
            }
            if matches!(glyph, '.' | '!' | ':') {
                let separator = match glyph {
                    '.' => Separator::Period,
                    '!' => Separator::Exclamation,
                    _ => Separator::Colon,
                };
                if run.is_empty() {
                    return Err(self.failure(Problem::MissingHead, start));
                }
                self.step();
                if self.glyph().is_none()
                    || self.glyph().is_some_and(|next| {
                        next.is_whitespace() || matches!(next, '}' | ']' | '>' | '»' | ')')
                    })
                {
                    return Err(self.failure(Problem::MissingBody, self.offset));
                }
                return self.headed(Symbol(run), None, start, separator);
            }
            if glyph == '<' {
                if run.is_empty() {
                    return Err(self.failure(Problem::MissingHead, start));
                }
                let angle_start = self.offset;
                let remaining = self.budget.remaining();
                let constraints = self.enclosed(Enclosure::Angled)?;
                let separator = match self.glyph() {
                    Some('.') => Separator::Period,
                    Some('!') => Separator::Exclamation,
                    Some(':') => Separator::Colon,
                    _ => {
                        self.offset = angle_start;
                        self.budget.remaining = remaining;
                        return Ok(Protos::Bare {
                            extent: Extent::from_offsets(start, angle_start),
                            text: run,
                        });
                    }
                };
                self.step();
                if self.glyph().is_none()
                    || self.glyph().is_some_and(|next| {
                        next.is_whitespace() || matches!(next, '}' | ']' | '>' | '»' | ')')
                    })
                {
                    return Err(self.failure(Problem::MissingBody, self.offset));
                }
                return self.headed(Symbol(run), Some(constraints), start, separator);
            }
            run.push(glyph);
            self.step();
        }
        if run.is_empty() {
            Err(self.failure(Problem::MissingBody, start))
        } else {
            Ok(Protos::Bare {
                extent: Extent::from_offsets(start, self.offset),
                text: run,
            })
        }
    }
    fn headed(
        &mut self,
        head: Symbol,
        constraints: Option<Protos>,
        start: usize,
        separator: Separator,
    ) -> Result<Protos, Error> {
        let body = self.node()?;
        Ok(Protos::Headed {
            extent: Extent::from_offsets(start, body.extent().end().get()),
            head,
            constraints: constraints.map(Box::new),
            separator,
            body: Box::new(body),
        })
    }
    fn space(&mut self) {
        loop {
            while self.glyph().is_some_and(char::is_whitespace) {
                self.step();
            }
            if self.glyph() != Some(';') {
                return;
            }
            while self.glyph().is_some_and(|glyph| glyph != '\n') {
                self.step();
            }
        }
    }
    fn glyph(&self) -> Option<char> {
        self.text.get(self.offset..)?.chars().next()
    }
    fn step(&mut self) {
        if let Some(glyph) = self.glyph() {
            self.offset += glyph.len_utf8();
        }
    }
    fn failure(&self, problem: Problem, start: usize) -> Error {
        Error {
            extent: Extent::from_offsets(start, self.offset),
            problem,
        }
    }
}
impl Protosizable for String {
    type Output = Result<Protos, Error>;
    fn protosize(&self) -> Self::Output {
        let mut budget = ReaderBudget::try_from(4_096).expect("default budget is bounded");
        self.protosize_with(&mut budget)
    }
}
impl BoundedProtosizable for String {
    fn protosize_with(&self, budget: &mut ReaderBudget) -> Result<Protos, Error> {
        Reader {
            text: self,
            offset: 0,
            budget,
            depth: 0,
        }
        .whole()
    }
}
impl Protosizable for str {
    type Output = Result<Protos, Error>;
    fn protosize(&self) -> Self::Output {
        let mut budget = ReaderBudget::try_from(4_096).expect("default budget is bounded");
        self.protosize_with(&mut budget)
    }
}
impl BoundedProtosizable for str {
    fn protosize_with(&self, budget: &mut ReaderBudget) -> Result<Protos, Error> {
        Reader {
            text: self,
            offset: 0,
            budget,
            depth: 0,
        }
        .whole()
    }
}
impl fmt::Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "structural error at {}..{}: {:?}",
            self.extent.start().get(),
            self.extent.end().get(),
            self.problem
        )
    }
}
impl std::error::Error for Error {}
