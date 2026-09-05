//! The reader: one pass over the characters with an explicit stack of frames,
//! each shape announcing its type and its context reading to its completion.

use crate::anatomy::{
    Bare, Boundary, Delineation, Enclosure, Extent, Fault, Head, Integer, Opaque, Problem,
    Protoform, Separator, Situated, Situation, Symbol,
};
use crate::glyph::{Classifying, Glyph};
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

/// An enclosure just removed from the reader stack, named while it becomes one
/// completed structure.
struct Closed {
    opened: usize,
    children: Vec<Situated<Protoform>>,
    qualifying: Option<Qualifying>,
}

/// The reader's state: the text, the offset, the open frames, the finished top-level structures.
pub(crate) struct Reader<'a> {
    pub(crate) text: &'a str,
    pub(crate) offset: usize,
    frames: Vec<Frame>,
    qualifying: Option<Qualifying>,
    done: Vec<Situated<Protoform>>,
}

/// The kind whose capability builds a fault at a byte span.
pub(crate) trait Faulting {
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
pub(crate) trait Peeking {
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
                head: Head::Symbol(
                    Symbol::try_from(piece.text).expect("a nonempty run piece is a symbol"),
                ),
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
                symbol: Symbol::try_from(pieces[last].text)
                    .expect("a nonempty run piece is a symbol"),
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
                Protoform::Bare(Bare::try_from(piece.text).expect("a nonempty run piece is bare")),
            ));
        } else {
            self.deliver(Situated(
                Situation {
                    extent: Extent(start as Integer, end as Integer),
                    children: vec![],
                },
                Protoform::Bare(Bare::try_from(run.text).expect("a run is bare")),
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
        let closed = match self.frames.pop() {
            Some(Frame::Enclosing {
                enclosure: open,
                opened,
                children,
                qualifying,
            }) if open == enclosure => Closed {
                opened,
                children,
                qualifying,
            },
            other => {
                if let Some(frame) = other {
                    self.frames.push(frame);
                }
                return Err(self.fault(self.offset, glyph_end, Problem::Unopened(enclosure)));
            }
        };
        self.offset = glyph_end;
        let mut situations = Vec::with_capacity(closed.children.len());
        let mut forms = Vec::with_capacity(closed.children.len());
        for Situated(at, form) in closed.children {
            situations.push(at);
            forms.push(form);
        }
        let structure = Situated(
            Situation {
                extent: Extent(closed.opened as Integer, glyph_end as Integer),
                children: situations,
            },
            Protoform::Enclosed(enclosure, forms),
        );
        match closed.qualifying {
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
        let separator = match self.glyph_at(end).map(Classifying::classify) {
            Some(Glyph::Separate(separator)) => separator,
            _ => return self.deliver(Situated(at, Protoform::Qualified(symbol, forms))),
        };
        let after = self
            .glyph_at(end + separator.glyph().len_utf8())
            .map(Classifying::classify);
        if matches!(after, Some(Glyph::Open(_) | Glyph::Bound(_) | Glyph::Plain)) {
            self.offset = end + separator.glyph().len_utf8();
            self.frames.push(Frame::Heading {
                head: Head::Qualified(symbol, forms),
                at,
                separator,
                start,
            });
        } else {
            self.deliver(Situated(at, Protoform::Qualified(symbol, forms)));
        }
    }

    fn read_bounded(&mut self, boundary: Boundary) -> Result<(), Fault> {
        use crate::opaque::Bounding;
        let opened = self.offset;
        let structure = match boundary {
            Boundary::CurlyQuotes => Protoform::Quoted(
                crate::Text::try_from(self.read_quoted()?)
                    .expect("the quoted reader stops at the closing quote"),
            ),
            Boundary::Parentheses => {
                Protoform::Parenthesized(Opaque::from(self.read_parenthesized()?))
            }
        };
        self.deliver(Situated(
            Situation {
                extent: Extent(opened as Integer, self.offset as Integer),
                children: vec![],
            },
            structure,
        ));
        Ok(())
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
