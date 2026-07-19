# protos — known non-idealities

## structural-codec-derive `field_meta` still emits the explicit field-name constructor

Symptom: the derive's `field_meta` form (exercised by the `Field` fixture,
`id = 23`) lowers TWO constructors — the bare elided-name form `Type` AND the
explicit `camelCase.PascalCase` form, i.e. a `name.Type` field-name rendering.
`core-schema` (at HEAD) has already dropped the explicit constructor from its
hand-authored `Field`, so its authored entry has ONE constructor. The derive and
`core-schema` have therefore drifted: the derive is stale.

This surfaces only when the derived entries are compared against `core-schema`'s
authored entries (the downstream half of law-5 conformance). The
machinery-internal half — the generated codec agrees with the trusted evaluator
over the derive's OWN table — is self-consistent and green
(`structural-codec-derive/fixtures/tests/conformance.rs`).

Current workaround: the downstream cross-check is NOT relocated into `core-schema`
in the consolidation train, because relocating it would ship a red test or need an
ignore-bypass, and the derive's semantics are not this train's to change.

Proper fix / open question: the explicit `name.Type` form the derive still emits is
a field-name rendering, which the standing total field-name ban outlaws. Bringing
the derive's `field_meta` into line with the ban (emit only the elided form, so its
`Field` entry matches `core-schema`'s one-constructor authored entry) is
field-name-ban work — psyche-governed and entangled with the Core* -> Encoded*
rename assigned to the Codex bootstrap train — not a clean in-scope consolidation
change. Once the derive is reconciled, the downstream cross-check should be seated in
`core-schema` (dev-dep on `structural-codec-derive-fixtures`), where the machinery
and derive resolve at one protos pin and the types unify.
