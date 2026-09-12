//! One iterative machine for every textual traversal of a structural tree.
//!
//! Printing, showing and canonical measurement are the same walk: a node
//! renders into pieces — literal text, and its children as further nodes —
//! and a stack keeps the walk flat, so depth cannot exhaust the machine.
//! The rendition says what the pieces are; the sink says what is done with
//! them.

use crate::core::{Escaping, Glyphing, Separating};
use crate::traversing::Structuring;
use crate::{Canonicalizable, Enclosure, Extent, Protos, Textualizable};
use std::fmt::{self, Write};

/// One piece of a rendering.
enum Step<'a> {
    Glyph(char),
    Text(&'a str),
    Owned(String),
    Node(&'a Protos),
    /// The end of the node opened at this index.
    Close(usize),
}

/// What a rendering makes of a tree.
enum Rendition {
    /// The canonical text of the tree.
    Printed,
    /// The one-line structural form, as a derived `Debug` would write it.
    Shown,
}

/// Where a rendering's pieces go, and what it makes of node boundaries.
trait Sinking {
    fn take(&mut self, text: &str) -> fmt::Result;
    fn take_glyph(&mut self, glyph: char) -> fmt::Result;
    /// A node begins at the current position; its index is the call count.
    fn open(&mut self);
    /// The node opened at `index` ends at the current position.
    fn close(&mut self, index: usize);
}

struct Written<'a, W: Write> {
    sink: &'a mut W,
}
impl<W: Write> Sinking for Written<'_, W> {
    fn take(&mut self, text: &str) -> fmt::Result {
        self.sink.write_str(text)
    }
    fn take_glyph(&mut self, glyph: char) -> fmt::Result {
        self.sink.write_char(glyph)
    }
    fn open(&mut self) {}
    fn close(&mut self, _index: usize) {}
}

/// The byte extents the rendering's own text would occupy.
struct Measured {
    offset: usize,
    extents: Vec<Extent>,
}
impl Sinking for Measured {
    fn take(&mut self, text: &str) -> fmt::Result {
        self.offset += text.len();
        Ok(())
    }
    fn take_glyph(&mut self, glyph: char) -> fmt::Result {
        self.offset += glyph.len_utf8();
        Ok(())
    }
    fn open(&mut self) {
        self.extents.push(Extent {
            start: self.offset,
            end: self.offset,
        });
    }
    fn close(&mut self, index: usize) {
        self.extents[index].end = self.offset;
    }
}

trait Rendering {
    /// The pieces of one node, in reading order, its children left as nodes.
    fn steps<'a>(&self, node: &'a Protos) -> Vec<Step<'a>>;
    fn render<S: Sinking>(&self, root: &Protos, sink: &mut S) -> fmt::Result;
}
impl Rendering for Rendition {
    fn steps<'a>(&self, node: &'a Protos) -> Vec<Step<'a>> {
        match self {
            Self::Printed => match node {
                Protos::Bare { text, .. } => vec![Step::Text(text)],
                Protos::Opaque {
                    boundary, content, ..
                } => {
                    let mut written = String::new();
                    boundary.print_opaque(content, &mut written);
                    vec![Step::Owned(written)]
                }
                Protos::Enclosed {
                    enclosure,
                    children,
                    ..
                } => {
                    let padded = *enclosure != Enclosure::Angled && !children.is_empty();
                    let mut steps = vec![Step::Glyph(enclosure.opener())];
                    if padded {
                        steps.push(Step::Glyph(' '));
                    }
                    for (index, child) in children.iter().enumerate() {
                        if index > 0 {
                            steps.push(Step::Glyph(' '));
                        }
                        steps.push(Step::Node(child));
                    }
                    if padded {
                        steps.push(Step::Glyph(' '));
                    }
                    steps.push(Step::Glyph(enclosure.closer()));
                    steps
                }
                Protos::Headed {
                    head,
                    constraints,
                    separator,
                    body,
                    ..
                } => {
                    let mut steps = vec![Step::Text(&head.0)];
                    if let Some(constraints) = constraints {
                        steps.push(Step::Node(constraints));
                    }
                    steps.push(Step::Glyph(separator.glyph()));
                    steps.push(Step::Node(body));
                    steps
                }
            },
            Self::Shown => match node {
                Protos::Bare { extent, text } => vec![Step::Owned(format!(
                    "Bare {{ extent: {extent:?}, text: {text:?} }}"
                ))],
                Protos::Opaque {
                    extent,
                    boundary,
                    content,
                } => vec![Step::Owned(format!(
                    "Opaque {{ extent: {extent:?}, boundary: {boundary:?}, content: {content:?} }}"
                ))],
                Protos::Enclosed {
                    extent,
                    enclosure,
                    children,
                } => {
                    let mut steps = vec![Step::Owned(format!(
                        "Enclosed {{ extent: {extent:?}, enclosure: {enclosure:?}, children: ["
                    ))];
                    for (index, child) in children.iter().enumerate() {
                        if index > 0 {
                            steps.push(Step::Text(", "));
                        }
                        steps.push(Step::Node(child));
                    }
                    steps.push(Step::Text("] }"));
                    steps
                }
                Protos::Headed {
                    extent,
                    head,
                    constraints,
                    separator,
                    body,
                } => {
                    let mut steps = vec![Step::Owned(format!(
                        "Headed {{ extent: {extent:?}, head: {head:?}, constraints: "
                    ))];
                    match constraints {
                        Some(constraints) => {
                            steps.push(Step::Text("Some("));
                            steps.push(Step::Node(constraints));
                            steps.push(Step::Text(")"));
                        }
                        None => steps.push(Step::Text("None")),
                    }
                    steps.push(Step::Owned(format!(", separator: {separator:?}, body: ")));
                    steps.push(Step::Node(body));
                    steps.push(Step::Text(" }"));
                    steps
                }
            },
        }
    }
    fn render<S: Sinking>(&self, root: &Protos, sink: &mut S) -> fmt::Result {
        let mut steps = vec![Step::Node(root)];
        let mut opened = 0usize;
        while let Some(step) = steps.pop() {
            match step {
                Step::Glyph(glyph) => sink.take_glyph(glyph)?,
                Step::Text(text) => sink.take(text)?,
                Step::Owned(text) => sink.take(&text)?,
                Step::Close(index) => sink.close(index),
                Step::Node(node) => {
                    let index = opened;
                    opened += 1;
                    sink.open();
                    steps.push(Step::Close(index));
                    steps.extend(self.steps(node).into_iter().rev());
                }
            }
        }
        Ok(())
    }
}

/// Take the measured extents, in the order the rendering opened the nodes.
trait Settling {
    fn settle(&mut self, extents: &[Extent]);
}
impl Settling for Protos {
    fn settle(&mut self, extents: &[Extent]) {
        let mut work = vec![self];
        let mut measured = extents.iter();
        while let Some(node) = work.pop() {
            *node.extent_mut() = *measured.next().expect("one extent for every node");
            work.extend(node.children_mut().into_iter().rev());
        }
    }
}

impl Textualizable for Protos {
    fn textualize(&self) -> String {
        let mut text = String::new();
        Rendition::Printed
            .render(self, &mut Written { sink: &mut text })
            .expect("a String accepts every write");
        text
    }
}
impl Canonicalizable for Protos {
    fn canonicalize(&mut self) {
        let mut measured = Measured {
            offset: 0,
            extents: Vec::new(),
        };
        Rendition::Printed
            .render(self, &mut measured)
            .expect("measuring accepts every write");
        self.settle(&measured.extents);
    }
}
impl fmt::Debug for Protos {
    /// The one-line form, whatever the alternate flag asks: an indented form
    /// would carry a built tree's unbounded depth into the output.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        Rendition::Shown.render(self, &mut Written { sink: formatter })
    }
}
