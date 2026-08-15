trait Behavior {
    fn act(&self);
}

mod source {
    pub struct Data {
        value: u8,
    }
}

use source::Data as ImportedData;
pub use source::Data as ReexportedData;
type FirstAlias = ReexportedData;
type SecondAlias = FirstAlias;

impl Behavior for ImportedData {
    fn act(&self) {}
}

impl Behavior for SecondAlias {
    fn act(&self) {}
}

use source::*;

impl Behavior for Data {
    fn act(&self) {}
}

#[path = "zst-alias-good-child.rs"]
mod attributed;
