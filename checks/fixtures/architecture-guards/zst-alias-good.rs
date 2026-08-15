trait Behavior {
    fn act(&self);
}

mod source {
    pub struct Data {
        value: u8,
    }

    pub mod nested {
        pub struct Unit {
            value: u8,
        }
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

mod nested {
    pub struct Unit {
        value: u8,
    }
}

impl Behavior for nested::Unit {
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

pub mod module_source {
    pub struct Unit {
        value: u8,
    }
}

mod module_exports {
    pub use crate::module_source as alias;
}

pub mod restricted_parent {
    mod restricted_source {
        pub(in super::super) struct GrandparentData {
            value: u8,
        }
    }

    pub mod nested {
        pub use super::restricted_source::*;
    }
}

pub mod path_forms {
    pub struct CratePathData {
        value: u8,
    }
    pub struct SuperPathData {
        value: u8,
    }

    pub mod source {
        pub struct MultiSuperData {
            value: u8,
        }
    }

    pub mod branch {
        pub struct SelfPathData {
            value: u8,
        }

        impl Behavior for self::SelfPathData {
            fn act(&self) {}
        }

        impl Behavior for super::SuperPathData {
            fn act(&self) {}
        }

        pub mod leaf {
            impl Behavior for super::super::source::MultiSuperData {
                fn act(&self) {}
            }

            impl Behavior for crate::path_forms::CratePathData {
                fn act(&self) {}
            }
        }
    }
}

mod grouped_reexports {
    pub use crate::source::{self};
}

mod grouped_reexport_collision {
    use crate::grouped_reexports::*;

    mod source {
        pub struct AliasZst {
            value: u8,
        }
    }

    impl Behavior for source::AliasZst {
        fn act(&self) {}
    }
}

use module_exports::*;
mod alias {
    pub struct Unit {
        value: u8,
    }
}

impl Behavior for alias::Unit {
    fn act(&self) {}
}

use restricted_parent::nested::*;

impl Behavior for GrandparentData {
    fn act(&self) {}
}

#[path = "zst-alias-good-child.rs"]
mod attributed;

#[path = "zst-alias-repeated-good-child.rs"]
mod first_instance;

#[path = "zst-alias-repeated-good-child.rs"]
mod second_instance;
