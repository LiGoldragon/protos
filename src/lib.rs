use std::collections::BTreeMap;
use std::fmt;
use std::marker::PhantomData;

pub type Text = String;
pub type Integer = i64;
pub type Decimal = f64;
pub type Boolean = bool;
pub type Symbol = Text;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Extent(pub Integer, pub Integer);

impl fmt::Debug for Extent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Extent({}, {})", self.0, self.1)
    }
}

pub type Path = Vec<Integer>;

pub type Situation = BTreeMap<Path, Extent>;

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

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum Enclosure {
    Braced,
    Bracketed,
    Angled,
}

impl fmt::Debug for Enclosure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Braced => write!(f, "Braced"),
            Self::Bracketed => write!(f, "Bracketed"),
            Self::Angled => write!(f, "Angled"),
        }
    }
}

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

#[derive(Clone)]
pub enum Head {
    Bare(Symbol),
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

#[derive(Clone)]
pub enum Protoform {
    Headed(Head, Separator, Box<Protoform>),
    Enclosed(Enclosure, Vec<Protoform>),
    Opaque(Boundary, Text),
    Bare(Head),
}

impl fmt::Debug for Protoform {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Headed(h, s, b) => f.debug_tuple("Headed").field(h).field(s).field(b).finish(),
            Self::Enclosed(e, c) => f.debug_tuple("Enclosed").field(e).field(c).finish(),
            Self::Opaque(b, c) => f.debug_tuple("Opaque").field(b).field(c).finish(),
            Self::Bare(h) => f.debug_tuple("Bare").field(h).finish(),
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
            (Self::Bare(h1), Self::Bare(h2)) => h1 == h2,
            _ => false,
        }
    }
}

impl Eq for Protoform {}

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

#[derive(Clone, PartialEq, Eq)]
pub enum Problem {
    Unclosed(Enclosure),
    UnclosedBoundary(Boundary),
    Unopened,
    MissingBody,
    MissingHead,
}

impl fmt::Debug for Problem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unclosed(e) => write!(f, "Unclosed({e:?})"),
            Self::UnclosedBoundary(b) => write!(f, "UnclosedBoundary({b:?})"),
            Self::Unopened => write!(f, "Unopened"),
            Self::MissingBody => write!(f, "MissingBody"),
            Self::MissingHead => write!(f, "MissingHead"),
        }
    }
}

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

pub struct Potential<T, C = ()>(Text, PhantomData<fn() -> (T, C)>);

impl<T, C> Potential<T, C> {
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Situated<F>(pub Option<Extent>, pub F);

pub trait Textualizable {
    fn textualize(&self) -> Text;
}

pub trait Protosizable {
    type Fault;
    fn protosize(&self) -> Result<Delineation, Self::Fault>;
}

pub trait Conceivable<C> {
    type Fault;
    fn conceive(&self) -> Result<C, Self::Fault>;
}

pub trait Incorporable<C>: Sized {
    type Fault;
    fn incorporate(concept: C) -> Result<Self, Self::Fault>;
}

pub trait Actualizable<T: Sized> {
    type Fault;
    fn actualize(&self) -> Result<T, Self::Fault>;
}

pub trait Pathed {
    fn path(&self) -> &[Integer];
}

pub trait Situating {
    fn situate(&self, path: &[Integer]) -> Option<Extent>;
}

trait Glyphing {
    fn glyph(&self) -> char;
}

impl Glyphing for Separator {
    fn glyph(&self) -> char {
        match self {
            Self::Period => '.',
            Self::Exclamation => '!',
            Self::Colon => ':',
        }
    }
}

impl Textualizable for Head {
    fn textualize(&self) -> Text {
        match self {
            Head::Bare(symbol) => symbol.clone(),
            Head::Qualified(symbol, children) => {
                let inner: Vec<String> = children.iter().map(|c| c.textualize()).collect();
                let joined = inner.join(" ");
                format!("{symbol}<{joined}>")
            }
        }
    }
}

impl Textualizable for Protoform {
    fn textualize(&self) -> Text {
        match self {
            Protoform::Headed(head, sep, body) => {
                let mut result = head.textualize();
                result.push(sep.glyph());
                result.push_str(&body.textualize());
                result
            }
            Protoform::Enclosed(enclosure, children) => {
                let (open, close) = match enclosure {
                    Enclosure::Braced => ("{", "}"),
                    Enclosure::Bracketed => ("[", "]"),
                    Enclosure::Angled => ("<", ">"),
                };
                if children.is_empty() {
                    format!("{open}{close}")
                } else {
                    let inner: Vec<String> =
                        children.iter().map(|c| c.textualize()).collect();
                    let joined = inner.join(" ");
                    match enclosure {
                        Enclosure::Angled => format!("{open}{joined}{close}"),
                        _ => format!("{open} {joined} {close}"),
                    }
                }
            }
            Protoform::Opaque(boundary, content) => match boundary {
                Boundary::CurlyQuotes => {
                    format!("\u{201C}{content}\u{201D}")
                }
                Boundary::Parentheses => {
                    let escaped = escape_parens_for_print(content);
                    format!("({escaped})")
                }
            },
            Protoform::Bare(head) => head.textualize(),
        }
    }
}

fn escape_parens_for_print(content: &str) -> String {
    let mut result = String::with_capacity(content.len());
    let mut depth: i32 = 0;
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

impl Textualizable for Delineation {
    fn textualize(&self) -> Text {
        let parts: Vec<String> = self.protoforms.iter().map(|p| p.textualize()).collect();
        parts.join(" ")
    }
}

const OPEN_BRACE: char = '{';
const CLOSE_BRACE: char = '}';
const OPEN_BRACKET: char = '[';
const CLOSE_BRACKET: char = ']';
const OPEN_ANGLE: char = '<';
const CLOSE_ANGLE: char = '>';
const OPEN_CURLY_QUOTE: char = '\u{201C}';
const CLOSE_CURLY_QUOTE: char = '\u{201D}';
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
        OPEN_ANGLE => Some(Enclosure::Angled),
        _ => None,
    }
}

fn closer_for_enclosure(e: Enclosure) -> char {
    match e {
        Enclosure::Braced => CLOSE_BRACE,
        Enclosure::Bracketed => CLOSE_BRACKET,
        Enclosure::Angled => CLOSE_ANGLE,
    }
}

fn is_closer(c: char) -> bool {
    matches!(
        c,
        CLOSE_BRACE
            | CLOSE_BRACKET
            | CLOSE_ANGLE
            | CLOSE_CURLY_QUOTE
            | CLOSE_PAREN
    )
}

fn parse_bare_run(
    run: &str,
    run_start: Integer,
    base_path: &[Integer],
) -> Result<(Protoform, Vec<(Path, Extent)>), Fault> {
    if let Some(first_char) = run.chars().next() {
        if is_separator(first_char) {
            return Err(Fault {
                extent: Extent(run_start, run_start + first_char.len_utf8() as Integer),
                problem: Problem::MissingHead,
            });
        }
    }

    let mut char_iter = run.char_indices().peekable();
    while let Some((byte_offset, ch)) = char_iter.next() {
        if is_separator(ch) {
            if let Some(&(next_offset, next_ch)) = char_iter.peek() {
                if !next_ch.is_whitespace() && !is_closer(next_ch) && !is_delimiter(next_ch) {
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
                    return Err(Fault {
                        extent: Extent(
                            run_start + byte_offset as Integer,
                            run_start + byte_offset as Integer + ch.len_utf8() as Integer,
                        ),
                        problem: Problem::MissingBody,
                    });
                }
            } else {
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

    let extent = Extent(run_start, run_start + run.len() as Integer);
    let situations = vec![(base_path.to_vec(), extent)];
    Ok((Protoform::Bare(Head::Bare(run.to_owned())), situations))
}

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
            let before = self.pos;
            while let Some(c) = self.peek_char() {
                if c.is_whitespace() {
                    self.advance_char();
                } else {
                    break;
                }
            }

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
                    return Err(Fault {
                        extent: Extent(0, 0),
                        problem: Problem::Unclosed(Enclosure::Braced),
                    });
                }
                break;
            }

            let c = self.peek_char().unwrap();

            if let Some(expected_closer) = closer {
                if c == expected_closer {
                    self.advance_char();
                    break;
                }
            }

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

    fn parse_one(&mut self, path: &[Integer]) -> Result<(Protoform, Vec<(Path, Extent)>), Fault> {
        let c = self.peek_char().unwrap();

        if let Some(enclosure) = enclosure_for_opener(c) {
            let start = self.pos as Integer;
            self.advance_char();
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
        } else if c == OPEN_CURLY_QUOTE {
            let start = self.pos as Integer;
            self.advance_char();
            let content_start = self.pos;
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
        } else if c == OPEN_PAREN {
            let start = self.pos as Integer;
            self.advance_char();
            let mut content = String::new();
            let mut depth = 1u32;
            loop {
                match self.advance_char() {
                    Some(ESCAPE_CHAR) => {
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
        } else {
            let start = self.pos;
            while let Some(c) = self.peek_char() {
                if c.is_whitespace() || is_delimiter(c) || c == COMMENT_CHAR {
                    break;
                }
                self.advance_char();
            }
            let run = &self.source[start..self.pos];

            let run_ends_with_sep =
                !run.is_empty() && run.chars().next_back().is_some_and(is_separator);
            let run_has_internal_sep = run.chars().any(is_separator);
            if !run.is_empty()
                && !run_ends_with_sep
                && !run_has_internal_sep
                && self.peek_char() == Some(OPEN_ANGLE)
            {
                let angle_start = self.pos as Integer;
                self.advance_char();
                match self.parse_contents(Some(CLOSE_ANGLE), path) {
                    Ok((children, mut child_situations)) => {
                        let end = self.pos as Integer;
                        let extent = Extent(start as Integer, end);
                        child_situations.push((path.to_vec(), extent));

                        let symbol = run.to_owned();

                        if let Some(next_c) = self.peek_char() {
                            if is_separator(next_c) {
                                let sep = separator_from_char(next_c);
                                self.advance_char();
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
                                        if let Some(entry) =
                                            child_situations.iter_mut().find(|(p, _)| p == path)
                                        {
                                            entry.1 =
                                                Extent(start as Integer, self.pos as Integer);
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
                                return Err(Fault {
                                    extent: Extent(self.pos as Integer - 1, self.pos as Integer),
                                    problem: Problem::MissingBody,
                                });
                            }
                        }

                        return Ok((
                            Protoform::Bare(Head::Qualified(symbol, children)),
                            child_situations,
                        ));
                    }
                    Err(_) => {
                        return Err(Fault {
                            extent: Extent(angle_start, self.source.len() as Integer),
                            problem: Problem::Unclosed(Enclosure::Angled),
                        });
                    }
                }
            }

            if run_has_internal_sep && self.peek_char() == Some(OPEN_ANGLE) {
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

            if !run.is_empty() {
                let last_char = run.chars().next_back().unwrap();
                if is_separator(last_char) {
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

            parse_bare_run(run, start as Integer, path)
        }
    }
}

fn attach_body_to_deepest(head: Protoform, sep: Separator, body: Protoform) -> Protoform {
    match head {
        Protoform::Bare(Head::Bare(symbol)) => {
            Protoform::Headed(Head::Bare(symbol), sep, Box::new(body))
        }
        Protoform::Bare(Head::Qualified(symbol, quals)) => {
            Protoform::Headed(Head::Qualified(symbol, quals), sep, Box::new(body))
        }
        Protoform::Headed(h, s, inner) => {
            Protoform::Headed(h, s, Box::new(attach_body_to_deepest(*inner, sep, body)))
        }
        other => Protoform::Headed(Head::Bare(String::new()), sep, Box::new(other)),
    }
}

impl Protosizable for Text {
    type Fault = Fault;

    fn protosize(&self) -> Result<Delineation, Fault> {
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

impl<C> Protosizable for Potential<(), C> {
    type Fault = Fault;

    fn protosize(&self) -> Result<Delineation, Fault> {
        self.0.protosize()
    }
}

impl<C, T> Actualizable<T> for Potential<T, C>
where
    C: Sized,
    T: Incorporable<C>,
    Delineation: Conceivable<C>,
    T::Fault: From<Fault> + From<<Delineation as Conceivable<C>>::Fault> + Pathed,
{
    type Fault = Situated<T::Fault>;

    fn actualize(&self) -> Result<T, Situated<T::Fault>> {
        let delineation = self.text().to_owned().protosize().map_err(|f| {
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

impl Situating for Delineation {
    fn situate(&self, path: &[Integer]) -> Option<Extent> {
        self.situation.get(path).copied()
    }
}
