# protos

`protos` is the implementation-free contract package for the Protos language
family. It relates component Capsules, their canonical short identifiers, and
textual projections without owning any component engine or language-specific
data.

The concrete mechanisms remain in their canonical micro-repositories:

- `content-identity` owns content hashes, the canonical `ShortCode`, and the
  `CapsuleNameTreeDomain`.
- `name-table` owns names and name tables.
- `raw-discovery` owns raw structural discovery.
- `structural-codec` owns structural forms and the bidirectional evaluator.

This package depends on exact published revisions of the contracts it names. It
contains no copied crates, workspace members, path dependencies, parser,
printer, evaluator, derive macro, short-code mint, or language-specific
Capsule implementation.

## Contracts

- `CapsuleKind` is closed to `Schema`, `Logos`, and `Nomos`. Rust is a textual
  Logos projection, not a Capsule kind.
- `ShortIdentifier` exposes `content_identity::ShortCode`; allocation policy
  stays with the owning component.
- `Capsule` requires its encoded truth, complete nametree, and non-optional
  typed content and nametree pins. Implementations provide pure identity
  derivations; the shared `verify` operation reports typed derivation failures
  or pinned-versus-actual mismatches.
- `TextualCapsuleAssociation` is implemented by the type that owns a projection
  association, never by `structural_codec::Textual`. Its associated textual
  representation may be source text, a whole document, or another textual
  view. Its associated Capsule is fixed for that implementation, while several
  association owners may target the same Capsule.

## Validation

```sh
cargo check --locked
cargo test --locked
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --locked --no-deps
nix flake check -L
```
