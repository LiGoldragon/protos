//! # structural-codec-derive-fixtures
//!
//! `core-schema`'s `FixtureFamily`, mirrored entirely through the
//! `#[structural_form]` attribute macro, plus the assembled derived table. This is
//! the proof that the derive generates — collision-free and without touching
//! `nota` — codecs the trusted evaluator proves equal (law 5, exercised in
//! `tests/conformance.rs`). The downstream cross-check that these derived entries
//! equal `core-schema`'s hand-authored entries lives in `core-schema`, where the
//! machinery resolves at a single pin.
//!
//! Every type below is generated from ONE structural-form authority: the attribute
//! yields the typed capture, the authoritative `StructuralEntry`
//! (`T::structural_entry()`), and the optimized
//! `structural_codec::conformance::GeneratedCodec` implementation.
//!
//! The `id`s match `core-schema`'s fixture universe exactly, so each derived entry
//! is compared, per type, INTO the authored table (`tests/entries.rs`).

use std::collections::BTreeMap;

use structural_codec::ids::{FIXTURE_UNIVERSE, ScopedEncodedTypeId};
use structural_codec::table::{
    AddressedStructuralTable, EncodedLayoutIdentity, RawProfileIdentity, TableIdentityPayload,
};
use structural_codec::{StructuralEntry, TableError};
use structural_codec_derive::structural_form;

// ===== the scalar leaf primitives =====

/// The `Integer` scalar leaf: flatten-then-parse a signed integer.
#[structural_form(id = 10, leaf(Integer))]
pub struct Integer;

/// The `Float` scalar leaf: the dotted-rejoin path shared with the string leaf
/// (`-122.3` is `Application(-122, 3)` flattened and parsed).
#[structural_form(id = 9, leaf(Float))]
pub struct Float;

/// The `Text` scalar leaf: the string-rejoin terminal of the delegate chain
/// (`alpha.beta.gamma` flattens to the string under the same control path as float).
#[structural_form(id = 33, leaf(Text))]
pub struct Text;

// ===== the Documentation -> Summary -> Text string-rejoin delegate chain =====

/// `Summary`: a transparent newtype delegating to `Text`.
#[structural_form(id = 32, delegate(inner = Text))]
pub struct Summary;

/// `Documentation`: a transparent newtype delegating to `Summary`, so a decode is a
/// two-level delegate chain terminating in the `Text` string rejoin.
#[structural_form(id = 31, delegate(inner = Summary))]
pub struct Documentation;

// ===== the Field meta-type: one constructor, the bare type reference =====

/// The `Field` meta-type: ONE constructor, the bare elided-name `Type`. Field names
/// are illegal in every Protos surface (psyche ruling 2026-07-19), so a field carries
/// nothing but the type standing at its position — the explicit `name.Type` form no
/// longer parses. Exercised by `DatabaseMarker`'s three positional fields.
#[structural_form(id = 23, field_meta)]
pub struct Field;

// ===== the schema declarations =====

/// `CommitSequence`: a newtype DECLARATION `CommitSequence.{ Integer }` wrapping
/// `Integer`.
#[structural_form(id = 1, newtype_declaration(inner = Integer, delimiter = Brace))]
pub struct CommitSequence;

/// `StateDigest`: a newtype DECLARATION over `Integer`.
#[structural_form(id = 2, newtype_declaration(inner = Integer, delimiter = Brace))]
pub struct StateDigest;

/// `DatabaseMarker`: a struct DECLARATION whose three delegated fields are each the
/// bare elided-name form (`CommitSequence`, `StateDigest`, `StateDigest`). The two
/// same-typed `StateDigest` fields are told apart by position alone — field names are
/// illegal, so no explicit `name.Type` form exists to distinguish them.
#[structural_form(
    id = 3,
    struct_declaration(
        field_type = Field,
        delimiter = Brace,
        fields = [CommitSequence, StateDigest, StateDigest]
    )
)]
pub struct DatabaseMarker;

/// The derived family's structural table: every derived type's authoritative
/// `StructuralEntry`, assembled and sealed into the addressed sidecar the evaluator
/// runs. Data-bearing (it carries the collected entries), so the assembly verb
/// lives on the noun that owns the entries.
pub struct DerivedTable {
    entries: BTreeMap<ScopedEncodedTypeId, StructuralEntry>,
}

impl DerivedTable {
    /// Collect every derived type's authoritative entry — the derive's entries are
    /// the ones sealed INTO the table (mission requirement).
    pub fn of_fixture_family() -> Self {
        let entries = [
            Integer::structural_entry(),
            Float::structural_entry(),
            Text::structural_entry(),
            Summary::structural_entry(),
            Documentation::structural_entry(),
            Field::structural_entry(),
            CommitSequence::structural_entry(),
            StateDigest::structural_entry(),
            DatabaseMarker::structural_entry(),
        ]
        .into_iter()
        .map(|entry| (entry.core_type, entry))
        .collect();
        Self { entries }
    }

    /// The collected authoritative entries, keyed by scoped Core-type id.
    pub fn entries(&self) -> &BTreeMap<ScopedEncodedTypeId, StructuralEntry> {
        &self.entries
    }

    /// Seal the derived entries into an addressed structural table. The table
    /// identity is derived from a fixed proof-of-concept payload (the evaluator and
    /// the conformance harness read only the entries, never the identity).
    pub fn seal(&self) -> Result<AddressedStructuralTable, TableError> {
        let payload = TableIdentityPayload {
            core_universe: FIXTURE_UNIVERSE,
            core_layout_identity: EncodedLayoutIdentity([0u8; 32]),
            raw_profile_identity: RawProfileIdentity([1u8; 32]),
            leaf_codec_contracts: Vec::new(),
            entries: self.entries.clone(),
        };
        AddressedStructuralTable::seal(payload)
    }
}
