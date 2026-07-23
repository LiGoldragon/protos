# Architecture — structural-codec-derive

This document states the durable direction of `structural-codec-derive`: what it is,
the one-authority principle it enforces, how each structural kind lowers to an entry
and a codec that the trusted evaluator proves equal, and its relationship to
`nota-derive` and the release train. It is the pickup point for the next agent on
the language-family train.

## Position in the language family

The next-generation NOTA family is four foundation crates with strictly downward
dependencies, then `core-schema` as the first real Core layer:

```
content-identity <- name-table <- raw-discovery <- structural-codec <- core-schema
```

`structural-codec` shipped the law-5 scaffolding — the `GeneratedCodec` trait and the
`ConformanceHarness` — with the note that "the evaluator is the sole implementation;
this trait has no generated implementor yet." `core-schema` made the Core layer real
and closed the signature-vs-Core deviation, but its `TextualSchema` reify/reflect
pair is a **hand-written** stand-in for the future generated codec.

`structural-codec-derive` is slice two, part two: the **first real
`GeneratedCodec` implementor**, generated from a derive. It is the generated-codec
side of law 5, standalone — the pattern `nota-derive` absorbs later.

## The one-authority principle (the crux)

One `#[structural_form(...)]` attribute is the single authority. `TypeSpec::expand`
lowers it three ways, so the three outputs cannot drift from each other:

1. **The authoritative `StructuralEntry`** (`T::structural_entry()`): the form as
   data. Built through the SAME `structural-codec` constructors a table author uses
   (`AuthoringForm::ObjectPrefixed(..).normalize()`, `StructuralForm::pascal_atom()`,
   `SequenceForm::Product`, `ConstructorCodec::new`, `PositionalSignature::new`), so a
   derived entry is `==` to the hand-authored one by construction, not by luck.
2. **The optimized `GeneratedCodec`**: a straight-line, type-specialized codec.
3. **The typed capture** the codec fills.

This is the accepted design's decision 7 ("nota-derive generates entries AND
optimized codecs AND conformance") made concrete: the **form is authoritative**, the
**codec is an optimization**, and **conformance is the safety net**.

## Why the generated codec is not the evaluator

The trusted evaluator (`structural-codec`) is generic over `StructuralValue`,
table-driven, and backtracks across a type's disjoint constructors. The generated
codec is its opposite by design: it knows the exact form at compile time, so it emits
direct `block.as_application()` / `block.as_delimited(D)` / `atom.text()` walks with
no table lookup and no dynamic dispatch — the fast path. Two properties make the two
provably equal rather than merely similar:

- **Interning order.** The generated decode validates the entire structural shape
  FIRST (no interning), then interns in the evaluator's depth-first order (head
  before payload, children left to right). This mirrors the evaluator's own two-phase
  "match to a draft, then resolve" discipline, so the `NameTable` deltas are
  byte-identical.
- **Interning atomicity.** The generated `decode` wraps its work in
  `NameTable::try_intern`, exactly as the evaluator does, so a failed decode leaves
  the table byte-unchanged (law 3) and the composed `decode_within` methods thread one
  transaction through the whole tree.

The `ConformanceHarness` then proves agreement on all four outputs — Core value,
`NameTable` delta, canonical output, typed error — over every fixture, so any future
divergence is a test failure.

## The five kinds and their lowering

Each kind mirrors one constructor shape of `structural-codec`'s kernel algebra. The
variant set lives in the `Kind` enum, never a string flag consulted at codegen.

- **`leaf(scalar)`** → form `Leaf(Scalar(s))`, empty signature. Capture: the native
  scalar (`i64`/`f64`/`String`/`bool`). Decode flattens with `Block::dotted_text` then
  parses; encode is `ScalarValue::render_block` — the same rejoin the evaluator uses,
  so float and string share one control path.
- **`delegate(inner = T)`** → form `Delegate { target: T::CORE_TYPE, payload: None }`,
  signature `[T::CORE_TYPE]`.
  Capture: a transparent wrapper over `T`. Decode/encode/`to_structural` recurse into
  `T`; `to_structural` adds the `Chosen{0, Delegated(..)}` layer the evaluator's
  `match_type ∘ Delegate` produces.
- **`newtype_declaration(inner, delimiter)`** → `Application(pascal, Delimited{D,
  Product([pascal])})`, signature `[inner::CORE_TYPE]`. Capture: the object and
  wrapped-type identifiers. This decodes a schema DECLARATION (`CommitSequence.{
  Integer }`), the self-hosting shape `core-schema` authors.
- **`struct_declaration(field_type, delimiter, fields)`** → `Application(pascal,
  Delimited{D, Product([Delegate { target: field_type, payload: None }; n])})`,
  signature = the fields'
  referenced types. Capture: object identifier + `Vec<field_type>`. Each field is
  decoded through the meta-type and wrapped in a `Delegated` layer.
- **`field_meta`** → ONE constructor: a bare `PascalCase` atom (elided name).
  Capture: a single-field struct `{ type_name }`. Field names are illegal in every
  Protos surface (psyche ruling 2026-07-19), so a field is nothing but the type at
  its position; the explicit `camelCase.PascalCase` application no longer parses.

## The mirrored fixture family (the proof)

`structural-codec-derive-fixtures` mirrors `core-schema`'s `FixtureFamily` entirely
through the macro, with ids matching `core-schema`'s fixture universe. `DerivedTable`
collects every derived `structural_entry()` and seals it into an addressed table — the
derive's entries are the ones sealed INTO the table the evaluator runs. The tests then
prove three independent things:

1. **Derived == authored.** Every derived entry equals `core-schema`'s hand-authored
   entry, per type. Drift is a failure.
2. **Signatures agree with Core.** The derived table passes
   `EncodedUniverse::validate_table` (every codec signature equals the Core field
   signature) and `validate_disjoint` (the `Field` constructors are provably distinct).
3. **Law 5.** The generated codecs agree with the evaluator on all four outputs across
   the family, including typed-error agreement on the three required malformed
   categories (wrong delimiter, unknown constructor shape, failed leaf parse).

## The declaration mirror, and a flagged reading

The `CommitSequence`/`StateDigest`/`DatabaseMarker` captures hold **identifiers**, not
values: their fixture forms are the schema-DECLARATION forms `core-schema` authored
(`CommitSequence.{ Integer }`), so a faithful capture stores the declaration's atoms.
This is the honest mirror of the fixture, and it is why the newtype capture carries
its own object name as data. A future value-level universe (where `CommitSequence`
wraps an actual integer VALUE) would use `delegate`/`leaf` forms instead; the derive
already supports those kinds. Flagged because both readings are defensible and the
fixture family fixes the choice.

## Crate shape

A two-crate workspace, because a `proc-macro` crate can only export procedural macros
(rust-crate-layout): the proc-macro crate `structural-codec-derive` (the attribute
macro) and the companion `structural-codec-derive-fixtures` (the mirrored types and the
law-5 suite). The companion is where the derived types live and where conformance runs;
it is the "runtime crate the proc-macro constraint requires."

## Greenfield by design — the coordination boundary

This crate does **not** edit `nota`, `nota-derive`, `core-schema`, `structural-codec`,
or any slice-one crate. `nota-derive` absorbs the entry+codec+conformance pattern later
on the release train; proving it standalone here means that absorption readapts to a
worked, proven reference rather than being invented during a live migration.

**Train status: currently NO-GO for riding the release train.** Cross-repository
consumption is by **pinned git rev** — the green path — not a materialized train.
Convergence and the eventual swap to train-pinned or path-unified dependencies readapt
to the release-train flow when it is ready; until then the git pins in `Cargo.toml` and
`fixtures/Cargo.toml` are authoritative.

## Upstream follow-ups for the manager

1. **`structural-codec` could expose a form-matches-case helper.** The generated
   codecs re-derive the atom-case check via `raw_discovery::AtomCase::of(atom) ==
   AtomCase::PascalCase`, duplicating the logic inside `AtomForm::accepts` (which is
   private to the evaluator's walk). A public `AtomForm::accepts` is already available;
   a first-class "does this atom satisfy this form's case" on the kernel would let the
   generated codec and the evaluator share one case predicate by name. Minor, not a
   blocker — the two computations are identical today.
2. **The canonical `Block → text` writer (`CanonicalText`) lives in `structural-codec`,
   flagged there as possibly belonging in `raw-discovery`.** The derive depends on it
   only transitively (through the harness); no action needed here, but the placement is
   worth resolving before the train materializes. Not a blocker.

No upstream change was required to build this crate: `structural-codec`'s
`GeneratedCodec` trait, `ConformanceHarness`, the public `StructuralEntry` /
`ConstructorCodec` / `StructuralForm` / `authoring` surfaces, and `core-schema`'s
`FixtureFamily` / `EncodedUniverse::validate_table` covered decode, encode, entry
construction, and validation with no fork.
