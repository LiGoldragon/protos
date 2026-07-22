//! The conservative disjointness checker: accepts a pair of decode forms only when
//! it can PROVE no block matches both; unprovable overlap is a hard error.

use std::collections::BTreeMap;

use name_table::{IdentifierNamespace, Name, NameTable};
use raw_discovery::{Delimiter, Recognizer};
use structural_codec::fixture::{FIELD, FixtureBuilder};
use structural_codec::{
    AddressedStructuralTable, AtomForm, CaseExpectation, ConstructorCodec, EncodedConstructorId,
    EncodedLayoutIdentity, PositionalSignature, RawProfileIdentity, ScopedEncodedTypeId,
    StructuralEntry, StructuralEvaluator, StructuralForm, StructuralRevision, StructuralValue,
    TableError, TableIdentityPayload,
};

fn entry_with_forms(forms: Vec<StructuralForm>) -> StructuralEntry {
    let core_type = ScopedEncodedTypeId::fixture(100);
    let constructors = forms
        .into_iter()
        .enumerate()
        .map(|(index, form)| {
            ConstructorCodec::new(
                EncodedConstructorId::new(core_type, index as u32),
                vec![form.clone()],
                form,
                PositionalSignature::default(),
            )
        })
        .collect();
    StructuralEntry::new(core_type, constructors)
}

fn sealed_table(entry: StructuralEntry) -> Result<AddressedStructuralTable, TableError> {
    sealed_entries(BTreeMap::from([(entry.core_type, entry)]))
}

fn sealed_entries(
    entries: BTreeMap<ScopedEncodedTypeId, StructuralEntry>,
) -> Result<AddressedStructuralTable, TableError> {
    AddressedStructuralTable::seal(
        StructuralRevision::new(2),
        TableIdentityPayload {
            core_universe: structural_codec::FIXTURE_UNIVERSE,
            core_layout_identity: EncodedLayoutIdentity([0; 32]),
            raw_profile_identity: RawProfileIdentity([1; 32]),
            leaf_codec_contracts: Vec::new(),
            entries,
        },
    )
}

fn chosen_constructor(value: StructuralValue) -> u32 {
    let StructuralValue::Chosen { constructor, .. } = value else {
        panic!("expected a constructor-tagged value");
    };
    constructor
}

/// The `Field` alternatives (bare `Type` atom versus `name.Type` application) are
/// provably disjoint — different block kinds.
#[test]
fn field_alternatives_are_provably_disjoint() {
    let table = FixtureBuilder::new().build().expect("seal");
    table
        .validate_disjoint()
        .expect("the whole fixture table validates");

    // and specifically the Field entry.
    let field = FixtureBuilder::new()
        .build()
        .expect("seal")
        .entry(FIELD)
        .expect("field entry")
        .clone();
    field.validate_disjoint().expect("field entry validates");
}

/// Two atoms of DIFFERENT concrete case are provably disjoint.
#[test]
fn distinct_atom_cases_are_disjoint() {
    let entry = entry_with_forms(vec![
        StructuralForm::Atom(AtomForm::with_case(CaseExpectation::PascalCase)),
        StructuralForm::Atom(AtomForm::with_case(CaseExpectation::CamelCase)),
    ]);
    entry.validate_disjoint().expect("distinct cases disjoint");
}

/// Two atoms of the SAME case overlap — the checker cannot prove them distinct, so it
/// errors (conservative-safe).
#[test]
fn identical_atom_cases_are_rejected() {
    let entry = entry_with_forms(vec![
        StructuralForm::Atom(AtomForm::with_case(CaseExpectation::PascalCase)),
        StructuralForm::Atom(AtomForm::with_case(CaseExpectation::PascalCase)),
    ]);
    assert!(
        entry.validate_disjoint().is_err(),
        "overlapping atom cases must be rejected"
    );
}

/// Delegate forms are opaque — their matchable block kind is unknown — so a pair of
/// them can never be proven disjoint and is rejected.
#[test]
fn delegate_forms_are_conservatively_rejected() {
    let entry = entry_with_forms(vec![
        StructuralForm::Delegate(ScopedEncodedTypeId::fixture(200)),
        StructuralForm::Delegate(ScopedEncodedTypeId::fixture(201)),
    ]);
    assert!(
        entry.validate_disjoint().is_err(),
        "opaque delegate forms must be conservatively rejected"
    );
}

/// An unguarded self-delegate cycle is rejected at seal time as a typed structural
/// failure rather than recursing until the process aborts.
#[test]
fn seal_rejects_self_delegate_cycle_with_typed_failure() {
    let recursive = ScopedEncodedTypeId::fixture(210);
    let delegate = StructuralForm::Delegate(recursive);
    let delimited = StructuralForm::Delimited {
        delimiter: Delimiter::Brace,
        sequence: structural_codec::SequenceForm::zero_or_more(StructuralForm::pascal_atom()),
    };
    let entry = StructuralEntry::new(
        recursive,
        vec![
            ConstructorCodec::new(
                EncodedConstructorId::new(recursive, 0),
                vec![delegate.clone()],
                delegate,
                PositionalSignature::default(),
            ),
            ConstructorCodec::new(
                EncodedConstructorId::new(recursive, 1),
                vec![delimited.clone()],
                delimited,
                PositionalSignature::default(),
            ),
        ],
    );

    let Err(TableError::Disjointness(error)) = sealed_table(entry) else {
        panic!("an unguarded self-delegate cycle must fail sealing");
    };
    assert!(matches!(
        error,
        structural_codec::DisjointnessError::DelegateExpansionCycle {
            core_type,
            first: 0,
            second: 1,
            reentered,
        } if core_type == recursive && reentered == recursive
    ));

    let bytes = rkyv::to_bytes::<rkyv::rancor::Error>(&error).expect("archive typed failure");
    let restored =
        rkyv::from_bytes::<structural_codec::DisjointnessError, rkyv::rancor::Error>(&bytes)
            .expect("restore typed failure");
    assert!(matches!(
        restored,
        structural_codec::DisjointnessError::DelegateExpansionCycle {
            core_type,
            first: 0,
            second: 1,
            reentered,
        } if core_type == recursive && reentered == recursive
    ));
}

/// A mutual unguarded delegate cycle also returns the exact typed seal failure.
#[test]
fn seal_rejects_mutual_delegate_cycle_with_typed_failure() {
    let outer = ScopedEncodedTypeId::fixture(220);
    let left = ScopedEncodedTypeId::fixture(221);
    let right = ScopedEncodedTypeId::fixture(222);
    let single_delegate = |core_type, target| {
        let delegate = StructuralForm::Delegate(target);
        StructuralEntry::new(
            core_type,
            vec![ConstructorCodec::new(
                EncodedConstructorId::new(core_type, 0),
                vec![delegate.clone()],
                delegate,
                PositionalSignature::default(),
            )],
        )
    };
    let outer_delegate = StructuralForm::Delegate(left);
    let outer_delimited = StructuralForm::Delimited {
        delimiter: Delimiter::Brace,
        sequence: structural_codec::SequenceForm::zero_or_more(StructuralForm::pascal_atom()),
    };
    let outer_entry = StructuralEntry::new(
        outer,
        vec![
            ConstructorCodec::new(
                EncodedConstructorId::new(outer, 0),
                vec![outer_delegate.clone()],
                outer_delegate,
                PositionalSignature::default(),
            ),
            ConstructorCodec::new(
                EncodedConstructorId::new(outer, 1),
                vec![outer_delimited.clone()],
                outer_delimited,
                PositionalSignature::default(),
            ),
        ],
    );

    assert!(matches!(
        sealed_entries(BTreeMap::from([
            (outer, outer_entry),
            (left, single_delegate(left, right)),
            (right, single_delegate(right, left)),
        ])),
        Err(TableError::Disjointness(
            structural_codec::DisjointnessError::DelegateExpansionCycle {
                core_type,
                first: 0,
                second: 1,
                reentered,
            }
        )) if core_type == outer && reentered == left
    ));
}

/// Guarded recursion remains valid: the distinct application heads prove separation
/// before either recursive payload needs expansion.
#[test]
fn seal_preserves_guarded_recursive_alternatives() {
    let recursive = ScopedEncodedTypeId::fixture(230);
    let pascal = StructuralForm::application(
        StructuralForm::pascal_atom(),
        StructuralForm::Delegate(recursive),
    );
    let camel = StructuralForm::application(
        StructuralForm::camel_atom(),
        StructuralForm::Delegate(recursive),
    );
    let entry = StructuralEntry::new(
        recursive,
        vec![
            ConstructorCodec::new(
                EncodedConstructorId::new(recursive, 0),
                vec![pascal.clone()],
                pascal,
                PositionalSignature::default(),
            ),
            ConstructorCodec::new(
                EncodedConstructorId::new(recursive, 1),
                vec![camel.clone()],
                camel,
                PositionalSignature::default(),
            ),
        ],
    );

    sealed_table(entry).expect("guarded recursion seals through its distinct heads");
}

/// An atom versus an application of a distinguishing head is disjoint by block kind.
#[test]
fn atom_versus_application_is_disjoint() {
    let entry = entry_with_forms(vec![
        StructuralForm::pascal_atom(),
        StructuralForm::application(StructuralForm::camel_atom(), StructuralForm::pascal_atom()),
    ]);
    entry
        .validate_disjoint()
        .expect("atom and application are disjoint");
}

/// Sealing is the mandatory proof boundary: no addressed table can contain an
/// unprovable overlap.
#[test]
fn seal_rejects_unprovable_decode_forms() {
    let error = sealed_table(entry_with_forms(vec![
        StructuralForm::pascal_atom(),
        StructuralForm::pascal_atom(),
    ]))
    .expect_err("two PascalCase alternatives overlap");
    assert!(
        matches!(error, TableError::Disjointness(_)),
        "seal reports the typed disjointness failure"
    );
}

/// Literal and unconstrained name-atom alternatives overlap. Sealing fails rather
/// than trying to preserve a keyword category with lexical exclusions.
#[test]
fn literal_and_unconstrained_name_atom_are_rejected() {
    let mut lexicon = NameTable::new(IdentifierNamespace::Fixture);
    let integer = lexicon
        .intern(Name::new("Integer"))
        .expect("intern Integer");
    let entry = entry_with_forms(vec![
        StructuralForm::Literal(integer),
        StructuralForm::Atom(AtomForm {
            case: None,
            sigil: None,
        }),
    ]);

    assert!(
        sealed_table(entry).is_err(),
        "sealing rejects literal/name-atom overlap"
    );
}

/// Construction order cannot change an already-proven disjoint decode result.
#[test]
fn disjoint_constructor_order_does_not_change_the_chosen_identifier() {
    let core_type = ScopedEncodedTypeId::fixture(101);
    let atom = StructuralForm::pascal_atom();
    let application =
        StructuralForm::application(StructuralForm::pascal_atom(), StructuralForm::pascal_atom());
    let table_with_order = |forms: Vec<(u32, StructuralForm)>| {
        let constructors = forms
            .into_iter()
            .map(|(constructor, form)| {
                ConstructorCodec::new(
                    EncodedConstructorId::new(core_type, constructor),
                    vec![form.clone()],
                    form,
                    PositionalSignature::default(),
                )
            })
            .collect();
        sealed_table(StructuralEntry::new(core_type, constructors)).expect("seal")
    };
    let first = table_with_order(vec![(7, atom.clone()), (9, application.clone())]);
    let second = table_with_order(vec![(9, application), (7, atom)]);
    let block = Recognizer::standard()
        .recognize("Integer")
        .expect("recognize")
        .root_object_at(0)
        .expect("root")
        .clone();

    for table in [&first, &second] {
        let mut names = NameTable::new(IdentifierNamespace::Fixture);
        let value = StructuralEvaluator::new(table)
            .decode(core_type, &block, &mut names)
            .expect("decode bare name");
        assert_eq!(chosen_constructor(value), 7);
    }
}

/// The table-level seal expands delegates to prove a wrapper form is disjoint from a
/// delimited alternative, while decoding retains the delegate's constructor wrapper.
#[test]
fn seal_proves_disjointness_through_a_delegate() {
    let reference = ScopedEncodedTypeId::fixture(200);
    let declaration = ScopedEncodedTypeId::fixture(201);
    let reference_entry = StructuralEntry::new(
        reference,
        vec![ConstructorCodec::new(
            EncodedConstructorId::new(reference, 7),
            vec![StructuralForm::pascal_atom()],
            StructuralForm::pascal_atom(),
            PositionalSignature::default(),
        )],
    );
    let newtype = StructuralForm::application(
        StructuralForm::pascal_atom(),
        StructuralForm::Delegate(reference),
    );
    let structure = StructuralForm::application(
        StructuralForm::pascal_atom(),
        StructuralForm::Delimited {
            delimiter: Delimiter::Brace,
            sequence: structural_codec::SequenceForm::zero_or_more(StructuralForm::pascal_atom()),
        },
    );
    let declaration_entry = StructuralEntry::new(
        declaration,
        vec![
            ConstructorCodec::new(
                EncodedConstructorId::new(declaration, 0),
                vec![newtype.clone()],
                newtype,
                PositionalSignature::default(),
            ),
            ConstructorCodec::new(
                EncodedConstructorId::new(declaration, 1),
                vec![structure.clone()],
                structure,
                PositionalSignature::default(),
            ),
        ],
    );
    let table = AddressedStructuralTable::seal(
        StructuralRevision::new(2),
        TableIdentityPayload {
            core_universe: structural_codec::FIXTURE_UNIVERSE,
            core_layout_identity: EncodedLayoutIdentity([0; 32]),
            raw_profile_identity: RawProfileIdentity([1; 32]),
            leaf_codec_contracts: Vec::new(),
            entries: BTreeMap::from([
                (reference, reference_entry),
                (declaration, declaration_entry),
            ]),
        },
    )
    .expect("the delegate expands to a Pascal atom, disjoint from a brace");

    let block = Recognizer::standard()
        .recognize("Record.Target")
        .expect("recognize")
        .root_object_at(0)
        .expect("root")
        .clone();
    let mut names = NameTable::new(IdentifierNamespace::Fixture);
    let value = StructuralEvaluator::new(&table)
        .decode(declaration, &block, &mut names)
        .expect("decode newtype form");
    let StructuralValue::Chosen { payload, .. } = value else {
        panic!("declaration is constructor-tagged");
    };
    let StructuralValue::Application(_, body) = payload.as_ref() else {
        panic!("declaration is an application");
    };
    assert!(
        matches!(body.as_ref(), StructuralValue::Delegated(inner) if matches!(inner.as_ref(), StructuralValue::Chosen { constructor: 7, .. })),
        "the evaluator retains the delegated reference constructor"
    );
}
