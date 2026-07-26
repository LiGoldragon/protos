# protos

`protos` is the implementation-free contract package for the Protos language
family. It relates component Capsules and textual projections without owning
any component engine or language-specific data.

The concrete mechanisms remain in their canonical micro-repositories:

- `content-identity` owns content hashes, their resolver-scoped display
  projections, and the `CapsuleNameTreeDomain`.
- `name-table` owns names and name tables.
- `raw-discovery` owns raw structural discovery.
- `structural-codec` owns structural forms and the bidirectional evaluator.
- `signal-frame` owns the nonzero numeric binding types and short-header
  encoding.

This package depends on exact published revisions of the contracts it names. It
contains no copied crates, workspace members, path dependencies, parser,
printer, evaluator, derive macro, display allocator, or language-specific
Capsule implementation.

## Contracts

- `CapsuleKind` is closed to `Schema`, `Logos`, and `Nomos`. Rust is a textual
  Logos projection, not a Capsule kind.
- `Capsule` requires its encoded truth, complete nametree, and non-optional
  typed content and nametree pins. Implementations provide pure identity
  derivations; the shared `verify` operation reports typed derivation failures
  or pinned-versus-actual mismatches.
- A short display is computed only from a domain-typed content hash and a
  caller-owned resolver. It is not stored in, archived with, or returned by a
  Capsule.
- `TextualCapsuleAssociation` is implemented by the type that owns a projection
  association, never by `structural_codec::Textual`. Its associated textual
  representation may be source text, a whole document, or another textual
  view. Its associated Capsule is fixed for that implementation, while several
  association owners may target the same Capsule.
- `WireContractFamily` closes the proven allocation set to ordinary
  `signal-spirit`, owner-only `meta-signal-spirit`, and the dedicated
  `signal-spirit-judge` request/reply contract. Their current bindings are
  `(ContractId 1, WireRevision 1)`, `(ContractId 2, WireRevision 1)`, and
  `(ContractId 3, WireRevision 1)`.
  Each family has a total typed active-or-retired allocation; compile-time
  projections derive from one canonical declaration. Alias owners receive no
  allocation entry and consume a canonical binding in their own component;
  legacy unbound frames are not revision 1.

## Validation

```sh
cargo check --locked
cargo test --locked
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --locked --no-deps
nix flake check -L
```
