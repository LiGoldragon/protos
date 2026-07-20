# content-identity

Portable content identity for stringless encoded values: the one rkyv
portable-archive discipline and one domain-separated, layout-versioned blake3
content hash.

This is the dependency-graph leaf of the shared-codec language family. It depends
only on `rkyv` and `blake3`, holds no strings in any Core-facing surface, and
depends on no other stack crate. Everything above it — the name table, the raw
discovery layer, the structural codec, and the Core language types — reinvented
this machinery independently and drifted; this crate is its single home.

## What it carries

- `PortableArchive` — the one rkyv portable-archive round-trip discipline
  (validation-on-read, fixed little-endian / 32-bit-pointer / unaligned layout),
  lifted verbatim from sema-engine's `EngineStoredValue` bound and
  blanket-implemented so consumers never restate it.
- `ContentHash<Domain>` — one generic 32-byte digest newtype, parameterized by a
  typed `HashDomain`, replacing the stack's five duplicate digest newtypes. The
  domain carries the layout-version tag, so "which layout" lives in the type, not
  a hand-remembered string suffix.
- `HashDomain`, `DomainSeparation`, `LayoutVersion` — typed, layout-versioned hash
  domains that reconcile the stack's two blake3 conventions storage-safely.
- `IdentityHasher` — the shared blake3 folding primitive, one home for the
  length-prefix convention.
- `Envelope<Domain>` — a content-addressed wrapper of stored bytes.

## The identity ruling

Content identity is blake3 over stringless encoded form rkyv bytes, with the NameTable
excluded, domain-separated, and layout-version-tagged. Because names are never in
the pre-image, a rename is hash-stable by construction.

## Storage-safe reconciliation

The stack had two incompatible blake3 domain conventions: schema's typed
`new_derive_key` contexts, and sema-engine's freeform length-prefixed magic
strings folded into a plain hasher. This crate unifies them behind one
`HashDomain` trait and one `ContentHash<Domain>` type without moving a single
stored byte:

- `DomainSeparation::Contextual` is the going-forward discipline — a derive-key
  context plus an explicit, structured layout-version preamble.
- `DomainSeparation::FrozenMagic` reproduces sema-engine's exact on-disk domain
  strings, so its stored digests never move when it migrates onto this crate.

The `tests/byte_compatibility.rs` suite proves this: it reproduces sema-engine's
`RecordKey` and `StoreSchemaHash` digests bit-for-bit through this crate's types.

## Build and test

```sh
nix flake check      # build, test, clippy, fmt, doc — the gate
cargo test           # inner-loop tests
```

## Status

Version 0.1.0. This is slice one, crate L1 of the accepted language-family
design. Consumption and integration — sema-engine, schema, and the Core language
crates migrating onto these types — will readapt to the forthcoming
release-train flow; this crate deliberately does not depend on any of them.
