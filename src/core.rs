use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Extent {
    pub start: usize,
    pub end: usize,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
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
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Protos {
    Headed {
        extent: Extent,
        head: Symbol,
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
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Error {
    pub extent: Extent,
    pub problem: Problem,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Problem {
    Empty,
    Multiple,
    Unclosed(char),
    Unexpected(char),
    MissingHead,
    MissingBody,
    Budget,
}
/// The number of structural nodes one reading act may visit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReaderBudget {
    pub remaining: usize,
}
pub trait ReaderBudgeting {
    fn spend(&mut self) -> bool;
}
impl ReaderBudgeting for ReaderBudget {
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
    fn protosize(&self) -> Result<Protos, Error>;
}
pub trait Textualizable {
    fn textualize(&self) -> String;
}

trait Glyphing {
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
trait Separating {
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
}
trait Reading {
    fn whole(&mut self) -> Result<Protos, Error>;
    fn node(&mut self) -> Result<Protos, Error>;
    fn enclosed(&mut self, enclosure: Enclosure) -> Result<Protos, Error>;
    fn opaque(&mut self, boundary: Boundary) -> Result<Protos, Error>;
    fn bare_or_headed(&mut self) -> Result<Protos, Error>;
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
        self.space();
        match self.glyph() {
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
        }
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
            if boundary == Boundary::Guillemets && glyph == '\\' {
                self.step();
                if let Some(escaped) = self.glyph() {
                    if escaped != boundary.closer() {
                        content.push('\\');
                    }
                    content.push(escaped);
                    self.step();
                } else {
                    content.push('\\');
                }
                continue;
            }
            if boundary == Boundary::Parentheses && glyph == '\\' {
                self.step();
                let Some(escaped) = self.glyph() else {
                    return Err(self.failure(Problem::Unclosed(boundary.opener()), start));
                };
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
        let mut run = String::new();
        while let Some(glyph) = self.glyph() {
            if glyph.is_whitespace()
                || matches!(
                    glyph,
                    '{' | '}' | '[' | ']' | '<' | '>' | '«' | '»' | '(' | ')' | ';'
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
                let body = self.node()?;
                return Ok(Protos::Headed {
                    extent: Extent {
                        start,
                        end: body.extent().end,
                    },
                    head: Symbol(run),
                    separator,
                    body: Box::new(body),
                });
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
            extent: Extent {
                start,
                end: self.offset,
            },
            problem,
        }
    }
}
impl Protosizable for String {
    fn protosize(&self) -> Result<Protos, Error> {
        let mut budget = ReaderBudget { remaining: 4_096 };
        self.protosize_with(&mut budget)
    }
}
impl BoundedProtosizable for String {
    fn protosize_with(&self, budget: &mut ReaderBudget) -> Result<Protos, Error> {
        Reader {
            text: self,
            offset: 0,
            budget,
        }
        .whole()
    }
}
impl Protosizable for str {
    fn protosize(&self) -> Result<Protos, Error> {
        let mut budget = ReaderBudget { remaining: 4_096 };
        self.protosize_with(&mut budget)
    }
}
impl BoundedProtosizable for str {
    fn protosize_with(&self, budget: &mut ReaderBudget) -> Result<Protos, Error> {
        Reader {
            text: self,
            offset: 0,
            budget,
        }
        .whole()
    }
}
trait Printing {
    fn print(&self, out: &mut String);
}
impl Printing for Protos {
    fn print(&self, out: &mut String) {
        match self {
            Self::Bare { text, .. } => out.push_str(text),
            Self::Opaque {
                boundary, content, ..
            } => {
                out.push(boundary.opener());
                if *boundary == Boundary::Guillemets {
                    for glyph in content.chars() {
                        if glyph == '»' {
                            out.push('\\');
                        }
                        out.push(glyph);
                    }
                } else {
                    let mut depth = 0usize;
                    for glyph in content.chars() {
                        if glyph == '\\' {
                            out.push('\\');
                        }
                        if glyph == ')' && depth == 0 {
                            out.push('\\');
                        }
                        if glyph == '(' {
                            depth += 1;
                        }
                        if glyph == ')' && depth > 0 {
                            depth -= 1;
                        }
                        out.push(glyph);
                    }
                }
                out.push(boundary.closer());
            }
            Self::Enclosed {
                enclosure,
                children,
                ..
            } => {
                out.push(enclosure.opener());
                if !children.is_empty() {
                    if *enclosure != Enclosure::Angled {
                        out.push(' ');
                    }
                    for (index, child) in children.iter().enumerate() {
                        if index > 0 {
                            out.push(' ');
                        }
                        child.print(out);
                    }
                    if *enclosure != Enclosure::Angled {
                        out.push(' ');
                    }
                }
                out.push(enclosure.closer());
            }
            Self::Headed {
                head,
                separator,
                body,
                ..
            } => {
                out.push_str(&head.0);
                out.push(separator.glyph());
                body.print(out);
            }
        }
    }
}
impl Textualizable for Protos {
    fn textualize(&self) -> String {
        let mut out = String::new();
        self.print(&mut out);
        out
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
