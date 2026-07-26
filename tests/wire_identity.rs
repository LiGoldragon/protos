use std::collections::BTreeSet;

use protos::{
    ACTIVE_WIRE_CONTRACT_ALLOCATIONS, META_SIGNAL_SPIRIT_BINDING, META_SIGNAL_SPIRIT_CONTRACT_ID,
    META_SIGNAL_SPIRIT_WIRE_REVISION, RETIRED_WIRE_CONTRACT_ALLOCATIONS, SIGNAL_SPIRIT_BINDING,
    SIGNAL_SPIRIT_CONTRACT_ID, SIGNAL_SPIRIT_WIRE_REVISION, WireContractFamily,
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

struct SignalSpiritPilotMirror;

impl WireContract for SignalSpiritPilotMirror {
    const BINDING: ContractBinding = SIGNAL_SPIRIT_BINDING;
}

struct MetaSignalSpiritMigrationMirror;

impl WireContract for MetaSignalSpiritMigrationMirror {
    const BINDING: ContractBinding = META_SIGNAL_SPIRIT_BINDING;
}

fn header<Contract: WireContract>() -> signal_frame::ShortHeader {
    BoundExchangeFrame::<Contract, (), ()>::new(
        WireRoute::new(RootCode::new(7), VariantCode::new(11)),
        ExchangeFrameBody::HandshakeRequest(HandshakeRequest::current()),
    )
    .short_header()
}

#[test]
fn initial_allocations_and_encoded_header_bits_are_exact() {
    assert_eq!(SIGNAL_SPIRIT_CONTRACT_ID.value(), 1);
    assert_eq!(SIGNAL_SPIRIT_WIRE_REVISION.value(), 1);
    assert_eq!(META_SIGNAL_SPIRIT_CONTRACT_ID.value(), 2);
    assert_eq!(META_SIGNAL_SPIRIT_WIRE_REVISION.value(), 1);
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
}

#[test]
fn declared_family_table_is_exhaustive_and_single_current() {
    assert_eq!(
        ACTIVE_WIRE_CONTRACT_ALLOCATIONS.len() + RETIRED_WIRE_CONTRACT_ALLOCATIONS.len(),
        WireContractFamily::ALL.len()
    );

    for family in WireContractFamily::ALL {
        let active: Vec<_> = ACTIVE_WIRE_CONTRACT_ALLOCATIONS
            .iter()
            .filter(|allocation| allocation.family() == family)
            .collect();
        let retired: Vec<_> = RETIRED_WIRE_CONTRACT_ALLOCATIONS
            .iter()
            .filter(|allocation| allocation.family() == family)
            .collect();
        assert_eq!(active.len() + retired.len(), 1);

        if let Some(allocation) = active.first() {
            assert_eq!(Some(allocation.current_binding()), family.current_binding());
            assert_eq!(
                Some(allocation.supported_bindings()),
                family.supported_bindings()
            );
            assert_eq!(
                allocation.supported_bindings().last(),
                Some(&allocation.current_binding())
            );
            assert!(family.supports_binding(allocation.current_binding()));
        } else {
            assert!(family.active_allocation().is_none());
            assert!(family.current_binding().is_none());
            assert!(family.supported_bindings().is_none());
        }
    }
}

#[test]
fn all_ids_are_nonzero_unique_and_never_active_after_retirement() {
    let mut globally_reserved = BTreeSet::new();
    for allocation in ACTIVE_WIRE_CONTRACT_ALLOCATIONS {
        let binding = allocation.current_binding();
        assert_ne!(binding.contract().value(), 0);
        assert_ne!(binding.revision().value(), 0);
        assert!(globally_reserved.insert(binding.contract().value()));
    }
    for tombstone in RETIRED_WIRE_CONTRACT_ALLOCATIONS {
        assert_ne!(tombstone.contract_id().value(), 0);
        assert!(globally_reserved.insert(tombstone.contract_id().value()));
        assert!(!tombstone.binding_history().is_empty());
        assert!(
            tombstone
                .binding_history()
                .iter()
                .all(|binding| binding.contract() == tombstone.contract_id())
        );
        assert!(
            ACTIVE_WIRE_CONTRACT_ALLOCATIONS
                .iter()
                .all(|active| active.family() != tombstone.family())
        );
    }
}

#[test]
fn revision_history_is_monotonic_and_retains_each_explicit_decoder() {
    for allocation in ACTIVE_WIRE_CONTRACT_ALLOCATIONS {
        let bindings = allocation.supported_bindings();
        assert!(!bindings.is_empty());
        for binding in bindings {
            assert_eq!(binding.contract(), allocation.current_binding().contract());
        }
        for revisions in bindings.windows(2) {
            assert!(revisions[0].revision() < revisions[1].revision());
        }
    }
    for tombstone in RETIRED_WIRE_CONTRACT_ALLOCATIONS {
        for revisions in tombstone.binding_history().windows(2) {
            assert!(revisions[0].revision() < revisions[1].revision());
        }
    }
}

#[test]
fn ordinary_and_owner_only_families_stay_distinct_for_the_same_local_route() {
    let ordinary = header::<SignalSpirit>();
    let meta = header::<MetaSignalSpirit>();
    assert_eq!(ordinary.route(), meta.route());
    assert_ne!(ordinary.binding(), meta.binding());
    assert_ne!(ordinary.to_le_bytes(), meta.to_le_bytes());
}

#[test]
fn pilot_and_migration_aliases_reuse_the_canonical_family_allocation() {
    assert_eq!(
        header::<SignalSpiritPilotMirror>().binding(),
        header::<SignalSpirit>().binding()
    );
    assert_eq!(
        header::<MetaSignalSpiritMigrationMirror>().binding(),
        header::<MetaSignalSpirit>().binding()
    );
    assert_eq!(ACTIVE_WIRE_CONTRACT_ALLOCATIONS.len(), 2);
}

#[test]
fn package_boundary_has_only_exact_one_way_git_dependencies() {
    let manifest = include_str!("../Cargo.toml");
    let dependency_sections = manifest
        .split_once("[dependencies]")
        .expect("dependencies section")
        .1
        .split_once("[lints.rust]")
        .expect("lints follow dependency sections")
        .0;
    assert!(!manifest.contains("[workspace]"));
    assert!(!dependency_sections.contains("protos-engine"));
    assert!(!dependency_sections.contains("path ="));
    assert!(
        manifest.contains("signal-frame.git\", rev = \"0786fbe8caf27552afcdd5deb85bc82ec6088337\"")
    );
}
