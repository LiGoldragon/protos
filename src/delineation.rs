//! The reader: one pass over the characters with an explicit stack of frames,
//! each shape announcing its type and its context reading to its completion.

use crate::anatomy::{
    Boundary, Delineation, Enclosure, Extent, Fault, Head, Integer, Problem, Protoform, Separator,
    Situated, Situation, Symbol, Text,
};
use crate::glyph::{Classifying, Glyph, Mark};
use crate::kinds::Protosizable;
use crate::kinds::{Delimiting, Glyphing};
use crate::run::{Piece, Run, Splitting, Symbolic};

/// A symbol immediately followed by `<`: the angled enclosure that follows qualifies it.
struct Qualifying {
    symbol: Symbol,
    start: usize,
}

/// An open context on the reader's stack.
enum Frame {
    /// An enclosure read up to here: its children so far, and the symbol it qualifies if angled after one.
    Enclosing {
        enclosure: Enclosure,
        opened: usize,
        children: Vec<Situated<Protoform>>,
        qualifying: Option<Qualifying>,
    },
    /// A head and its separator, awaiting the one structure that is its body.
    Heading {
        head: Head,
        at: Situation,
        separator: Separator,
        start: usize,
    },
}

/// The reader's state: the text, the offset, the open frames, the finished top-level structures.
struct Reader<'a> {
    text: &'a str,
    offset: usize,
    frames: Vec<Frame>,
    qualifying: Option<Qualifying>,
    done: Vec<Situated<Protoform>>,
}

/// The kind whose capability builds a fault at a byte span.
trait Faulting {
    fn fault(&self, start: usize, end: usize, problem: Problem) -> Fault;
}

impl Faulting for Reader<'_> {
    fn fault(&self, start: usize, end: usize, problem: Problem) -> Fault {
        Fault {
            extent: Extent(start as Integer, end as Integer),
            problem,
        }
    }
}

/// The kind whose capability yields the glyph at an offset.
trait Peeking {
    fn glyph_at(&self, offset: usize) -> Option<char>;
}

impl Peeking for Reader<'_> {
    fn glyph_at(&self, offset: usize) -> Option<char> {
        self.text.get(offset..)?.chars().next()
    }
}

/// The kind whose capability hands a finished structure to the frame that awaits it.
trait Delivering {
    fn deliver(&mut self, structure: Situated<Protoform>);
}

impl Delivering for Reader<'_> {
    fn deliver(&mut self, mut structure: Situated<Protoform>) {
        loop {
            match self.frames.pop() {
                Some(Frame::Heading {
                    head,
                    at,
                    separator,
                    start,
                }) => {
                    let Situated(body_at, body) = structure;
                    let extent = Extent(start as Integer, body_at.extent.1);
                    structure = Situated(
                        Situation {
                            extent,
                            children: vec![at, body_at],
                        },
                        Protoform::Headed(head, separator, Box::new(body)),
                    );
                }
                Some(Frame::Enclosing {
                    enclosure,
                    opened,
                    mut children,
                    qualifying,
                }) => {
                    children.push(structure);
                    self.frames.push(Frame::Enclosing {
                        enclosure,
                        opened,
                        children,
                        qualifying,
                    });
                    return;
                }
                None => {
                    self.done.push(structure);
                    return;
                }
            }
        }
    }
}

/// The kind whose capabilities read one shape each, from the offset to its completion.
trait Reading {
    fn read_comment(&mut self);
    fn read_run(&mut self);
    fn read_open(&mut self, enclosure: Enclosure);
    fn read_close(&mut self, enclosure: Enclosure) -> Result<(), Fault>;
    fn read_bounded(&mut self, boundary: Boundary) -> Result<(), Fault>;
    fn read_quoted(&mut self) -> Result<String, Fault>;
    fn read_parenthesized(&mut self) -> Result<String, Fault>;
    fn qualify(&mut self, symbol: Symbol, start: usize, constraints: Situated<Protoform>);
}

/// The kind whose capability pushes the heads of a run's leading pieces as frames.
trait HeadPushing {
    fn push_heads(&mut self, pieces: &[Piece<'_>]);
}

impl HeadPushing for Reader<'_> {
    fn push_heads(&mut self, pieces: &[Piece<'_>]) {
        for piece in pieces {
            let end = piece.start as usize + piece.text.len();
            self.frames.push(Frame::Heading {
                head: Head::Symbol(piece.text.to_owned()),
                at: Situation {
                    extent: Extent(piece.start, end as Integer),
                    children: vec![],
                },
                separator: piece.separator.unwrap_or(Separator::Period),
                start: piece.start as usize,
            });
        }
    }
}

impl Reading for Reader<'_> {
    fn read_comment(&mut self) {
        while let Some(glyph) = self.glyph_at(self.offset) {
            if glyph == '\n' {
                return;
            }
            self.offset += glyph.len_utf8();
        }
    }

    fn read_run(&mut self) {
        let start = self.offset;
        let mut end = start;
        let mut following = None;
        while let Some(glyph) = self.glyph_at(end) {
            match glyph.classify() {
                Glyph::Plain | Glyph::Separate(_) => end += glyph.len_utf8(),
                other => {
                    following = Some(other);
                    break;
                }
            }
        }
        self.offset = end;
        let run = Run {
            text: &self.text[start..end],
            start: start as Integer,
        };
        let pieces = run.pieces();
        let last = pieces.len() - 1;
        let heads_are_symbols = pieces[..last].iter().all(Symbolic::is_symbol);
        let opens_body = matches!(following, Some(Glyph::Open(_) | Glyph::Bound(_)));
        let qualifies = following == Some(Glyph::Open(Enclosure::Angled));
        if last > 0 && heads_are_symbols && !pieces[last].is_symbol() && opens_body {
            self.push_heads(&pieces[..last]);
        } else if heads_are_symbols && pieces[last].is_symbol() && qualifies {
            self.push_heads(&pieces[..last]);
            self.qualifying = Some(Qualifying {
                symbol: pieces[last].text.to_owned(),
                start: pieces[last].start as usize,
            });
        } else if heads_are_symbols && pieces[last].is_symbol() {
            self.push_heads(&pieces[..last]);
            let piece = pieces[last];
            self.deliver(Situated(
                Situation {
                    extent: Extent(piece.start, end as Integer),
                    children: vec![],
                },
                Protoform::Bare(Head::Symbol(piece.text.to_owned())),
            ));
        } else {
            self.deliver(Situated(
                Situation {
                    extent: Extent(start as Integer, end as Integer),
                    children: vec![],
                },
                Protoform::Bare(Head::Symbol(run.text.to_owned())),
            ));
        }
    }

    fn read_open(&mut self, enclosure: Enclosure) {
        let qualifying = self.qualifying.take();
        self.frames.push(Frame::Enclosing {
            enclosure,
            opened: self.offset,
            children: vec![],
            qualifying,
        });
        self.offset += enclosure.opener().len_utf8();
    }

    fn read_close(&mut self, enclosure: Enclosure) -> Result<(), Fault> {
        let glyph_end = self.offset + enclosure.closer().len_utf8();
        let (opened, children, qualifying) = match self.frames.pop() {
            Some(Frame::Enclosing {
                enclosure: open,
                opened,
                children,
                qualifying,
            }) if open == enclosure => (opened, children, qualifying),
            other => {
                if let Some(frame) = other {
                    self.frames.push(frame);
                }
                return Err(self.fault(self.offset, glyph_end, Problem::Unopened(enclosure)));
            }
        };
        self.offset = glyph_end;
        let mut situations = Vec::with_capacity(children.len());
        let mut forms = Vec::with_capacity(children.len());
        for Situated(at, form) in children {
            situations.push(at);
            forms.push(form);
        }
        let structure = Situated(
            Situation {
                extent: Extent(opened as Integer, glyph_end as Integer),
                children: situations,
            },
            Protoform::Enclosed(enclosure, forms),
        );
        match qualifying {
            Some(Qualifying { symbol, start }) => self.qualify(symbol, start, structure),
            None => self.deliver(structure),
        }
        Ok(())
    }

    fn qualify(&mut self, symbol: Symbol, start: usize, constraints: Situated<Protoform>) {
        let Situated(constraints_at, mut constraints_form) = constraints;
        let end = constraints_at.extent.1 as usize;
        let forms = match &mut constraints_form {
            Protoform::Enclosed(_, forms) => std::mem::take(forms),
            _ => vec![],
        };
        let mut at = constraints_at;
        at.extent = Extent(start as Integer, end as Integer);
        let head = Head::Qualified(symbol, forms);
        let separator = match self.glyph_at(end).map(Classifying::classify) {
            Some(Glyph::Separate(separator)) => separator,
            _ => return self.deliver(Situated(at, Protoform::Bare(head))),
        };
        let after = self
            .glyph_at(end + separator.glyph().len_utf8())
            .map(Classifying::classify);
        if matches!(after, Some(Glyph::Open(_) | Glyph::Bound(_) | Glyph::Plain)) {
            self.offset = end + separator.glyph().len_utf8();
            self.frames.push(Frame::Heading {
                head,
                at,
                separator,
                start,
            });
        } else {
            self.deliver(Situated(at, Protoform::Bare(head)));
        }
    }

    fn read_bounded(&mut self, boundary: Boundary) -> Result<(), Fault> {
        let opened = self.offset;
        let content = match boundary {
            Boundary::CurlyQuotes => self.read_quoted()?,
            Boundary::Parentheses => self.read_parenthesized()?,
        };
        self.deliver(Situated(
            Situation {
                extent: Extent(opened as Integer, self.offset as Integer),
                children: vec![],
            },
            Protoform::Opaque(boundary, Text(content)),
        ));
        Ok(())
    }

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
        let stray = Boundary::CurlyQuotes.closer();
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
            } else if glyph == stray {
                return Err(self.fault(
                    here - glyph.len_utf8(),
                    here,
                    Problem::Stray(Boundary::CurlyQuotes),
                ));
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

/// The kind whose capability reads the whole text into its delineation.
trait Delineating {
    fn delineate(self) -> Result<Delineation, Fault>;
}

impl Delineating for Reader<'_> {
    fn delineate(mut self) -> Result<Delineation, Fault> {
        while let Some(glyph) = self.glyph_at(self.offset) {
            match glyph.classify() {
                Glyph::Space => self.offset += glyph.len_utf8(),
                Glyph::Comment => self.read_comment(),
                Glyph::Open(enclosure) => self.read_open(enclosure),
                Glyph::Close(enclosure) => self.read_close(enclosure)?,
                Glyph::Bound(boundary) => self.read_bounded(boundary)?,
                Glyph::Unbound(boundary) => {
                    return Err(self.fault(
                        self.offset,
                        self.offset + glyph.len_utf8(),
                        Problem::Stray(boundary),
                    ));
                }
                Glyph::Separate(_) | Glyph::Plain => self.read_run(),
            }
        }
        while let Some(frame) = self.frames.pop() {
            if let Frame::Enclosing {
                enclosure, opened, ..
            } = frame
            {
                return Err(self.fault(opened, self.text.len(), Problem::Unclosed(enclosure)));
            }
        }
        Ok(Delineation(self.done))
    }
}

impl Protosizable for str {
    type Fault = Fault;

    fn protosize(&self) -> Result<Delineation, Fault> {
        Reader {
            text: self,
            offset: 0,
            frames: Vec::new(),
            qualifying: None,
            done: Vec::new(),
        }
        .delineate()
    }
}

impl Protosizable for String {
    type Fault = Fault;

    fn protosize(&self) -> Result<Delineation, Fault> {
        self.as_str().protosize()
    }
}
