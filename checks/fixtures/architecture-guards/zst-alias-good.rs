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

mod shadow_source {
    pub struct Node;
}

mod data_exports {
    pub mod alias_source {
        pub struct Data {
            value: u8,
        }
    }
    pub use crate::data_exports::alias_source as alias;
}

use shadow_source::*;
struct Node {
    value: u8,
}

impl Behavior for Node {
    fn act(&self) {}
}

impl Behavior for data_exports::alias::Data {
    fn act(&self) {}
}

use data_exports::alias as ImportedDataNamespace;

impl Behavior for ImportedDataNamespace::Data {
    fn act(&self) {}
}

#[path = "zst-alias-good-child.rs"]
mod attributed;

#[path = "zst-alias-repeated-good-child.rs"]
mod first_instance;

#[path = "zst-alias-repeated-good-child.rs"]
mod second_instance;
