//! Situation: lookup by path, and the iterative drop of a deep tree.

use crate::anatomy::{Extent, Integer, Situation};
use crate::kinds::Locating;

/// The situation of nothing: what a missing child resolves to.
static NOWHERE: Situation = Situation {
    extent: Extent(0, 0),
    children: Vec::new(),
};

impl Locating for Situation {
    fn locate(&self, path: &[Integer]) -> Option<Extent> {
        let mut here = self;
        for &index in path {
            here = here.children.get(usize::try_from(index).ok()?)?;
        }
        Some(here.extent)
    }

    fn part(&self, index: Integer) -> &Situation {
        match usize::try_from(index) {
            Ok(index) => self.children.get(index).unwrap_or(&NOWHERE),
            Err(_) => &NOWHERE,
        }
    }
}

impl Drop for Situation {
    fn drop(&mut self) {
        let mut work = std::mem::take(&mut self.children);
        while let Some(mut situation) = work.pop() {
            work.append(&mut situation.children);
        }
    }
}
