//! The writer: one pass that emits canonical text and records the extent of
//! every structure as it begins and ends it.

use crate::anatomy::{
    Boundary, Delineation, Enclosure, Extent, Head, Integer, Protoform, Situated, Situation,
};
use crate::glyph::Mark;
use crate::kinds::{Conceivable, Protosizable, Situating, Textualizable};
use crate::kinds::{Delimiting, Glyphing};
use std::convert::Infallible;

/// The kind whose capability says whether an enclosure's content is spaced from its delimiters.
trait Spacing {
    fn spaced(&self) -> bool;
}

impl Spacing for Enclosure {
    fn spaced(&self) -> bool {
        match self {
            Self::Braced | Self::Bracketed => true,
            Self::Angled => false,
        }
    }
}

/// The kind whose capability writes opaque content between its boundary, escaping what must be.
trait Escaping {
    fn write_within(&self, boundary: Boundary, out: &mut String);
}

impl Escaping for str {
    fn write_within(&self, boundary: Boundary, out: &mut String) {
        out.push(boundary.opener());
        match boundary {
            Boundary::CurlyQuotes => out.push_str(self),
            Boundary::Parentheses => {
                let opener = boundary.opener();
                let closer = boundary.closer();
                let escape = Mark::Escape.glyph();
                let mut open = Vec::new();
                let mut unbalanced = Vec::new();
                for (offset, glyph) in self.char_indices() {
                    if glyph == opener {
                        open.push(offset);
                    } else if glyph == closer && open.pop().is_none() {
                        unbalanced.push(offset);
                    }
                }
                unbalanced.append(&mut open);
                unbalanced.sort_unstable();
                let mut next = unbalanced.into_iter().peekable();
                for (offset, glyph) in self.char_indices() {
                    if next.peek() == Some(&offset) {
                        next.next();
                        out.push(escape);
                    } else if glyph == escape {
                        out.push(escape);
                    }
                    out.push(glyph);
                }
            }
        }
        out.push(boundary.closer());
    }
}

/// One step of the writer's walk.
enum Step<'a> {
    /// Begin a structure: emit its opening, schedule its parts and its finish.
    Begin(&'a Protoform),
    /// Write a head, situating it.
    Head(&'a Head),
    /// Finish a structure begun at `start` whose last `arity` situations are its children.
    Finish { start: usize, arity: usize },
    /// Emit one glyph.
    Glyph(char),
    /// Emit the one space between siblings.
    Space,
}

/// The writer's state: the text so far, the steps to take, the situations of finished parts.
struct Writer<'a> {
    out: String,
    steps: Vec<Step<'a>>,
    situations: Vec<Situation>,
}

/// The kind whose capabilities take the writer's steps.
trait Stepping<'a> {
    fn begin(&mut self, form: &'a Protoform);
    fn head(&mut self, head: &'a Head);
    fn qualified(&mut self, symbol: &'a crate::Symbol, constraints: &'a [Protoform]);
    fn finish(&mut self, start: usize, arity: usize);
    fn schedule(&mut self, forms: &'a [Protoform]);
    fn write(self) -> Situated<String>;
}

impl<'a> Stepping<'a> for Writer<'a> {
    fn begin(&mut self, form: &'a Protoform) {
        let start = self.out.len();
        match form {
            Protoform::Headed(head, separator, body) => {
                self.steps.push(Step::Finish { start, arity: 2 });
                self.steps.push(Step::Begin(body));
                self.steps.push(Step::Glyph(separator.glyph()));
                self.steps.push(Step::Head(head));
            }
            Protoform::Enclosed(enclosure, children) => {
                self.out.push(enclosure.opener());
                self.steps.push(Step::Finish {
                    start,
                    arity: children.len(),
                });
                self.steps.push(Step::Glyph(enclosure.closer()));
                if enclosure.spaced() && !children.is_empty() {
                    self.steps.push(Step::Space);
                    self.schedule(children);
                    self.steps.push(Step::Space);
                } else {
                    self.schedule(children);
                }
            }
            Protoform::Quoted(text) => {
                text.write_within(Boundary::CurlyQuotes, &mut self.out);
                self.finish(start, 0);
            }
            Protoform::Parenthesized(text) => {
                text.write_within(Boundary::Parentheses, &mut self.out);
                self.finish(start, 0);
            }
            Protoform::Bare(bare) => {
                self.out.push_str(bare.as_ref());
                self.finish(start, 0);
            }
            Protoform::Qualified(symbol, constraints) => self.qualified(symbol, constraints),
        }
    }

    fn head(&mut self, head: &'a Head) {
        let start = self.out.len();
        match head {
            Head::Symbol(symbol) => {
                self.out.push_str(symbol.as_ref());
                self.finish(start, 0);
            }
            Head::Qualified(symbol, constraints) => self.qualified(symbol, constraints),
        }
    }

    fn qualified(&mut self, symbol: &'a crate::Symbol, constraints: &'a [Protoform]) {
        let start = self.out.len();
        self.out.push_str(symbol.as_ref());
        self.out.push(Enclosure::Angled.opener());
        self.steps.push(Step::Finish {
            start,
            arity: constraints.len(),
        });
        self.steps.push(Step::Glyph(Enclosure::Angled.closer()));
        self.schedule(constraints);
    }

    fn finish(&mut self, start: usize, arity: usize) {
        let children = self.situations.split_off(self.situations.len() - arity);
        self.situations.push(Situation {
            extent: Extent(start as Integer, self.out.len() as Integer),
            children,
        });
    }

    fn schedule(&mut self, forms: &'a [Protoform]) {
        for (index, form) in forms.iter().enumerate().rev() {
            self.steps.push(Step::Begin(form));
            if index > 0 {
                self.steps.push(Step::Space);
            }
        }
    }

    fn write(mut self) -> Situated<String> {
        while let Some(step) = self.steps.pop() {
            match step {
                Step::Begin(form) => self.begin(form),
                Step::Head(head) => self.head(head),
                Step::Finish { start, arity } => self.finish(start, arity),
                Step::Glyph(glyph) => self.out.push(glyph),
                Step::Space => self.out.push(' '),
            }
        }
        let situation = self.situations.pop().unwrap_or(Situation {
            extent: Extent(0, 0),
            children: vec![],
        });
        Situated(situation, self.out)
    }
}

impl Situating for Protoform {
    fn situate(&self) -> Situated<String> {
        Writer {
            out: String::new(),
            steps: vec![Step::Begin(self)],
            situations: Vec::new(),
        }
        .write()
    }
}

trait Writing {
    fn write_text(&self) -> String;
}

impl Writing for Protoform {
    fn write_text(&self) -> String {
        self.situate().1
    }
}

impl Situating for Situated<Protoform> {
    fn situate(&self) -> Situated<String> {
        self.1.situate()
    }
}

impl Writing for Situated<Protoform> {
    fn write_text(&self) -> String {
        self.1.write_text()
    }
}

impl Writing for Delineation {
    fn write_text(&self) -> String {
        let mut out = String::new();
        for (index, structure) in self.0.iter().enumerate() {
            if index > 0 {
                out.push(' ');
            }
            out.push_str(&structure.write_text());
        }
        out
    }
}

impl<T, C> Textualizable<C> for T
where
    T: Conceivable<C, Fault = Infallible>,
    C: Protosizable<Fault = Infallible>,
{
    fn textualize(&self) -> String {
        self.conceive()
            .expect("an infallible ascent cannot fault")
            .1
            .protosize()
            .expect("an infallible structural projection cannot fault")
            .write_text()
    }
}
