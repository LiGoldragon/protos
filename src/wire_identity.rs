//! Append-only allocation data for Protos wire-contract families.
//!
//! This module assigns identity; it does not allocate identity at runtime.

use std::num::{NonZeroU16, NonZeroU32};

use signal_frame::{ContractBinding, ContractId, WireRevision};

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
    #[allow(dead_code)] // The initial production registry has no tombstones.
    const fn new(
        family: WireContractFamily,
        contract_id: ContractId,
        binding_history: &'static [ContractBinding],
    ) -> Self {
        Self {
            family,
            contract_id,
            binding_history,
        }
    }

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

/// The total, typed allocation state of one canonical family.
///
/// Match this enum to distinguish active from retired state. Retirement is a
/// first-class allocation state, never an absent lookup result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WireContractAllocation {
    /// The family has a current encoder binding.
    Active(ActiveWireContractAllocation),
    /// The family has a permanent tombstone and decoder history only.
    Retired(RetiredWireContractAllocation),
}

impl WireContractAllocation {
    /// Canonical family which owns this allocation.
    pub const fn family(self) -> WireContractFamily {
        match self {
            Self::Active(allocation) => allocation.family(),
            Self::Retired(allocation) => allocation.family(),
        }
    }

    /// Every retained decoder binding, oldest first.
    pub const fn binding_history(self) -> &'static [ContractBinding] {
        match self {
            Self::Active(allocation) => allocation.supported_bindings(),
            Self::Retired(allocation) => allocation.binding_history(),
        }
    }

    /// Binding new encoders emit, when this allocation is active.
    pub const fn current_binding(self) -> Option<ContractBinding> {
        match self {
            Self::Active(allocation) => Some(allocation.current_binding()),
            Self::Retired(_) => None,
        }
    }
}

#[allow(unused_macro_rules)]
macro_rules! wire_contract_registry {
    (
        active {
            $(
                $active_variant:ident {
                    constants: ($active_contract_id:ident, $active_wire_revision:ident, $active_binding:ident),
                    history: $active_history:ident,
                    allocation: $active_allocation:ident,
                    id: $active_id:literal,
                    revisions: [$($active_revision:literal),+ $(,)?],
                }
            )*
        }
        retired {
            $(
                $retired_variant:ident {
                    constants: ($retired_contract_id:ident, $retired_wire_revision:ident, $retired_binding:ident),
                    history: $retired_history:ident,
                    allocation: $retired_allocation:ident,
                    id: $retired_id:literal,
                    revisions: [$($retired_revision:literal),+ $(,)?],
                }
            )*
        }
    ) => {
        /// Canonical identity of a declared Protos wire-contract family.
        ///
        /// Only canonical families appear here. Alias owners select one of
        /// these bindings in their own component; aliases receive no registry
        /// entry or allocation mechanism in this crate.
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        #[repr(u8)]
        pub enum WireContractFamily {
            $(
                $active_variant,
            )*
            $(
                $retired_variant,
            )*
        }

        impl WireContractFamily {
            /// Every declared family, in canonical declaration order.
            pub const ALL: &'static [Self] = &[
                $(
                    Self::$active_variant,
                )*
                $(
                    Self::$retired_variant,
                )*
            ];

            /// Number of declared canonical families.
            pub const COUNT: usize = Self::ALL.len();

            /// This family's total typed allocation state.
            pub const fn allocation(self) -> WireContractAllocation {
                match self {
                    $(
                        Self::$active_variant => $active_allocation,
                    )*
                    $(
                        Self::$retired_variant => $retired_allocation,
                    )*
                }
            }

            /// Every retained decoder binding, oldest first.
            pub const fn binding_history(self) -> &'static [ContractBinding] {
                self.allocation().binding_history()
            }

            /// Binding new encoders emit, when this family remains active.
            pub const fn current_binding(self) -> Option<ContractBinding> {
                self.allocation().current_binding()
            }

            /// Whether this family retains an explicit decoder for `binding`.
            pub fn supports_binding(self, binding: ContractBinding) -> bool {
                self.binding_history().contains(&binding)
            }

            const fn ordinal(self) -> u8 {
                self as u8
            }
        }

        $(
            wire_contract_registry!(@entry active, $active_variant, $active_contract_id,
                $active_wire_revision, $active_binding, $active_history, $active_allocation,
                $active_id, [$($active_revision),+]);
        )*
        $(
            wire_contract_registry!(@entry retired, $retired_variant, $retired_contract_id,
                $retired_wire_revision, $retired_binding, $retired_history, $retired_allocation,
                $retired_id, [$($retired_revision),+]);
        )*

        /// Every canonical allocation, in canonical declaration order.
        pub const WIRE_CONTRACT_ALLOCATIONS: &[WireContractAllocation] = &[
            $(
                $active_allocation,
            )*
            $(
                $retired_allocation,
            )*
        ];

        /// Canonical active allocations, derived from the registry declaration.
        pub const ACTIVE_WIRE_CONTRACT_ALLOCATIONS: &[ActiveWireContractAllocation] = &[
            $(
                ActiveWireContractAllocation::new(
                    WireContractFamily::$active_variant,
                    $active_binding,
                    $active_history,
                ),
            )*
        ];

        /// Permanent retired allocations, derived from the registry declaration.
        pub const RETIRED_WIRE_CONTRACT_ALLOCATIONS: &[RetiredWireContractAllocation] = &[
            $(
                RetiredWireContractAllocation::new(
                    WireContractFamily::$retired_variant,
                    $retired_contract_id,
                    $retired_history,
                ),
            )*
        ];
    };
    (@entry active, $family:ident, $contract_id:ident, $wire_revision:ident,
        $binding:ident, $history:ident, $allocation:ident, $id:literal,
        [$($revision:literal),+]) => {
        /// Stable numeric identity of this canonical family.
        pub const $contract_id: ContractId = contract_id($id);
        /// Current encoder revision of this canonical family.
        pub const $wire_revision: WireRevision =
            wire_revision(wire_contract_registry!(@last $($revision),+));
        /// Current binding emitted by this canonical family.
        pub const $binding: ContractBinding = ContractBinding::new($contract_id, $wire_revision);
        const $history: &[ContractBinding] = &[
            $(
                ContractBinding::new($contract_id, wire_revision($revision)),
            )+
        ];
        const $allocation: WireContractAllocation =
            WireContractAllocation::Active(ActiveWireContractAllocation::new(
                WireContractFamily::$family,
                $binding,
                $history,
            ));
    };
    (@entry retired, $family:ident, $contract_id:ident, $wire_revision:ident,
        $binding:ident, $history:ident, $allocation:ident, $id:literal,
        [$($revision:literal),+]) => {
        /// Stable numeric identity of this canonical family.
        pub const $contract_id: ContractId = contract_id($id);
        /// Last retained decoder revision of this retired family.
        pub const $wire_revision: WireRevision =
            wire_revision(wire_contract_registry!(@last $($revision),+));
        /// Last retained decoder binding of this retired family.
        pub const $binding: ContractBinding = ContractBinding::new($contract_id, $wire_revision);
        const $history: &[ContractBinding] = &[
            $(
                ContractBinding::new($contract_id, wire_revision($revision)),
            )+
        ];
        const $allocation: WireContractAllocation =
            WireContractAllocation::Retired(RetiredWireContractAllocation::new(
                WireContractFamily::$family,
                $contract_id,
                $history,
            ));
    };
    (@last $last:literal) => { $last };
    (@last $first:literal, $($rest:literal),+) => {
        wire_contract_registry!(@last $($rest),+)
    };
}

wire_contract_registry! {
    active {
        SignalSpirit {
            constants: (
                SIGNAL_SPIRIT_CONTRACT_ID,
                SIGNAL_SPIRIT_WIRE_REVISION,
                SIGNAL_SPIRIT_BINDING
            ),
            history: SIGNAL_SPIRIT_BINDINGS,
            allocation: SIGNAL_SPIRIT_ALLOCATION,
            id: 1,
            revisions: [1],
        }
        MetaSignalSpirit {
            constants: (
                META_SIGNAL_SPIRIT_CONTRACT_ID,
                META_SIGNAL_SPIRIT_WIRE_REVISION,
                META_SIGNAL_SPIRIT_BINDING
            ),
            history: META_SIGNAL_SPIRIT_BINDINGS,
            allocation: META_SIGNAL_SPIRIT_ALLOCATION,
            id: 2,
            revisions: [1],
        }
        SignalSpiritJudge {
            constants: (
                SIGNAL_SPIRIT_JUDGE_CONTRACT_ID,
                SIGNAL_SPIRIT_JUDGE_WIRE_REVISION,
                SIGNAL_SPIRIT_JUDGE_BINDING
            ),
            history: SIGNAL_SPIRIT_JUDGE_BINDINGS,
            allocation: SIGNAL_SPIRIT_JUDGE_ALLOCATION,
            id: 3,
            revisions: [1],
        }
        SignalSemaTranslator {
            constants: (
                SIGNAL_SEMA_TRANSLATOR_CONTRACT_ID,
                SIGNAL_SEMA_TRANSLATOR_WIRE_REVISION,
                SIGNAL_SEMA_TRANSLATOR_BINDING
            ),
            history: SIGNAL_SEMA_TRANSLATOR_BINDINGS,
            allocation: SIGNAL_SEMA_TRANSLATOR_ALLOCATION,
            id: 4,
            revisions: [1],
        }
        SignalLojix {
            constants: (
                SIGNAL_LOJIX_CONTRACT_ID,
                SIGNAL_LOJIX_WIRE_REVISION,
                SIGNAL_LOJIX_BINDING
            ),
            history: SIGNAL_LOJIX_BINDINGS,
            allocation: SIGNAL_LOJIX_ALLOCATION,
            id: 5,
            revisions: [1],
        }
        MetaSignalLojix {
            constants: (
                META_SIGNAL_LOJIX_CONTRACT_ID,
                META_SIGNAL_LOJIX_WIRE_REVISION,
                META_SIGNAL_LOJIX_BINDING
            ),
            history: META_SIGNAL_LOJIX_BINDINGS,
            allocation: META_SIGNAL_LOJIX_ALLOCATION,
            id: 6,
            revisions: [1],
        }
    }
    retired {}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RegistryValidationError {
    EmptyHistory,
    BindingContractMismatch,
    CurrentBindingMismatch,
    RevisionsNotStrictlyIncreasing,
    FamilyRepeated,
    ContractIdRepeated,
}

const fn same_binding(left: ContractBinding, right: ContractBinding) -> bool {
    left.contract().value() == right.contract().value()
        && left.revision().value() == right.revision().value()
}

const fn validate_history(
    history: &[ContractBinding],
    contract: ContractId,
    current: Option<ContractBinding>,
) -> Result<(), RegistryValidationError> {
    if history.is_empty() {
        return Err(RegistryValidationError::EmptyHistory);
    }

    let mut index = 0;
    while index < history.len() {
        let binding = history[index];
        if binding.contract().value() != contract.value() {
            return Err(RegistryValidationError::BindingContractMismatch);
        }
        if index > 0 && history[index - 1].revision().value() >= binding.revision().value() {
            return Err(RegistryValidationError::RevisionsNotStrictlyIncreasing);
        }
        index += 1;
    }

    if let Some(current) = current {
        if !same_binding(current, history[history.len() - 1]) {
            return Err(RegistryValidationError::CurrentBindingMismatch);
        }
    }

    Ok(())
}

/// Validate an entire canonical registry without I/O or mutable state.
const fn validate_registry(
    registry: &[WireContractAllocation],
) -> Result<(), RegistryValidationError> {
    let mut index = 0;
    while index < registry.len() {
        let allocation = registry[index];
        let (family, contract, history, current) = match allocation {
            WireContractAllocation::Active(active) => (
                active.family(),
                active.current_binding().contract(),
                active.supported_bindings(),
                Some(active.current_binding()),
            ),
            WireContractAllocation::Retired(retired) => (
                retired.family(),
                retired.contract_id(),
                retired.binding_history(),
                None,
            ),
        };
        match validate_history(history, contract, current) {
            Ok(()) => {}
            Err(error) => return Err(error),
        }

        let mut previous = 0;
        while previous < index {
            let prior = registry[previous];
            if prior.family().ordinal() == family.ordinal() {
                return Err(RegistryValidationError::FamilyRepeated);
            }

            let prior_contract = match prior {
                WireContractAllocation::Active(active) => active.current_binding().contract(),
                WireContractAllocation::Retired(retired) => retired.contract_id(),
            };
            if prior_contract.value() == contract.value() {
                return Err(RegistryValidationError::ContractIdRepeated);
            }
            previous += 1;
        }
        index += 1;
    }
    Ok(())
}

const _: () = match validate_registry(WIRE_CONTRACT_ALLOCATIONS) {
    Ok(()) => (),
    Err(_) => panic!("the canonical wire-contract registry is invalid"),
};

#[cfg(test)]
mod tests {
    use super::*;

    const ACTIVE_HISTORY: &[ContractBinding] =
        &[ContractBinding::new(contract_id(11), wire_revision(1))];
    const RETIRED_HISTORY: &[ContractBinding] =
        &[ContractBinding::new(contract_id(12), wire_revision(1))];
    const LOST_CURRENT_HISTORY: &[ContractBinding] = &[
        ContractBinding::new(contract_id(11), wire_revision(1)),
        ContractBinding::new(contract_id(11), wire_revision(2)),
    ];
    const DUPLICATE_REVISION_HISTORY: &[ContractBinding] = &[
        ContractBinding::new(contract_id(11), wire_revision(1)),
        ContractBinding::new(contract_id(11), wire_revision(1)),
    ];
    const NONMONOTONIC_HISTORY: &[ContractBinding] = &[
        ContractBinding::new(contract_id(11), wire_revision(2)),
        ContractBinding::new(contract_id(11), wire_revision(1)),
    ];
    const SYNTHETIC_VALID: &[WireContractAllocation] = &[
        WireContractAllocation::Active(ActiveWireContractAllocation::new(
            WireContractFamily::SignalSpirit,
            ACTIVE_HISTORY[0],
            ACTIVE_HISTORY,
        )),
        WireContractAllocation::Retired(RetiredWireContractAllocation::new(
            WireContractFamily::MetaSignalSpirit,
            contract_id(12),
            RETIRED_HISTORY,
        )),
    ];

    #[test]
    fn production_registry_is_valid() {
        assert_eq!(validate_registry(WIRE_CONTRACT_ALLOCATIONS), Ok(()));
        assert_eq!(validate_registry(SYNTHETIC_VALID), Ok(()));
    }

    #[test]
    fn validator_rejects_reused_active_and_tombstone_id() {
        let reused = [
            SYNTHETIC_VALID[0],
            WireContractAllocation::Retired(RetiredWireContractAllocation::new(
                WireContractFamily::MetaSignalSpirit,
                contract_id(11),
                ACTIVE_HISTORY,
            )),
        ];
        assert_eq!(
            validate_registry(&reused),
            Err(RegistryValidationError::ContractIdRepeated)
        );
    }

    #[test]
    fn validator_rejects_duplicate_tombstones() {
        let duplicate_tombstones = [
            WireContractAllocation::Retired(RetiredWireContractAllocation::new(
                WireContractFamily::SignalSpirit,
                contract_id(12),
                RETIRED_HISTORY,
            )),
            SYNTHETIC_VALID[1],
        ];
        assert_eq!(
            validate_registry(&duplicate_tombstones),
            Err(RegistryValidationError::ContractIdRepeated)
        );
    }

    #[test]
    fn validator_rejects_lost_current_history_and_reactivation() {
        let lost_current_history = [WireContractAllocation::Active(
            ActiveWireContractAllocation::new(
                WireContractFamily::SignalSpirit,
                ContractBinding::new(contract_id(11), wire_revision(1)),
                LOST_CURRENT_HISTORY,
            ),
        )];
        assert_eq!(
            validate_registry(&lost_current_history),
            Err(RegistryValidationError::CurrentBindingMismatch)
        );

        let reactivated = [
            SYNTHETIC_VALID[0],
            WireContractAllocation::Retired(RetiredWireContractAllocation::new(
                WireContractFamily::SignalSpirit,
                contract_id(12),
                RETIRED_HISTORY,
            )),
        ];
        assert_eq!(
            validate_registry(&reactivated),
            Err(RegistryValidationError::FamilyRepeated)
        );
    }

    #[test]
    fn validator_rejects_duplicate_and_nonmonotonic_revisions() {
        let duplicate_revision = [WireContractAllocation::Retired(
            RetiredWireContractAllocation::new(
                WireContractFamily::SignalSpirit,
                contract_id(11),
                DUPLICATE_REVISION_HISTORY,
            ),
        )];
        assert_eq!(
            validate_registry(&duplicate_revision),
            Err(RegistryValidationError::RevisionsNotStrictlyIncreasing)
        );

        let nonmonotonic = [WireContractAllocation::Retired(
            RetiredWireContractAllocation::new(
                WireContractFamily::SignalSpirit,
                contract_id(11),
                NONMONOTONIC_HISTORY,
            ),
        )];
        assert_eq!(
            validate_registry(&nonmonotonic),
            Err(RegistryValidationError::RevisionsNotStrictlyIncreasing)
        );
    }

    #[test]
    fn zero_values_are_unrepresentable() {
        assert!(ContractId::try_new(0).is_err());
        assert!(WireRevision::try_new(0).is_err());
    }
}
