trait Behavior {
    fn act(&self);
}

mod source {
    pub struct AliasZst;
    pub struct GlobZst;
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

use source::*;

impl Behavior for GlobZst {
    fn act(&self) {}
}

#[path = "zst-alias-bad-child.rs"]
mod attributed;
