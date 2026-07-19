//! Law 5, executed: `structural-codec`'s `ConformanceHarness` runs every fixture
//! through BOTH the generated codec and the trusted evaluator (over the derive's own
//! table) and asserts agreement on all four outputs — the Core value, the NameTable
//! delta, the canonical output, and the typed-error decision. Each fixture list mixes
//! valid inputs with malformed ones (wrong delimiter, unknown constructor shape,
//! failed leaf parse) so error agreement is proven alongside value agreement.

use name_table::NameTable;
use raw_discovery::{Block, Recognizer};
use structural_codec::StructuralEvaluator;
use structural_codec::conformance::{ConformanceHarness, GeneratedCodec};
use structural_codec::table::AddressedStructuralTable;
use structural_codec_derive_fixtures::{
    CommitSequence, DatabaseMarker, DerivedTable, Documentation, Field, Float, Integer,
    StateDigest, Summary, Text,
};

fn derived_table() -> AddressedStructuralTable {
    DerivedTable::of_fixture_family()
        .seal()
        .expect("seal derived table")
}

fn blocks(sources: &[&str]) -> Vec<Block> {
    sources
        .iter()
        .map(|source| {
            Recognizer::standard()
                .recognize(source)
                .unwrap_or_else(|error| panic!("recognize {source}: {error}"))
                .root_object_at(0)
                .unwrap_or_else(|| panic!("no root object in {source}"))
                .clone()
        })
        .collect()
}

/// Assert the generated codec `T` agrees with the evaluator on every source — value,
/// NameTable delta, canonical output, and typed error.
fn conforms<T: GeneratedCodec>(table: &AddressedStructuralTable, sources: &[&str]) {
    ConformanceHarness::new(table, T::CORE_TYPE)
        .check::<T>(&blocks(sources))
        .unwrap_or_else(|error| panic!("law 5 for {}: {error}", std::any::type_name::<T>()));
}

#[test]
fn law_five_the_generated_codecs_match_the_evaluator() {
    let table = derived_table();

    // Scalar leaves: valid values plus failed-leaf-parse and non-flattenable errors.
    conforms::<Integer>(&table, &["42", "-7", "0", "notanumber", "1.5", "{ x }"]);
    conforms::<Float>(&table, &["-122.3", "3.14", "0", "notafloat", "{ x }"]);
    conforms::<Text>(&table, &["hello", "alpha.beta.gamma", "Word", "{ x }"]);

    // The Documentation -> Summary -> Text string-rejoin delegate chain.
    conforms::<Summary>(&table, &["hello", "alpha.beta", "{ x }"]);
    conforms::<Documentation>(&table, &["alpha.beta.gamma", "word", "{ x }"]);

    // The Field meta-type: both disjoint constructors, plus shape errors.
    conforms::<Field>(
        &table,
        &[
            "Integer",
            "commitSequence.Integer",
            "secretDigest.StateDigest",
            "123",
            "Foo.Bar",
            "{ x }",
        ],
    );

    // Newtype declarations: valid, wrong delimiter, unknown shape, wrong arity.
    conforms::<CommitSequence>(
        &table,
        &[
            "CommitSequence.{ Integer }",
            "CommitSequence.( Integer )",
            "notADeclaration",
            "CommitSequence.{ Integer Extra }",
        ],
    );
    conforms::<StateDigest>(
        &table,
        &["StateDigest.{ Integer }", "StateDigest.( Integer )"],
    );

    // The struct declaration exercising both Field alternatives, plus errors.
    conforms::<DatabaseMarker>(
        &table,
        &[
            "DatabaseMarker.{ CommitSequence StateDigest secretDigest.StateDigest }",
            "DatabaseMarker.{ CommitSequence }",
            "DatabaseMarker.( CommitSequence StateDigest secretDigest.StateDigest )",
        ],
    );
}

/// Assert a source is rejected by BOTH the generated codec and the evaluator — the
/// typed-error agreement the harness proves, made explicit for the three required
/// malformed categories so the error path is genuinely exercised (both Err), not
/// merely both Ok.
fn rejected_by_both<T: GeneratedCodec>(evaluator: &StructuralEvaluator<'_>, source: &str) {
    let block = &blocks(&[source])[0];

    let mut generated_names = NameTable::new();
    assert!(
        T::decode(block, &mut generated_names).is_err(),
        "generated codec should reject {source}",
    );

    let mut evaluator_names = NameTable::new();
    assert!(
        evaluator
            .decode(T::CORE_TYPE, block, &mut evaluator_names)
            .is_err(),
        "evaluator should reject {source}",
    );
}

#[test]
fn malformed_inputs_are_rejected_by_both_paths() {
    let table = derived_table();
    let evaluator = StructuralEvaluator::new(&table);

    // Failed leaf parse.
    rejected_by_both::<Integer>(&evaluator, "notanumber");
    rejected_by_both::<Float>(&evaluator, "notafloat");
    // Wrong delimiter.
    rejected_by_both::<CommitSequence>(&evaluator, "CommitSequence.( Integer )");
    // Unknown constructor shape.
    rejected_by_both::<CommitSequence>(&evaluator, "notADeclaration");
    rejected_by_both::<Field>(&evaluator, "123");
    // Wrong arity.
    rejected_by_both::<DatabaseMarker>(&evaluator, "DatabaseMarker.{ CommitSequence }");
}
