//! Iterative destruction for structural trees.

use crate::{Extent, Protos};

trait Emptying {
    fn empty() -> Self;
}

impl Emptying for Protos {
    fn empty() -> Self {
        Self::Bare {
            extent: Extent { start: 0, end: 0 },
            text: String::new(),
        }
    }
}

trait Shedding {
    fn shed(&mut self, work: &mut Vec<Protos>);
}

impl Shedding for Protos {
    fn shed(&mut self, work: &mut Vec<Protos>) {
        match self {
            Self::Headed {
                constraints, body, ..
            } => {
                if let Some(constraints) = constraints.take() {
                    work.push(*constraints);
                }
                work.push(*std::mem::replace(body, Box::new(Self::empty())));
            }
            Self::Enclosed { children, .. } => work.append(children),
            Self::Opaque { .. } | Self::Bare { .. } => {}
        }
    }
}

impl Drop for Protos {
    fn drop(&mut self) {
        let mut work = Vec::new();
        self.shed(&mut work);
        while let Some(mut form) = work.pop() {
            form.shed(&mut work);
        }
    }
}
