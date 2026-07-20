# name-table architecture

## Overview

`name-table` is crate L2 of the shared-codec language family. The family's rule
is that dependencies run strictly downward and stringless encoded form never depends on
text: `content-identity <- name-table <- raw-discovery <- structural-codec`. This
crate holds the identifier space every `Encoded*` type indexes into, and depends
only on `content-identity` (for the shared `PortableArchive` rkyv discipline) and
`rkyv`.

## Direction

The psyche's settled rulings this crate embodies:

- All `Encoded*` types are stringless; every identifier is a closed namespace
  enum variant carrying one `u16` local allocation, and all names live here.
- One component NameTable is a composed view: it owns one allocation slice and
  borrows completed source slices without copying or renumbering them. The former
  `extend_from` flat-table model is retired.
- The `NameTable` is excluded from `Encoded` content hashes by construction —
  renaming is a `NameTable`-only edit and never moves `Encoded` identity. Names and
  `Encoded` values never serialize together.
- Capitalization is semantic: capitalized-leading is an object, lowercase-leading
  is a name; the derived-name rule (field name = `snake_case` of type name) lives
  here as methods on `Name` — one home for the walkers duplicated in `schema` and
  `schema-rust`.
- Interning is transactional: a failed decode alternative leaves no allocation
  effect. A real staging surface provides this, not a convention.
- Named views (`Textual*`) are derived from `Encoded` + `NameTable`, never stored.
- A `NameTable` may later be stored as a first-class co-versioned sibling of
  `Encoded` in daemon stores, so it must be cleanly archivable; where it is stored is
  not this crate's business.

Consumption and integration will readapt to the forthcoming release-train flow.
This crate migrates no consumer; schema, nomos, logos, and the rest adopt these
types in later train slices.

## Components and boundaries

- `Identifier` / `IdentifierNamespace` (`src/identifier.rs`) — the closed,
  rkyv-archivable namespace enum and its registry. `Schema(u16)`, `Logos(u16)`,
  `LogosStandard(u16)`, `Nomos(u16)`, and fixture values are distinct without
  arithmetic or a flat integer representation.
- `Name` (`src/name.rs`) — the interned name and the one home of the derived-name
  rule. The two source walkers (`schema`'s `field_name`, `schema-rust`'s
  `screaming`) are the same word-boundary walk under two casings; `DerivedCasing`
  names that difference as data so the loop is written once. `pascal_case` is the
  inverse round-trip partner.
- `NameTable` (`src/table.rs`) — one composed component view with an owned,
  append-only home slice and borrowed read-only source slices. Its canonical
  archive and `NameTableDomain` identity are only its owned slice; consumers
  recompose independently identified borrowed slices after loading. The derived
  lookup accepts exactly one canonical name per identifier.
- `NameTransaction` (`src/transaction.rs`) — the speculative interning overlay.
- `NameResolver` / `NameInterner` (`src/boundary.rs`) — the two codec-boundary
  capabilities, threaded down a codec call tree, never held by a node.
- `TextualProjection` (`src/projection.rs`) — the derive-a-named-view surface.
- `NameTableError` (`src/error.rs`) — the typed crate-boundary error (thiserror).

## Names never serialize with encoded values

This is structural, not a runtime check. A `Encoded` value is built from
`Identifier` indices and holds no names, so no name can enter its content-hash
pre-image (`content-identity` hashes the stringless bytes). A NameTable serializes only its owned namespace slice (namespace, ordered
canonical names); borrowed slices keep their own archive and identity. The lookup index is derived, never archived. The two data shapes have disjoint
pre-images, so a rename — a table-only edit — cannot move any `Encoded` address. The
`archive` and `transaction` test suites prove the byte-level and identity-level
stability.

## The transactional contract

The accepted hardening requires that a failed decode alternative leave no
allocation effect. `name-table` meets it structurally rather than by undo: a
`NameTransaction` stages new names on the side and never mutates the committed
table until `commit`. A dropped or rolled-back transaction therefore leaves the
table byte-identical by construction — there is nothing to undo. A decode runs
each alternative inside a transaction (`begin`, or the `try_intern` closure form)
and commits only the winner; the loser is dropped and leaks nothing.

## The one home for the walkers

The derived-name rule was hand-written independently as `schema`'s
`Name::field_name` (PascalCase to `snake_case`, 16 call sites) and `schema-rust`'s
`ScreamingName::screaming` (PascalCase to `SCREAMING_SNAKE`). Both are the same
word-boundary walk. Here they are one private walk (`Name::derived_name`)
parameterized by a `DerivedCasing`, with `field_name` and `screaming` as the two
public spellings and `pascal_case` as the inverse. schema's walker first strips a
namespace through its own `local_part()`; that namespace split is a schema concern
and is deliberately excluded here — on a bare name the behaviors are identical,
which the `walkers` tests assert against the exact ported expectations.

## Constraints

- Every function is a method on a data-bearing type or a trait impl; no free
  helpers outside test code.
- Domain values are typed newtypes; the one casing axis is a `DerivedCasing` enum,
  not a boolean.
- Typed errors at the boundary; no `anyhow`/`eyre`.
- No unsafe code (`unsafe_code = "forbid"`).
- A `NameTable`'s archived bytes are its names and nothing else; the lookup index
  is never serialized.

## Invariants

- Interning is deterministic: a name interns to the same identifier every time
  within one table lineage.
- Identifiers are namespace-local and stable: a component allocates only into
  its home `u16` slice. Borrowed identifiers retain their original enum variant
  and local value; no flat-table extension or arithmetic conversion exists.
- A rolled-back or dropped transaction leaves the table byte-identical, down to
  `to_archive_bytes` and `identity` (proven in `tests/transaction.rs`).
- `field_name`/`screaming` reproduce the two source walkers exactly (proven in
  `tests/walkers.rs`).
- The derived-name walkers build strings ONLY at the `NameTable`
  interning/emission boundary — the psyche ruled of them "that is necessary." They
  are never reached inside the Nomos schema-to-logos transformation, which is
  stringless by his ruling that "in the nomos transformation (schema to logos),
  there shall be no string manipulation/introduction/reading of any kind." String
  work here is a boundary concern; the transformation between stringless forms
  stays index-only.

## Code map

- `src/lib.rs` — module root and public re-exports.
- `src/identifier.rs` — `Identifier`.
- `src/name.rs` — `Name`, `DerivedCasing`, the derived-name walkers.
- `src/table.rs` — `NameTable`, `NameTableDomain`.
- `src/transaction.rs` — `NameTransaction`.
- `src/boundary.rs` — `NameResolver`, `NameInterner`.
- `src/projection.rs` — `TextualProjection`.
- `src/error.rs` — `NameTableError`.
- `tests/interning.rs` — determinism, resolve round-trips, boundary capabilities.
- `tests/continuity.rs` — composed borrowed-slice stability without copying.
- `tests/transaction.rs` — rollback/commit and the interning-atomicity law.
- `tests/walkers.rs` — derived-name outputs vs the ported source expectations.
- `tests/archive.rs` — portable archive round-trip of a populated table.
- `tests/projection.rs` — the Textual-projection surface and rename-stability.
