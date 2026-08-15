trait Behavior {
    fn act(&self);
}

mod source {
    pub struct AliasZst;
    pub struct GlobZst;
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

impl Behavior for Second {
    fn act(&self) {}
}

impl Behavior for transitive::Second {
    fn act(&self) {}
}

#[path = "zst-alias-bad-child.rs"]
mod attributed;

#[path = "zst-alias-repeated-child.rs"]
mod first_instance;

#[path = "zst-alias-repeated-child.rs"]
mod second_instance;
