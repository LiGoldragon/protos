# content-identity architecture

## Overview

`content-identity` is the leaf of the shared-codec language family's crate graph.
The whole family's rule is that dependencies run strictly downward and stringless
Core never depends on text: `content-identity <- name-table <- raw-discovery <-
structural-codec`. This crate sits at the bottom and depends only on `rkyv` and
`blake3`. It holds the portable-archive discipline and the content-hash primitive
that every layer above reinvented independently.

## Direction

The psyche's settled rulings this crate embodies:

- Content identity is blake3 over stringless encoded form rkyv bytes, with the NameTable
  excluded, domain-separated, and layout-version-tagged. Rename is hash-stable.
- All `Encoded*` types are stringless; the identity machinery must never require
  names. Names live in a separate NameTable that is never part of a hash
  pre-image.
- The stack's two existing blake3 conventions — schema's typed `new_derive_key`
  domains and sema-engine's freeform length-prefixed magic strings — reconcile
  storage-safely: adopt typed domains, but preserve sema-engine's exact existing
  domain strings and derivation so on-disk digests do not move.

Consumption and integration will readapt to the forthcoming release-train flow.
This crate does not depend on any other stack crate and does not migrate any
consumer; sema-engine, schema, and the Core language crates adopt these types in
later train slices.

## Components and boundaries

- `PortableArchive` (`src/portable.rs`) — the rkyv round-trip bound lifted
  verbatim from sema-engine's `EngineStoredValue`/`EngineStoredRecord`
  (`record.rs:153-198`). Blanket-implemented; adds `to_archive_bytes` /
  `from_archive_bytes` so canonical bytes are reachable from one place.
- `IdentityHasher` (`src/hasher.rs`) — a `blake3::Hasher` wrapper owning the two
  folding disciplines: `update_raw` (tag bytes, raw little-endian counts) and
  `update_length_prefixed` (sema-engine's `update_bytes` primitive,
  `versioning.rs:299-302`). The single home for the length-prefix convention.
- `HashDomain` / `DomainSeparation` / `LayoutVersion` (`src/domain.rs`) — a
  domain is a marker type reporting a `DomainSeparation` and a `LayoutVersion`.
- `ContentHash<Domain>` (`src/hash.rs`) — one 32-byte digest newtype, domain in a
  `PhantomData` marker so digests under different domains cannot be confused.
- `Envelope<Domain>` (`src/envelope.rs`) — payload bytes plus layout version plus
  the payload's content hash, with a self-consistency `verify`.
- `ArchiveError` (`src/error.rs`) — the typed crate-boundary error (thiserror).

## The two blake3 disciplines, one type

`DomainSeparation` names how a domain primes its blake3 pre-image:

- `Contextual { context, layout }` — going-forward: blake3 `new_derive_key`
  context, then the layout version folded as an explicit length-prefixed
  preamble. The layout is a structured field, not a string suffix, so a layout
  bump is a typed, reviewable version change.
- `FrozenMagic { magic, layout }` — storage-frozen: a plain hasher whose first
  fold is the length-prefixed magic string, which already encodes its own version
  in the bytes. This reproduces sema-engine's exact on-disk domain strings; the
  reported `layout` is for inspection and is never double-folded.

This is how the special case dissolves: one `HashDomain` trait and one
`ContentHash<Domain>` type serve both conventions, and byte-compatibility holds
because the frozen discipline changes no bytes.

## Constraints

- No strings in Core-facing surfaces. Domain contexts and magic strings are
  static configuration, not Core data.
- Every function is a method on a data-bearing type; no free helpers outside test
  code.
- Typed errors at the boundary; no `anyhow`/`eyre`.
- rkyv storage support on `ContentHash` is deliberately elided at this slice (the
  authoritative design marks it "rkyv derive elided"). A consumer that stores a
  `ContentHash` in an archived record adds that surface when it migrates.

## Invariants

- `of_core` never folds a name; identity depends only on the stringless canonical
  bytes, so a rename cannot move an address.
- A `FrozenMagic` domain reproduces the corresponding sema-engine digest
  bit-for-bit (proven in `tests/byte_compatibility.rs`).
- Distinct domain contexts and distinct layout versions separate the address
  space for identical bytes (proven in `tests/derivation.rs`).

## Code map

- `src/lib.rs` — module root and public re-exports.
- `src/portable.rs` — `PortableArchive`.
- `src/hasher.rs` — `IdentityHasher`.
- `src/domain.rs` — `HashDomain`, `DomainSeparation`, `LayoutVersion`.
- `src/hash.rs` — `ContentHash<Domain>`.
- `src/envelope.rs` — `Envelope<Domain>`.
- `src/error.rs` — `ArchiveError`.
- `tests/byte_compatibility.rs` — sema-engine digest reproduction.
- `tests/derivation.rs` — determinism, separation, round-trip, envelope.
