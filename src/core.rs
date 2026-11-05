use std::collections::BTreeSet;
use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(
    feature = "rkyv",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)
)]
pub struct Extent {
    pub start: usize,
    pub end: usize,
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
/// The structural work one reading act may still perform and its deepest
/// permitted node. Both values are checked at construction and remain exactly
/// representable as Protos integers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ReaderBudget {
    remaining: i64,
    maximum_depth: i64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ReaderBudgetField {
    Remaining,
    MaximumDepth,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ReaderBudgetConstructionReason {
    Negative,
    OutsideNativeWidth,
    OutsideIntegerWidth,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ReaderBudgetConstructionError {
    pub field: ReaderBudgetField,
    pub reason: ReaderBudgetConstructionReason,
}
pub trait BuildsReaderBudget: Sized {
    fn from_integer_limits(
        remaining: i64,
        maximum_depth: i64,
    ) -> Result<Self, ReaderBudgetConstructionError>;
    fn from_native_limits(
        remaining: usize,
        maximum_depth: usize,
    ) -> Result<Self, ReaderBudgetConstructionError>;
}
pub trait ReadsReaderBudget {
    fn remaining(&self) -> i64;
    fn maximum_depth(&self) -> i64;
    fn permits_depth(&self, depth: usize) -> bool;
}

/// A nonrecursive borrowed view of the lexical layer shared by structural
/// reading and fixed-prefix readers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LexicalCursor<'text> {
    text: &'text str,
    offset: usize,
}
pub trait LexicallyReadable {
    fn lexical_cursor(&self) -> LexicalCursor<'_>;
}
pub trait LexicallyTraversable<'text> {
    fn skip_space_and_comments(&mut self);
    fn expect_delimiter(&mut self, expected: char) -> Result<(), Error>;
    fn token(&mut self) -> Result<&'text str, Error>;
    fn offset(&self) -> usize;
}

impl LexicallyReadable for str {
    fn lexical_cursor(&self) -> LexicalCursor<'_> {
        LexicalCursor {
            text: self,
            offset: 0,
        }
    }
}

impl<'text> LexicallyTraversable<'text> for LexicalCursor<'text> {
    fn skip_space_and_comments(&mut self) {
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

    fn expect_delimiter(&mut self, expected: char) -> Result<(), Error> {
        if !matches!(expected, '{' | '}' | '[' | ']' | '<' | '>') {
            return Err(self.failure(Problem::Unexpected(expected), self.offset));
        }
        self.skip_space_and_comments();
        if self.glyph() == Some(expected) {
            self.step();
            Ok(())
        } else {
            Err(self.failure(
                self.glyph()
                    .map(Problem::Unexpected)
                    .unwrap_or(Problem::MissingBody),
                self.offset,
            ))
        }
    }

    fn token(&mut self) -> Result<&'text str, Error> {
        self.skip_space_and_comments();
        let start = self.offset;
        while let Some(glyph) = self.glyph() {
            if glyph.is_whitespace()
                || matches!(
                    glyph,
                    '{' | '}' | '[' | ']' | '<' | '>' | '«' | '»' | '(' | ')' | ';'
                )
            {
                break;
            }
            self.step();
        }
        if self.offset == start {
            Err(self.failure(
                self.glyph()
                    .map(Problem::Unexpected)
                    .unwrap_or(Problem::MissingBody),
                start,
            ))
        } else {
            Ok(&self.text[start..self.offset])
        }
    }

    fn offset(&self) -> usize {
        self.offset
    }
}

impl LexicalCursor<'_> {
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
            extent: Extent {
                start,
                end: self.offset,
            },
            problem,
        }
    }
}
impl BuildsReaderBudget for ReaderBudget {
    fn from_integer_limits(
        remaining: i64,
        maximum_depth: i64,
    ) -> Result<Self, ReaderBudgetConstructionError> {
        for (field, value) in [
            (ReaderBudgetField::Remaining, remaining),
            (ReaderBudgetField::MaximumDepth, maximum_depth),
        ] {
            if value < 0 {
                return Err(ReaderBudgetConstructionError {
                    field,
                    reason: ReaderBudgetConstructionReason::Negative,
                });
            }
            usize::try_from(value).map_err(|_| ReaderBudgetConstructionError {
                field,
                reason: ReaderBudgetConstructionReason::OutsideNativeWidth,
            })?;
        }
        Ok(Self {
            remaining,
            maximum_depth,
        })
    }

    fn from_native_limits(
        remaining: usize,
        maximum_depth: usize,
    ) -> Result<Self, ReaderBudgetConstructionError> {
        let remaining = i64::try_from(remaining).map_err(|_| ReaderBudgetConstructionError {
            field: ReaderBudgetField::Remaining,
            reason: ReaderBudgetConstructionReason::OutsideIntegerWidth,
        })?;
        let maximum_depth =
            i64::try_from(maximum_depth).map_err(|_| ReaderBudgetConstructionError {
                field: ReaderBudgetField::MaximumDepth,
                reason: ReaderBudgetConstructionReason::OutsideIntegerWidth,
            })?;
        Self::from_integer_limits(remaining, maximum_depth)
    }
}
impl ReadsReaderBudget for ReaderBudget {
    fn remaining(&self) -> i64 {
        self.remaining
    }

    fn maximum_depth(&self) -> i64 {
        self.maximum_depth
    }

    fn permits_depth(&self, depth: usize) -> bool {
        i64::try_from(depth).is_ok_and(|depth| depth < self.maximum_depth)
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
        if !self.budget.permits_depth(self.depth) {
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
                        extent: Extent {
                            start,
                            end: self.offset,
                        },
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
                        extent: Extent {
                            start,
                            end: self.offset,
                        },
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
                extent: Extent {
                    start,
                    end: self.offset,
                },
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
                let remaining = self.budget.remaining;
                let constraints = self.enclosed(Enclosure::Angled)?;
                let separator = match self.glyph() {
                    Some('.') => Separator::Period,
                    Some('!') => Separator::Exclamation,
                    Some(':') => Separator::Colon,
                    _ => {
                        self.offset = angle_start;
                        self.budget.remaining = remaining;
                        return Ok(Protos::Bare {
                            extent: Extent {
                                start,
                                end: angle_start,
                            },
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
                extent: Extent {
                    start,
                    end: self.offset,
                },
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
            extent: Extent {
                start,
                end: body.extent().end,
            },
            head,
            constraints: constraints.map(Box::new),
            separator,
            body: Box::new(body),
        })
    }
    fn space(&mut self) {
        let mut cursor = LexicalCursor {
            text: self.text,
            offset: self.offset,
        };
        cursor.skip_space_and_comments();
        self.offset = cursor.offset;
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
            extent: Extent {
                start,
                end: self.offset,
            },
            problem,
        }
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
            self.extent.start, self.extent.end, self.problem
        )
    }
}
impl std::error::Error for Error {}
