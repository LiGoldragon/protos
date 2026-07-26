//! Implementation-free component contracts for the Protos family.
//!
//! Concrete identity, name, and structural machinery lives in its canonical
//! micro-repository. This crate owns only the contracts that relate components.

mod capsule;
mod textual_capsule;
mod wire_identity;

pub use capsule::{
    Capsule, CapsuleKind, CapsuleVerificationError, CapsuleVerificationResult, ShortIdentifier,
};
pub use content_identity::{CapsuleNameTreeDomain, ContentHash, HashDomain, ShortCode};
pub use textual_capsule::TextualCapsuleAssociation;
pub use wire_identity::{
    ACTIVE_WIRE_CONTRACT_ALLOCATIONS, ActiveWireContractAllocation, META_SIGNAL_SPIRIT_BINDING,
    META_SIGNAL_SPIRIT_CONTRACT_ID, META_SIGNAL_SPIRIT_WIRE_REVISION,
    RETIRED_WIRE_CONTRACT_ALLOCATIONS, RetiredWireContractAllocation, SIGNAL_SPIRIT_BINDING,
    SIGNAL_SPIRIT_CONTRACT_ID, SIGNAL_SPIRIT_JUDGE_BINDING, SIGNAL_SPIRIT_JUDGE_CONTRACT_ID,
    SIGNAL_SPIRIT_JUDGE_WIRE_REVISION, SIGNAL_SPIRIT_WIRE_REVISION, WIRE_CONTRACT_ALLOCATIONS,
    WireContractAllocation, WireContractFamily,
};
