# protos architecture

## Why this repository exists

protos is the separate home the psyche asked for the shared structural machinery
of the NOTA/Core language family: "consolidate into protos", the machinery in its
own separate repository. Before consolidation the five crates lived in five
repositories that pinned each other by individual git revisions, and consumers
pinned each of the five independently. That fanned every machinery change into a
rev-bump cascade. protos gathers the machinery into one workspace so it moves as a
unit; intra-workspace dependencies are path dependencies, and consumers pin one
repository instead of five.

## The downward layering

The workspace is a strict downward dependency family. Each crate depends only on
those above it, and the leaf depends on no sibling:

- `content-identity` is the leaf. It owns the one portable-archive discipline and
  the one content-hash family, and depends on no sibling.
- `name-table` depends on `content-identity`. It owns the identifier space and the
  single home of the derived-name walkers.
- `raw-discovery` depends on `content-identity`. It discovers raw structure and
  never classifies. It adopts `content-identity`'s `PortableArchive` bound rather
  than restating the rkyv feature discipline inline.
- `structural-codec` depends on `content-identity`, `name-table`, and
  `raw-discovery`. It is the trusted structural-form evaluator.
- `structural-codec-derive` is the derive sibling. Its `fixtures` member holds the
  law-5 conformance suite that proves the generated codecs agree with the trusted
  evaluator.

## The conformance boundary

Law-5 conformance has two halves. The half that proves the generated codec agrees
with the trusted evaluator is machinery-internal — it names only `structural-codec`,
`name-table`, `raw-discovery`, and the derive — and lives here, in
`structural-codec-derive/fixtures/tests/conformance.rs`.

The other half — that the derived entries equal `core-schema`'s hand-authored
entries — inherently reaches down to a consumer. It cannot live upstream without
forcing two incompatible versions of the machinery into one dependency graph (the
in-workspace path version and the version a re-pinned `core-schema` would pull by
git). It therefore lives downstream in `core-schema`, where the machinery and the
derive resolve at a single pin and their types unify. Keeping that cross-check
upstream would be the special case; placing it downstream is the normal case.
