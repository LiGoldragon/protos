# structural-codec-derive

The **generated-codec side of conformance law 5**, standalone. An attribute macro
that lowers **one structural-form authority** into three things — the authoritative
form-as-data, an optimized codec, and the proof they agree — exactly the pattern
`nota-derive` will absorb later on the release train, proven collision-free now
without touching `nota`.

## The one-authority contract

A single `#[structural_form(...)]` attribute is the authority. From it the macro
generates (design decision 7, psyche-accepted):

1. **The authoritative `StructuralEntry`** — `T::structural_entry()` returns the
   form as data, byte-identical to a hand-authored entry. This is the entry sealed
   INTO the table the evaluator runs.
2. **An optimized `GeneratedCodec`** — a straight-line, type-specialized
   `decode`/`encode`/`to_structural` that walks the known form directly through
   raw-discovery primitives and **never consults the trusted evaluator**. Interning
   is atomic (a failed decode leaves the `NameTable` byte-unchanged), in the
   evaluator's DFS order.
3. **The typed capture** the codec fills — a struct or enum mirroring the
   constructor's decoded content.

`structural-codec`'s `ConformanceHarness` then proves the generated codec and the
evaluator agree on **all four outputs**: the Core value, the `NameTable` delta, the
canonical output, and the typed-error decision — including error agreement on
malformed inputs.

## What a user writes, and what is generated

```rust
use structural_codec_derive::structural_form;

// A scalar leaf.
#[structural_form(id = 10, leaf(Integer))]
pub struct Integer;

// A newtype DECLARATION `CommitSequence.{ Integer }`.
#[structural_form(id = 1, newtype_declaration(inner = Integer, delimiter = Brace))]
pub struct CommitSequence;
```

Each attribute names a scoped Core-type `id` (in the fixture universe) and one
`kind`. The macro replaces the named placeholder with the typed capture, an inherent
`structural_entry()`, and a `GeneratedCodec` implementation. The five kinds cover
the whole fixture family:

| kind | form | typed capture |
| --- | --- | --- |
| `leaf(Integer\|Float\|Text\|Boolean)` | `Leaf(Scalar(..))` | the native scalar |
| `delegate(inner = T)` | `Delegate { target: T::CORE_TYPE, payload: None }` | a transparent wrapper over `T` |
| `newtype_declaration(inner = T, delimiter = D)` | `Object.{ Type }` | object + wrapped-type identifiers |
| `struct_declaration(field_type = F, delimiter = D, fields = [..])` | `Object.{ Field* }` | object identifier + `Vec<F>` |
| `field_meta` | a bare `Type` (the elided name) | a single-field struct `{ type_name }` |

## The mirrored fixture family

`structural-codec-derive-fixtures` mirrors `core-schema`'s `FixtureFamily` entirely
through the macro — `Integer`/`Float`/`Text` leaves, the `Documentation → Summary →
Text` string-rejoin delegate chain, the `Field` meta-type, the
`CommitSequence`/`StateDigest` newtype declarations, and the `DatabaseMarker` struct
whose three bare positional fields include two same-typed `StateDigest` fields told
apart by position alone (field names are illegal). The test here proves:

- **Law 5** (`tests/conformance.rs`): the generated codecs agree with the evaluator
  on all four outputs across the whole family, including typed-error agreement on
  wrong-delimiter, unknown-constructor-shape, failed-leaf-parse, and the now-illegal
  explicit `name.Type` field inputs.

The complementary **derived == authored** cross-check — that every
`T::structural_entry()` equals `core-schema`'s hand-authored entry for the same type
— lives downstream in `core-schema`, where the machinery and the derive resolve at a
single protos pin and the entry types unify. Holding it here would force two
machinery versions into one dependency graph.

## Dependencies

Consumed across repositories by **pinned git rev** — the green path while the
release train is assembled, exactly as the slice-one and `core-schema` crates
consume each other:

| crate | rev |
| --- | --- |
| `structural-codec` | `104f92454a5ba88b376fa706a9fe38c4a4b65ee0` |
| `core-schema` (dev) | `33e5be2769b87920b773c7707c1ceb2c97cd42e8` |
| `name-table` | `c3237f77c087e6feab49d6cf34971cebc14a11e6` |
| `raw-discovery` | `a4e8c6df84e6a487ca6fe2f3641f9bafd0b0d8c8` |
| `content-identity` (transitive) | `6cc0408cdb96f174cc8fdf6ca23420038de28450` |

## Build & test

`nix flake check` is the gate (build, test, clippy, fmt, doc). `cargo test
--workspace` is the inner loop.

## Relationship to nota-derive and the release train

Greenfield by design — see `ARCHITECTURE.md`. This crate does **not** edit `nota`,
`nota-derive`, `core-schema`, or the four slice-one crates. It proves the
entry+codec+conformance pattern `nota-derive` will absorb later, so that absorption
readapts to a worked, proven reference rather than being invented live.

**Train status: currently NO-GO for riding the release train.** Cross-repository
consumption is by pinned git rev — the green path — not a materialized train; the
git pins in `Cargo.toml` are authoritative until the train is ready.
