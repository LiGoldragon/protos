# protos

`protos` is the implementation-free contract package for the Protos language
family. It relates kind-typed Capsules, textual projections, and wire-contract
allocations without owning a component engine or language-specific data.

The concrete mechanisms remain in their canonical micro-repositories:

- `content-identity` owns whole-Capsule identities and pure-content hashes.
- `signal-frame` owns the nonzero numeric binding types and short-header
  encoding.

`protos` uses rkyv only for its checked Capsule archive boundary. It has no
name-table, translator, parser, printer, evaluator, daemon, path dependency, or
language-specific Capsule implementation.

## Contracts

- `Capsule<Kind, CompleteNameTreePin>` is a generic struct with private fields.
  The private-sealed `CapsuleKind` construction set is `Ethos`, `Nomos`, and
  `Logos`; Rust has no marker.
- Normal construction accepts a raw `ContentAddressedHash` and maps the marker
  to exactly one stored outer variant: `Ethos`, `Nomos`, or `WholeLogos`.
- The complete NameTree pin is opaque. This package stores and returns it but
  does not compose, verify, interpret, or query it. Content verification is
  likewise not wired into this contract.
- Accessors borrow each stored value, and `into_parts` consumes the Capsule into
  the same `(CapsuleIdentity, CompleteNameTreePin)` positional pair.
- The portable archive contains exactly the stored `CapsuleIdentity` and pin,
  positionally. The marker is not stored separately. Checked restoration first
  validates the archive, then refuses a valid identity variant that disagrees
  with the requested marker using `CapsuleKindMismatch`.
- `TextualCapsuleAssociation` fixes the kind and complete-pin type for one
  association owner. Several owners may target the same Capsule type, while one
  owner cannot select multiple Capsule types.
- `WireContractFamily` closes the proven allocation set to ordinary
  `signal-spirit`, owner-only `meta-signal-spirit`, the dedicated
  `signal-spirit-judge` request/reply contract, and
  `signal-sema-translator`. Their current bindings are
  `(ContractId 1, WireRevision 1)`, `(ContractId 2, WireRevision 1)`,
  `(ContractId 3, WireRevision 1)`, and
  `(ContractId 4, WireRevision 1)`.
  Each family has a total typed active-or-retired allocation; compile-time
  projections derive from one canonical declaration. Alias owners receive no
  allocation entry and consume a canonical binding in their own component;
  legacy unbound frames are not revision 1.

The Capsule contract does not decide how module tables compose the complete
NameTree pin, whether Capsule identity is minted or derived, or whether encoded
content gains recursive per-thing hashing.

## Validation

```sh
cargo check --locked
cargo test --locked
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --locked --no-deps
nix flake check -L
```
