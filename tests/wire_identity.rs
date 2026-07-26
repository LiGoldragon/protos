use std::collections::BTreeSet;

use protos::{
    ACTIVE_WIRE_CONTRACT_ALLOCATIONS, META_SIGNAL_SPIRIT_BINDING, META_SIGNAL_SPIRIT_CONTRACT_ID,
    META_SIGNAL_SPIRIT_WIRE_REVISION, RETIRED_WIRE_CONTRACT_ALLOCATIONS, SIGNAL_SPIRIT_BINDING,
    SIGNAL_SPIRIT_CONTRACT_ID, SIGNAL_SPIRIT_JUDGE_BINDING, SIGNAL_SPIRIT_JUDGE_CONTRACT_ID,
    SIGNAL_SPIRIT_JUDGE_WIRE_REVISION, SIGNAL_SPIRIT_WIRE_REVISION, WIRE_CONTRACT_ALLOCATIONS,
    WireContractAllocation, WireContractFamily,
};
use signal_frame::{
    BoundExchangeFrame, ContractBinding, ContractId, ExchangeFrameBody, HandshakeRequest, RootCode,
    VariantCode, WireContract, WireRevision, WireRoute,
};

struct SignalSpirit;

impl WireContract for SignalSpirit {
    const BINDING: ContractBinding = SIGNAL_SPIRIT_BINDING;
}

struct MetaSignalSpirit;

impl WireContract for MetaSignalSpirit {
    const BINDING: ContractBinding = META_SIGNAL_SPIRIT_BINDING;
}

struct SignalSpiritJudge;

impl WireContract for SignalSpiritJudge {
    const BINDING: ContractBinding = SIGNAL_SPIRIT_JUDGE_BINDING;
}

fn header<Contract: WireContract>() -> signal_frame::ShortHeader {
    BoundExchangeFrame::<Contract, (), ()>::new(
        WireRoute::new(RootCode::new(7), VariantCode::new(11)),
        ExchangeFrameBody::HandshakeRequest(HandshakeRequest::current()),
    )
    .short_header()
}

#[test]
fn established_and_judge_allocations_and_encoded_header_bits_are_exact() {
    assert_eq!(SIGNAL_SPIRIT_CONTRACT_ID.value(), 1);
    assert_eq!(SIGNAL_SPIRIT_WIRE_REVISION.value(), 1);
    assert_eq!(META_SIGNAL_SPIRIT_CONTRACT_ID.value(), 2);
    assert_eq!(META_SIGNAL_SPIRIT_WIRE_REVISION.value(), 1);
    assert_eq!(SIGNAL_SPIRIT_JUDGE_CONTRACT_ID.value(), 3);
    assert_eq!(SIGNAL_SPIRIT_JUDGE_WIRE_REVISION.value(), 1);
    assert!(ContractId::try_new(0).is_err());
    assert!(WireRevision::try_new(0).is_err());

    assert_eq!(
        header::<SignalSpirit>().to_le_bytes(),
        [1, 0, 0, 0, 1, 0, 11, 7]
    );
    assert_eq!(
        header::<MetaSignalSpirit>().to_le_bytes(),
        [2, 0, 0, 0, 1, 0, 11, 7]
    );
    assert_eq!(
        header::<SignalSpiritJudge>().to_le_bytes(),
        [3, 0, 0, 0, 1, 0, 11, 7]
    );
}

#[test]
fn declared_family_table_is_exhaustive_and_total() {
    assert_eq!(
        WireContractFamily::ALL,
        &[
            WireContractFamily::SignalSpirit,
            WireContractFamily::MetaSignalSpirit,
            WireContractFamily::SignalSpiritJudge,
        ]
    );
    assert_eq!(ACTIVE_WIRE_CONTRACT_ALLOCATIONS.len(), 3);
    assert!(RETIRED_WIRE_CONTRACT_ALLOCATIONS.is_empty());
    assert_eq!(
        ACTIVE_WIRE_CONTRACT_ALLOCATIONS.len() + RETIRED_WIRE_CONTRACT_ALLOCATIONS.len(),
        WireContractFamily::ALL.len()
    );
    assert_eq!(WIRE_CONTRACT_ALLOCATIONS.len(), WireContractFamily::COUNT);

    for family in WireContractFamily::ALL {
        match family.allocation() {
            WireContractAllocation::Active(allocation) => {
                assert_eq!(allocation.family(), *family);
                assert_eq!(Some(allocation.current_binding()), family.current_binding());
                assert_eq!(allocation.supported_bindings(), family.binding_history());
                assert_eq!(
                    allocation.supported_bindings().last(),
                    Some(&allocation.current_binding())
                );
                assert!(family.supports_binding(allocation.current_binding()));
            }
            WireContractAllocation::Retired(allocation) => {
                assert_eq!(allocation.family(), *family);
                assert_eq!(allocation.binding_history(), family.binding_history());
                assert!(family.current_binding().is_none());
            }
        }
    }
}

#[test]
fn all_ids_are_nonzero_unique_and_never_active_after_retirement() {
    let mut globally_reserved = BTreeSet::new();
    for allocation in WIRE_CONTRACT_ALLOCATIONS {
        let (contract_id, history) = match allocation {
            WireContractAllocation::Active(active) => (
                active.current_binding().contract(),
                active.supported_bindings(),
            ),
            WireContractAllocation::Retired(retired) => {
                (retired.contract_id(), retired.binding_history())
            }
        };
        assert_ne!(contract_id.value(), 0);
        assert!(globally_reserved.insert(contract_id.value()));
        assert!(!history.is_empty());
        assert!(
            history
                .iter()
                .all(|binding| binding.contract() == contract_id)
        );
    }
}

#[test]
fn revision_history_is_monotonic_and_retains_each_explicit_decoder() {
    for allocation in WIRE_CONTRACT_ALLOCATIONS {
        let history = allocation.binding_history();
        assert!(!history.is_empty());
        for revisions in history.windows(2) {
            assert!(revisions[0].revision() < revisions[1].revision());
        }
        if let Some(current) = allocation.current_binding() {
            assert_eq!(Some(&current), history.last());
        }
    }
}

#[test]
fn all_three_families_stay_distinct_for_the_same_local_route() {
    let ordinary = header::<SignalSpirit>();
    let meta = header::<MetaSignalSpirit>();
    let judge = header::<SignalSpiritJudge>();
    assert_eq!(ordinary.route(), meta.route());
    assert_eq!(ordinary.route(), judge.route());
    assert_ne!(ordinary.binding(), meta.binding());
    assert_ne!(ordinary.binding(), judge.binding());
    assert_ne!(meta.binding(), judge.binding());
    assert_ne!(ordinary.to_le_bytes(), meta.to_le_bytes());
    assert_ne!(ordinary.to_le_bytes(), judge.to_le_bytes());
    assert_ne!(meta.to_le_bytes(), judge.to_le_bytes());
}
