trait Behavior {
    fn act(&self);
}

mod source {
    pub struct AliasZst;
    pub struct GlobZst;

    pub mod nested {
        pub struct Unit;
    }
}

pub mod module_source {
    pub struct Unit;
}

mod module_exports {
    pub use crate::module_source as alias;
}

pub mod restricted_parent {
    mod restricted_source {
        pub(in super::super) struct GrandparentZst;
    }

    pub mod nested {
        pub use super::restricted_source::*;
    }
}

pub mod path_forms {
    pub struct CratePathZst;
    pub struct SuperPathZst;

    pub mod source {
        pub struct MultiSuperZst;
    }

    pub mod branch {
        pub struct SelfPathZst;

        impl Behavior for self::SelfPathZst {
            fn act(&self) {}
        }

        impl Behavior for super::SuperPathZst {
            fn act(&self) {}
        }

        pub mod leaf {
            impl Behavior for super::super::source::MultiSuperZst {
                fn act(&self) {}
            }

            impl Behavior for crate::path_forms::CratePathZst {
                fn act(&self) {}
            }
        }
    }
}

mod exports {
    pub(crate) struct CrateZst;
    pub(crate) use self::CrateZst as First;
    pub(crate) use self::First as Second;
}

mod transitive {
    pub struct ExportedZst;
    pub use self::ExportedZst as First;
    pub use self::First as Second;
}

mod alias_exports {
    pub use crate::source as alias;
}

mod forward_exports {
    pub use First as Second;
    pub use crate::source as First;
}

// This same-name data type must not be confused with source::AliasZst.
struct AliasZst {
    value: u8,
}

use source::AliasZst as ImportedZst;
pub use source::AliasZst as ReexportedZst;
type FirstAlias = ReexportedZst;
type SecondAlias = FirstAlias;

impl Behavior for ImportedZst {
    fn act(&self) {}
}

impl Behavior for ReexportedZst {
    fn act(&self) {}
}

impl Behavior for SecondAlias {
    fn act(&self) {}
}

use source::{self as src};

impl Behavior for src::AliasZst {
    fn act(&self) {}
}

use source::*;
use exports::*;
use transitive::*;

impl Behavior for GlobZst {
    fn act(&self) {}
}

impl Behavior for nested::Unit {
    fn act(&self) {}
}

use module_exports::*;

impl Behavior for alias::Unit {
    fn act(&self) {}
}

use restricted_parent::nested::*;

impl Behavior for GrandparentZst {
    fn act(&self) {}
}

impl Behavior for Second {
    fn act(&self) {}
}

impl Behavior for transitive::Second {
    fn act(&self) {}
}

impl Behavior for alias_exports::alias::AliasZst {
    fn act(&self) {}
}

use alias_exports::alias as ImportedNamespace;

impl Behavior for ImportedNamespace::AliasZst {
    fn act(&self) {}
}

impl Behavior for forward_exports::Second::AliasZst {
    fn act(&self) {}
}

use forward_exports::Second as ForwardNamespace;

impl Behavior for ForwardNamespace::AliasZst {
    fn act(&self) {}
}

#[path = "zst-alias-bad-child.rs"]
mod attributed;

#[path = "zst-alias-repeated-child.rs"]
mod first_instance;

#[path = "zst-alias-repeated-child.rs"]
mod second_instance;
