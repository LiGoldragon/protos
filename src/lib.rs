//! Implementation-free component contracts for the Protos family.
//!
//! Concrete identity, name, and structural machinery lives in its canonical
//! micro-repository. This crate owns only the contracts that relate components.

mod capsule;
mod textual_capsule;

pub use capsule::{
    Capsule, CapsuleKind, CapsuleVerificationError, CapsuleVerificationResult, ShortIdentifier,
};
pub use content_identity::{CapsuleNameTreeDomain, ContentHash, HashDomain, ShortCode};
pub use textual_capsule::{CapsuleUnviewContext, TextualCapsuleAssociation};
