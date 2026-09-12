//! Iterative `Clone`, `PartialEq` and `Debug` for structural trees.
//!
//! A structural tree is built as well as read. The reader bounds its own
//! descent, but a tree composed in memory is as deep as the composition that
//! built it, and every traversal of `Protos` is therefore written with an
//! explicit stack — as destruction, printing and canonicalization already are.

use crate::{Boundary, Enclosure, Extent, Protos, Separator, Symbol};
use std::fmt;

enum CloneStep<'a> {
    Visit(&'a Protos),
    Headed {
        extent: Extent,
        head: &'a Symbol,
        constrained: bool,
        separator: Separator,
    },
    Enclosed {
        extent: Extent,
        enclosure: Enclosure,
        count: usize,
    },
}

impl Clone for Protos {
    fn clone(&self) -> Self {
        let mut steps = vec![CloneStep::Visit(self)];
        let mut cloned: Vec<Self> = Vec::new();
        while let Some(step) = steps.pop() {
            match step {
                CloneStep::Visit(form) => match form {
                    Self::Headed {
                        extent,
                        head,
                        constraints,
                        separator,
                        body,
                    } => {
                        steps.push(CloneStep::Headed {
                            extent: *extent,
                            head,
                            constrained: constraints.is_some(),
                            separator: *separator,
                        });
                        steps.push(CloneStep::Visit(body));
                        if let Some(constraints) = constraints {
                            steps.push(CloneStep::Visit(constraints));
                        }
                    }
                    Self::Enclosed {
                        extent,
                        enclosure,
                        children,
                    } => {
                        steps.push(CloneStep::Enclosed {
                            extent: *extent,
                            enclosure: *enclosure,
                            count: children.len(),
                        });
                        for child in children.iter().rev() {
                            steps.push(CloneStep::Visit(child));
                        }
                    }
                    Self::Opaque {
                        extent,
                        boundary,
                        content,
                    } => cloned.push(Self::Opaque {
                        extent: *extent,
                        boundary: *boundary,
                        content: content.clone(),
                    }),
                    Self::Bare { extent, text } => cloned.push(Self::Bare {
                        extent: *extent,
                        text: text.clone(),
                    }),
                },
                CloneStep::Headed {
                    extent,
                    head,
                    constrained,
                    separator,
                } => {
                    let body = cloned.pop().expect("a cloned body");
                    let constraints = if constrained {
                        Some(Box::new(cloned.pop().expect("cloned constraints")))
                    } else {
                        None
                    };
                    cloned.push(Self::Headed {
                        extent,
                        head: head.clone(),
                        constraints,
                        separator,
                        body: Box::new(body),
                    });
                }
                CloneStep::Enclosed {
                    extent,
                    enclosure,
                    count,
                } => {
                    let children = cloned.split_off(cloned.len() - count);
                    cloned.push(Self::Enclosed {
                        extent,
                        enclosure,
                        children,
                    });
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
            match (left, right) {
                (
                    Self::Headed {
                        extent: left_extent,
                        head: left_head,
                        constraints: left_constraints,
                        separator: left_separator,
                        body: left_body,
                    },
                    Self::Headed {
                        extent: right_extent,
                        head: right_head,
                        constraints: right_constraints,
                        separator: right_separator,
                        body: right_body,
                    },
                ) => {
                    if left_extent != right_extent
                        || left_head != right_head
                        || left_separator != right_separator
                        || left_constraints.is_some() != right_constraints.is_some()
                    {
                        return false;
                    }
                    pairs.push((left_body, right_body));
                    if let (Some(left), Some(right)) = (left_constraints, right_constraints) {
                        pairs.push((left, right));
                    }
                }
                (
                    Self::Enclosed {
                        extent: left_extent,
                        enclosure: left_enclosure,
                        children: left_children,
                    },
                    Self::Enclosed {
                        extent: right_extent,
                        enclosure: right_enclosure,
                        children: right_children,
                    },
                ) => {
                    if left_extent != right_extent
                        || left_enclosure != right_enclosure
                        || left_children.len() != right_children.len()
                    {
                        return false;
                    }
                    pairs.extend(left_children.iter().zip(right_children));
                }
                (
                    Self::Opaque {
                        extent: left_extent,
                        boundary: left_boundary,
                        content: left_content,
                    },
                    Self::Opaque {
                        extent: right_extent,
                        boundary: right_boundary,
                        content: right_content,
                    },
                ) => {
                    if left_extent != right_extent
                        || left_boundary != right_boundary
                        || left_content != right_content
                    {
                        return false;
                    }
                }
                (
                    Self::Bare {
                        extent: left_extent,
                        text: left_text,
                    },
                    Self::Bare {
                        extent: right_extent,
                        text: right_text,
                    },
                ) => {
                    if left_extent != right_extent || left_text != right_text {
                        return false;
                    }
                }
                _ => return false,
            }
        }
        true
    }
}
impl Eq for Protos {}

enum ShowStep<'a> {
    Form(&'a Protos),
    Text(&'static str),
    Owned(String),
}

trait Showing {
    /// The steps that write one node, its children left as further nodes.
    fn steps(&self) -> Vec<ShowStep<'_>>;
    fn opaque(extent: Extent, boundary: Boundary, content: &str) -> String;
}
impl Showing for Protos {
    fn steps(&self) -> Vec<ShowStep<'_>> {
        match self {
            Self::Headed {
                extent,
                head,
                constraints,
                separator,
                body,
            } => {
                let mut steps = vec![ShowStep::Owned(format!(
                    "Headed {{ extent: {extent:?}, head: {head:?}, constraints: "
                ))];
                match constraints {
                    Some(constraints) => {
                        steps.push(ShowStep::Text("Some("));
                        steps.push(ShowStep::Form(constraints));
                        steps.push(ShowStep::Text(")"));
                    }
                    None => steps.push(ShowStep::Text("None")),
                }
                steps.push(ShowStep::Owned(format!(
                    ", separator: {separator:?}, body: "
                )));
                steps.push(ShowStep::Form(body));
                steps.push(ShowStep::Text(" }"));
                steps
            }
            Self::Enclosed {
                extent,
                enclosure,
                children,
            } => {
                let mut steps = vec![ShowStep::Owned(format!(
                    "Enclosed {{ extent: {extent:?}, enclosure: {enclosure:?}, children: ["
                ))];
                for (index, child) in children.iter().enumerate() {
                    if index > 0 {
                        steps.push(ShowStep::Text(", "));
                    }
                    steps.push(ShowStep::Form(child));
                }
                steps.push(ShowStep::Text("] }"));
                steps
            }
            Self::Opaque {
                extent,
                boundary,
                content,
            } => vec![ShowStep::Owned(Self::opaque(*extent, *boundary, content))],
            Self::Bare { extent, text } => vec![ShowStep::Owned(format!(
                "Bare {{ extent: {extent:?}, text: {text:?} }}"
            ))],
        }
    }
    fn opaque(extent: Extent, boundary: Boundary, content: &str) -> String {
        format!("Opaque {{ extent: {extent:?}, boundary: {boundary:?}, content: {content:?} }}")
    }
}

impl fmt::Debug for Protos {
    /// The one-line form, whatever the alternate flag asks. A tree is written
    /// from an explicit stack so that depth cannot exhaust the machine, and an
    /// indented form would carry that depth into the output.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut steps = vec![ShowStep::Form(self)];
        while let Some(step) = steps.pop() {
            match step {
                ShowStep::Text(text) => formatter.write_str(text)?,
                ShowStep::Owned(text) => formatter.write_str(&text)?,
                ShowStep::Form(form) => steps.extend(form.steps().into_iter().rev()),
            }
        }
        Ok(())
    }
}
