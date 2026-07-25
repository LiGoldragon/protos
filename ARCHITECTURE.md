# protos architecture

## Repository boundary

The micro-repositories are canonical. `protos` is one ordinary Cargo package,
not a workspace and not an implementation bundle. It owns only cross-component
contracts that cannot live in a single mechanism repository.

Dependency direction is one-way:

```text
protos
  ├── content-identity
  ├── name-table
  └── structural-codec
        ├── content-identity
        ├── name-table
        └── raw-discovery
```

`structural-codec` does not depend on `protos`. The raw-discovery dependency is
transitive at runtime; `protos` names the same exact revision as a development
dependency only because a concrete law fixture implementing
`structural_codec::Textual` must spell its public `SealedTokenProfile` return
type.

## Capsule contract

`CapsuleKind` closes the component set to Schema, Logos, and Nomos. A textual
language or tool does not create another content kind. In particular, Rust is a
projection of Logos and owns neither a Capsule kind nor an identity domain here.

`ShortIdentifier` returns the canonical numeric `content_identity::ShortCode`.
Minting and collision state are authority-bearing implementation concerns and
remain outside this package.

`Capsule` has required, pure accessors for:

- the component's stringless encoded truth;
- its complete nametree;
- a non-optional `ContentHash<ContentDomain>` pin;
- a non-optional `ContentHash<CapsuleNameTreeDomain>` pin.

The implementing component supplies pure derivation methods for both identities.
The provided `verify` operation compares derived identities with required pins.
Its closed outcomes distinguish content derivation, nametree derivation, content
mismatch, and nametree mismatch; mismatch values retain both the pinned and
derived hashes. There is no optional-pin state and no runtime domain tag.

## Textual association

`TextualCapsuleAssociation` belongs to the textual projection type. Its
associated Capsule is constrained so
`Capsule::EncodedForm == structural_codec::Textual::Encoded`.

An associated type, rather than a caller-selected generic Capsule parameter,
makes the association single-valued under Rust coherence: a projection chooses
one Capsule. Different projection types may choose the same Capsule, so several
views can coexist without manufacturing new semantic identities.

Both directions are explicit:

- `view_capsule` produces the projection's `TextualForm`;
- `unview_capsule` consumes that view, component-owned typed context, and an
  already-issued `ShortCode`.

The context exposes its nametree explicitly. The contract provides no mint,
hidden authority, parser, printer, evaluator, or default construction policy.

## Testing boundary

Runtime fixtures prove verification success and every typed failure, required
pin types, the three-kind closure, canonical `ShortCode` reuse, and two
projection types round-tripping one Capsule type. Compile-fail documentation
proves that no `Rust` kind exists, a projection cannot implement two
associations, and a projection cannot associate a Capsule with a different
encoded type.

The repository cannot enumerate all future downstream implementations without
reversing dependency direction. It therefore proves its closed types and
coherence laws locally and leaves component inventories to their owners.
