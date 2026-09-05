//! Iterative drop for the protoform tree: a deep tree never recurses on its way out.

use crate::anatomy::{Head, Protoform};

/// The kind whose capability moves a node's children out onto a worklist, leaving the node a leaf.
pub(crate) trait Shedding {
    /// Shed the children onto the worklist.
    fn shed(&mut self, work: &mut Vec<Protoform>);
}

impl Shedding for Head {
    fn shed(&mut self, work: &mut Vec<Protoform>) {
        if let Head::Qualified(_, constraints) = self {
            work.append(constraints);
        }
    }
}

impl Shedding for Protoform {
    fn shed(&mut self, work: &mut Vec<Protoform>) {
        match self {
            Protoform::Headed(head, _, body) => {
                head.shed(work);
                work.push(std::mem::replace(
                    body.as_mut(),
                    Protoform::Bare(Head::Symbol(String::new())),
                ));
            }
            Protoform::Enclosed(_, children) => work.append(children),
            Protoform::Bare(head) => head.shed(work),
            Protoform::Opaque(..) => {}
        }
    }
}

impl Drop for Protoform {
    fn drop(&mut self) {
        let mut work = Vec::new();
        self.shed(&mut work);
        while let Some(mut form) = work.pop() {
            form.shed(&mut work);
        }
    }
}
