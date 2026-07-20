# name-table

The stringless encoded-form identifier space: namespace-variant `Identifier`
values, composed `NameTable` slices with transactional interning, one canonical
name per identifier, and the one home for derived-name walkers.

This is crate L2 of the shared-codec language family. The family's rule is that
dependencies run strictly downward and stringless encoded form never depends on text:
`content-identity <- name-table <- raw-discovery <- structural-codec`. This crate
sits just above the leaf and depends only on `content-identity` (for the shared
`PortableArchive` rkyv discipline) and `rkyv`.

## The stringless principle

Every `Encoded*` type in the family is stringless: it carries `Identifier`
values, never names. An identifier is its component namespace variant plus a
`u16` local allocation, not a flat integer. All names live in `NameTable`; an
encoded value made only of identifiers has no name in its bytes, so:

- **Rename-stability by construction.** Content identity (from `content-identity`,
  over a `Encoded` value's stringless bytes) never folds a name. A rename is a
  `NameTable`-only edit that changes how identifiers resolve, and can never move
  `Encoded` identity. Names and `Encoded` values are structurally incapable of sharing a
  serialization pre-image — a table's canonical bytes are its ordered names and
  nothing else.

## What it carries

- `Identifier` — a closed namespace enum (`Schema(u16)`, `Logos(u16)`,
  `LogosStandard(u16)`, `Nomos(u16)`, and fixtures). Equal locals in different
  variants are distinct without namespace arithmetic.
- `NameTable` — one component's composed view: it owns one append-only home slice
  and borrows completed source slices without copying or renumbering them.
  `intern` writes only the home slice and `resolve` dispatches by identifier
  variant. The old `extend_from` flat table is retired.
- `NameTransaction` — a speculative interning overlay that merges on commit. A
  failed decode alternative leaves no allocation effect, because the committed
  table is never mutated until commit — a dropped transaction is an effect-free
  rollback by construction, not by undo.
- `Name` — the interned name and the one home of the derived-name rule:
  `field_name` (PascalCase to `snake_case`), `screaming` (to
  `SCREAMING_SNAKE_CASE`), `pascal_case` (the inverse). These consolidate walkers
  that were hand-written independently in `schema` (`Name::field_name`) and
  `schema-rust` (`ScreamingName::screaming`).
- `NameResolver` / `NameInterner` — the two codec-boundary capabilities: the
  read-only view an encode path is threaded, and the mutating view a decode path
  is threaded. A codec never holds the whole table, only the capability its
  direction needs.
- `TextualProjection` — the surface for deriving a named `Textual*` view from a
  `Encoded` value plus a table. The named view is derived on demand, never stored;
  concrete `Textual*` types belong to later crates.

## The transactional contract

Interning is transactional so a failed decode never leaks a name. Run each decode
alternative inside a transaction (`NameTable::begin`, or `NameTable::try_intern`
for the closure form) and commit only the winner:

- Names interned through the transaction stage on the side, at the identifiers
  they would occupy after a commit.
- `commit` merges the staged names into the committed table.
- `rollback` (or simply dropping the transaction) discards the staging buffer; the
  committed table is untouched, down to its archived bytes and its identity.

## Build and test

```sh
nix flake check      # build, test, clippy, fmt, doc — the gate
cargo test           # inner-loop tests
```

## Status

Version 0.3.0 removes transparent alias storage and introduces the sliced identifier archive layout. Existing
flat-table archives are intentionally not decoded as sliced data: consumers must
advance in the producer-to-consumer train and regenerate their encoded/name-table
pairs under the new layout.
