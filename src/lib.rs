//! Implementation-free component contracts for the Protos family.
//!
//! Concrete identity, name, and structural machinery lives in its canonical
//! micro-repository. This crate owns only the contracts that relate components.

mod capsule;
mod interface;
mod population;
mod textual_capsule;
mod wire_identity;

pub use capsule::{
    Capsule, CapsuleArchiveError, CapsuleKind, CapsuleKindMismatch, Ethos, Logos, Nomos,
};
pub use content_identity::{CapsuleIdentity, CapsuleIdentityVariant, ContentAddressedHash};
pub use interface::{Input, Output, Refusal};
pub use population::EncodedPopulation;
pub use textual_capsule::TextualCapsuleAssociation;
pub use wire_identity::{
    ACTIVE_WIRE_CONTRACT_ALLOCATIONS, ActiveWireContractAllocation, META_SIGNAL_LOJIX_BINDING,
    META_SIGNAL_LOJIX_CONTRACT_ID, META_SIGNAL_LOJIX_WIRE_REVISION, META_SIGNAL_SPIRIT_BINDING,
    META_SIGNAL_SPIRIT_CONTRACT_ID, META_SIGNAL_SPIRIT_WIRE_REVISION,
    RETIRED_WIRE_CONTRACT_ALLOCATIONS, RetiredWireContractAllocation, SIGNAL_LOJIX_BINDING,
    SIGNAL_LOJIX_CONTRACT_ID, SIGNAL_LOJIX_WIRE_REVISION, SIGNAL_SEMA_TRANSLATOR_BINDING,
    SIGNAL_SEMA_TRANSLATOR_CONTRACT_ID, SIGNAL_SEMA_TRANSLATOR_WIRE_REVISION,
    SIGNAL_SPIRIT_BINDING, SIGNAL_SPIRIT_CONTRACT_ID, SIGNAL_SPIRIT_JUDGE_BINDING,
    SIGNAL_SPIRIT_JUDGE_CONTRACT_ID, SIGNAL_SPIRIT_JUDGE_WIRE_REVISION,
    SIGNAL_SPIRIT_WIRE_REVISION, WIRE_CONTRACT_ALLOCATIONS, WireContractAllocation,
    WireContractFamily,
};
