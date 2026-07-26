//! Append-only allocation data for Protos wire-contract families.
//!
//! This module assigns identity; it does not allocate identity at runtime.

use std::num::{NonZeroU16, NonZeroU32};

use signal_frame::{ContractBinding, ContractId, WireRevision};

/// Canonical identity of a declared Protos wire-contract family.
///
/// Pilot and migration mirrors which are aliases select the same family
/// variant. They do not add variants or allocation records.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum WireContractFamily {
    /// Ordinary `signal-spirit` contract family.
    SignalSpirit,
    /// Owner-only `meta-signal-spirit` contract family.
    MetaSignalSpirit,
}

impl WireContractFamily {
    /// Every declared family, in allocation order.
    pub const ALL: [Self; 2] = [Self::SignalSpirit, Self::MetaSignalSpirit];

    /// Return this family's active allocation, or `None` after retirement.
    pub const fn active_allocation(self) -> Option<&'static ActiveWireContractAllocation> {
        match self {
            Self::SignalSpirit => Some(&ACTIVE_WIRE_CONTRACT_ALLOCATIONS[0]),
            Self::MetaSignalSpirit => Some(&ACTIVE_WIRE_CONTRACT_ALLOCATIONS[1]),
        }
    }

    /// Return the binding new encoders must emit, or `None` after retirement.
    pub const fn current_binding(self) -> Option<ContractBinding> {
        match self.active_allocation() {
            Some(allocation) => Some(allocation.current_binding()),
            None => None,
        }
    }

    /// Return every retained decoder binding, or `None` after retirement.
    pub const fn supported_bindings(self) -> Option<&'static [ContractBinding]> {
        match self.active_allocation() {
            Some(allocation) => Some(allocation.supported_bindings()),
            None => None,
        }
    }

    /// Whether this family retains an explicit decoder for `binding`.
    pub fn supports_binding(self, binding: ContractBinding) -> bool {
        self.supported_bindings()
            .is_some_and(|bindings| bindings.contains(&binding))
    }
}

/// One active append-only family allocation.
///
/// Fields are private so downstream build scripts consume the canonical table
/// rather than constructing records with caller-selected numbers.
///
/// ```compile_fail
/// use protos::{ActiveWireContractAllocation, WireContractFamily};
///
/// let _forged = ActiveWireContractAllocation {
///     family: WireContractFamily::SignalSpirit,
///     current: protos::SIGNAL_SPIRIT_BINDING,
///     supported: &[],
/// };
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActiveWireContractAllocation {
    family: WireContractFamily,
    current: ContractBinding,
    supported: &'static [ContractBinding],
}

impl ActiveWireContractAllocation {
    const fn new(
        family: WireContractFamily,
        current: ContractBinding,
        supported: &'static [ContractBinding],
    ) -> Self {
        Self {
            family,
            current,
            supported,
        }
    }

    /// Canonical family which owns this allocation.
    pub const fn family(self) -> WireContractFamily {
        self.family
    }

    /// Binding new encoders must emit.
    pub const fn current_binding(self) -> ContractBinding {
        self.current
    }

    /// Every explicitly supported decoder binding, oldest first.
    ///
    /// A breaking body change appends a higher revision here and changes
    /// `current`; it does not discard the old decoder binding.
    pub const fn supported_bindings(self) -> &'static [ContractBinding] {
        self.supported
    }
}

/// Permanent reservation for a retired contract family.
///
/// Tombstones remain in [`RETIRED_WIRE_CONTRACT_ALLOCATIONS`] forever. Their
/// contract IDs can never return to [`ACTIVE_WIRE_CONTRACT_ALLOCATIONS`].
/// Construction is private for the same reason as active allocations.
///
/// ```compile_fail
/// use protos::{RetiredWireContractAllocation, WireContractFamily};
///
/// let _forged = RetiredWireContractAllocation {
///     family: WireContractFamily::SignalSpirit,
///     contract_id: protos::SIGNAL_SPIRIT_CONTRACT_ID,
///     binding_history: &[protos::SIGNAL_SPIRIT_BINDING],
/// };
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RetiredWireContractAllocation {
    family: WireContractFamily,
    contract_id: ContractId,
    binding_history: &'static [ContractBinding],
}

impl RetiredWireContractAllocation {
    /// Canonical family whose allocation is retired.
    pub const fn family(self) -> WireContractFamily {
        self.family
    }

    /// Complete retained binding history, oldest first.
    pub const fn binding_history(self) -> &'static [ContractBinding] {
        self.binding_history
    }

    /// Permanently reserved contract ID.
    pub const fn contract_id(self) -> ContractId {
        self.contract_id
    }
}

const fn contract_id(value: u32) -> ContractId {
    let Some(value) = NonZeroU32::new(value) else {
        panic!("wire contract IDs must be nonzero");
    };
    ContractId::new(value)
}

const fn wire_revision(value: u16) -> WireRevision {
    let Some(value) = NonZeroU16::new(value) else {
        panic!("wire revisions must be nonzero");
    };
    WireRevision::new(value)
}

/// Stable numeric identity of the ordinary `signal-spirit` family.
pub const SIGNAL_SPIRIT_CONTRACT_ID: ContractId = contract_id(1);
/// Current archived-body revision of the ordinary `signal-spirit` family.
pub const SIGNAL_SPIRIT_WIRE_REVISION: WireRevision = wire_revision(1);
/// Current binding emitted by ordinary `signal-spirit` encoders.
pub const SIGNAL_SPIRIT_BINDING: ContractBinding =
    ContractBinding::new(SIGNAL_SPIRIT_CONTRACT_ID, SIGNAL_SPIRIT_WIRE_REVISION);

/// Stable numeric identity of the owner-only `meta-signal-spirit` family.
pub const META_SIGNAL_SPIRIT_CONTRACT_ID: ContractId = contract_id(2);
/// Current archived-body revision of the owner-only `meta-signal-spirit` family.
pub const META_SIGNAL_SPIRIT_WIRE_REVISION: WireRevision = wire_revision(1);
/// Current binding emitted by owner-only `meta-signal-spirit` encoders.
pub const META_SIGNAL_SPIRIT_BINDING: ContractBinding = ContractBinding::new(
    META_SIGNAL_SPIRIT_CONTRACT_ID,
    META_SIGNAL_SPIRIT_WIRE_REVISION,
);

const SIGNAL_SPIRIT_BINDINGS: &[ContractBinding] = &[SIGNAL_SPIRIT_BINDING];
const META_SIGNAL_SPIRIT_BINDINGS: &[ContractBinding] = &[META_SIGNAL_SPIRIT_BINDING];

/// Canonical active allocations, in stable contract-ID order.
///
/// Entries are appended only after a family has been proven canonical. Aliases
/// and unbound legacy frames do not receive entries.
pub const ACTIVE_WIRE_CONTRACT_ALLOCATIONS: &[ActiveWireContractAllocation] = &[
    ActiveWireContractAllocation::new(
        WireContractFamily::SignalSpirit,
        SIGNAL_SPIRIT_BINDING,
        SIGNAL_SPIRIT_BINDINGS,
    ),
    ActiveWireContractAllocation::new(
        WireContractFamily::MetaSignalSpirit,
        META_SIGNAL_SPIRIT_BINDING,
        META_SIGNAL_SPIRIT_BINDINGS,
    ),
];

/// Permanent retired-ID reservations.
///
/// No proven family is retired in the initial allocation set.
pub const RETIRED_WIRE_CONTRACT_ALLOCATIONS: &[RetiredWireContractAllocation] = &[];
