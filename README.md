# protos

The shared structural machinery of the NOTA/Core language family, consolidated
into one cargo workspace and one repository. These crates were authored as five
separate repositories pinned into each other by individual git revisions; protos
gathers them so the machinery moves as a unit, with intra-workspace path
dependencies instead of rev pins.

## Members

The workspace layers strictly downward — each crate depends only on the ones
above it:

- `content-identity` (L1) — the dependency-graph leaf: the one rkyv
  portable-archive discipline (`PortableArchive`) and one domain-separated,
  layout-versioned blake3 content hash. Depends only on `rkyv` and `blake3`.
- `name-table` (L2) — the stringless-Core identifier space: `Identifier` indices,
  the interning `NameTable` with a transactional staging surface, and the one home
  of the derived-name walkers (`field_name`, `screaming`, `pascal_case`).
- `raw-discovery` (L3) — the language-agnostic raw structure layer: discovers
  NOTA-family structure and never classifies. Adopts `content-identity`'s shared
  `PortableArchive` bound.
- `structural-codec` (L4) — the Core-associated, bidirectional, revisioned
  structural-form kernel: a trusted runtime evaluator over data-loadable dialect
  tables, governed by conformance laws.
- `structural-codec-derive` — the generated-codec side of conformance law 5: an
  attribute macro lowering one structural-form authority into the authoritative
  `StructuralEntry` data and an optimized `GeneratedCodec`, proven equivalent to
  the trusted evaluator. Its `fixtures` member carries the law-5 conformance suite.

## Consumers

`core-schema`, `core-logos`, `core-nomos`, and `textual-rust` consume this
machinery. They pin protos rather than the five former repositories.

## Build and test

```sh
nix flake check      # build, test, clippy, fmt, doc across the workspace — the gate
cargo test --workspace   # inner-loop tests
```
