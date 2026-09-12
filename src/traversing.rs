//! Iterative `Clone`, `PartialEq` and `Drop` for structural trees.
//!
//! A structural tree is built as well as read. The reader bounds its own
//! descent, but a tree composed in memory is as deep as the composition that
//! built it, so every traversal of `Protos` walks an explicit stack. All
//! three read a node the same way: what it holds besides its children, and
//! its children.

use crate::{Boundary, Enclosure, Extent, Protos, Separator, Symbol};

/// A node minus its children: everything it holds itself, an enclosure's
/// child count and a head's constraints standing for the children they lead
/// to. Two nodes with equal aspects differ only in what their children are.
#[derive(PartialEq, Eq)]
pub(crate) enum Aspect<'a> {
    Headed(Extent, &'a Symbol, bool, Separator),
    Enclosed(Extent, Enclosure, usize),
    Opaque(Extent, Boundary, &'a str),
    Bare(Extent, &'a str),
}

pub(crate) trait Structuring {
    /// This node's children, in rendering order: a head's constraints before
    /// its body, an enclosure's children in their own order.
    fn children(&self) -> Vec<&Protos>;
    /// This node with its children left empty, ready to adopt them back.
    fn skeleton(&self) -> Protos;
    /// Take `children` back in the order `children` yields them.
    fn adopt(&mut self, children: Vec<Protos>);
    /// Hand this node's owned children to `work`, leaving it childless.
    fn shed(&mut self, work: &mut Vec<Protos>);
    /// The same children, to be written into.
    fn children_mut(&mut self) -> Vec<&mut Protos>;
    /// Where this node's own extent is written.
    fn extent_mut(&mut self) -> &mut Extent;
    /// Everything this node holds itself, its children apart.
    fn aspect(&self) -> Aspect<'_>;
    fn empty() -> Self;
}
impl Structuring for Protos {
    fn children(&self) -> Vec<&Protos> {
        match self {
            Self::Headed {
                constraints, body, ..
            } => match constraints {
                Some(constraints) => vec![constraints, body],
                None => vec![body],
            },
            Self::Enclosed { children, .. } => children.iter().collect(),
            Self::Opaque { .. } | Self::Bare { .. } => Vec::new(),
        }
    }
    fn children_mut(&mut self) -> Vec<&mut Protos> {
        match self {
            Self::Headed {
                constraints, body, ..
            } => match constraints {
                Some(constraints) => vec![constraints, body],
                None => vec![body],
            },
            Self::Enclosed { children, .. } => children.iter_mut().collect(),
            Self::Opaque { .. } | Self::Bare { .. } => Vec::new(),
        }
    }
    fn extent_mut(&mut self) -> &mut Extent {
        match self {
            Self::Headed { extent, .. }
            | Self::Enclosed { extent, .. }
            | Self::Opaque { extent, .. }
            | Self::Bare { extent, .. } => extent,
        }
    }
    fn skeleton(&self) -> Protos {
        match self.aspect() {
            Aspect::Headed(extent, head, constrained, separator) => Self::Headed {
                extent,
                head: head.clone(),
                constraints: constrained.then(|| Box::new(<Self as Structuring>::empty())),
                separator,
                body: Box::new(<Self as Structuring>::empty()),
            },
            Aspect::Enclosed(extent, enclosure, _) => Self::Enclosed {
                extent,
                enclosure,
                children: Vec::new(),
            },
            Aspect::Opaque(extent, boundary, content) => Self::Opaque {
                extent,
                boundary,
                content: content.to_owned(),
            },
            Aspect::Bare(extent, text) => Self::Bare {
                extent,
                text: text.to_owned(),
            },
        }
    }
    fn aspect(&self) -> Aspect<'_> {
        match self {
            Self::Headed {
                extent,
                head,
                constraints,
                separator,
                ..
            } => Aspect::Headed(*extent, head, constraints.is_some(), *separator),
            Self::Enclosed {
                extent,
                enclosure,
                children,
            } => Aspect::Enclosed(*extent, *enclosure, children.len()),
            Self::Opaque {
                extent,
                boundary,
                content,
            } => Aspect::Opaque(*extent, *boundary, content),
            Self::Bare { extent, text } => Aspect::Bare(*extent, text),
        }
    }
    fn adopt(&mut self, mut children: Vec<Protos>) {
        match self {
            Self::Headed {
                constraints, body, ..
            } => {
                **body = children.pop().expect("a body");
                if let Some(constraints) = constraints {
                    **constraints = children.pop().expect("constraints");
                }
            }
            Self::Enclosed { children: held, .. } => *held = children,
            Self::Opaque { .. } | Self::Bare { .. } => {}
        }
    }
    fn shed(&mut self, work: &mut Vec<Protos>) {
        match self {
            Self::Headed {
                constraints, body, ..
            } => {
                if let Some(constraints) = constraints.take() {
                    work.push(*constraints);
                }
                work.push(*std::mem::replace(
                    body,
                    Box::new(<Self as Structuring>::empty()),
                ));
            }
            Self::Enclosed { children, .. } => work.append(children),
            Self::Opaque { .. } | Self::Bare { .. } => {}
        }
    }
    fn empty() -> Self {
        Self::Bare {
            extent: Extent { start: 0, end: 0 },
            text: String::new(),
        }
    }
}

enum CloneStep<'a> {
    Visit(&'a Protos),
    Adopt(Protos, usize),
}

impl Clone for Protos {
    fn clone(&self) -> Self {
        let mut steps = vec![CloneStep::Visit(self)];
        let mut cloned: Vec<Self> = Vec::new();
        while let Some(step) = steps.pop() {
            match step {
                CloneStep::Visit(node) => {
                    let children = node.children();
                    steps.push(CloneStep::Adopt(node.skeleton(), children.len()));
                    steps.extend(children.into_iter().rev().map(CloneStep::Visit));
                }
                CloneStep::Adopt(mut node, count) => {
                    let children = cloned.split_off(cloned.len() - count);
                    node.adopt(children);
                    cloned.push(node);
                }
            }
        }
        cloned.pop().expect("a cloned tree")
    }
}

impl PartialEq for Protos {
    fn eq(&self, other: &Self) -> bool {
        let mut pairs = vec![(self, other)];
        while let Some((left, right)) = pairs.pop() {
            if left.aspect() != right.aspect() {
                return false;
            }
            pairs.extend(left.children().into_iter().zip(right.children()));
        }
        true
    }
}
impl Eq for Protos {}

impl Drop for Protos {
    fn drop(&mut self) {
        let mut work = Vec::new();
        self.shed(&mut work);
        while let Some(mut node) = work.pop() {
            node.shed(&mut work);
        }
    }
}
