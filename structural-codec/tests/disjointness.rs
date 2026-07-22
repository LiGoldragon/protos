//! The conservative disjointness checker: accepts a pair of decode forms only when
//! it can PROVE no block matches both; unprovable overlap is a hard error.

use std::collections::BTreeMap;

use name_table::{IdentifierNamespace, Name, NameTable};
use raw_discovery::Recognizer;
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
    let mut entries = BTreeMap::new();
    entries.insert(entry.core_type, entry);
    AddressedStructuralTable::seal(
        StructuralRevision::new(2),
        TableIdentityPayload {
            core_universe: structural_codec::FIXTURE_UNIVERSE,
            core_layout_identity: EncodedLayoutIdentity([0; 32]),
            raw_profile_identity: RawProfileIdentity([1; 32]),
            committed_lexicon: b"disjointness-test".to_vec(),
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

/// A committed literal and a name atom that excludes it are provably disjoint. The
/// same decoded constructor is selected after the codecs are authored in reverse
/// order, because constructor identifiers—not vector positions—carry the result.
#[test]
fn literal_and_excluded_name_atom_are_order_independent() {
    let mut lexicon = NameTable::new(IdentifierNamespace::Fixture);
    let integer = lexicon
        .intern(Name::new("Integer"))
        .expect("intern Integer");
    let declared = StructuralForm::Atom(AtomForm::excluding_literals(
        CaseExpectation::PascalCase,
        vec![integer],
    ));
    let literal = StructuralForm::Literal(integer);
    let core_type = ScopedEncodedTypeId::fixture(101);

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

    let literal_first = table_with_order(vec![(0, literal.clone()), (1, declared.clone())]);
    let declared_first = table_with_order(vec![(1, declared), (0, literal)]);
    let block = Recognizer::standard()
        .recognize("Integer")
        .expect("recognize")
        .root_object_at(0)
        .expect("root")
        .clone();

    for table in [&literal_first, &declared_first] {
        let evaluator = StructuralEvaluator::with_lexicon(table, &lexicon);
        let mut names = NameTable::new(IdentifierNamespace::Fixture);
        let value = evaluator
            .decode(core_type, &block, &mut names)
            .expect("decode builtin literal");
        assert_eq!(chosen_constructor(value), 0, "Integer remains the literal");
    }
}
