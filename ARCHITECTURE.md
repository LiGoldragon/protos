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
transitive at runtime. The existing exact raw-discovery development pin remains
part of this package's locked producer family; the textual association contract
does not expose raw-discovery or structural-codec textual types.

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

`TextualCapsuleAssociation` belongs to an association owner, not to
`structural_codec::Textual` or to the textual representation. The owner may be
a type-level association marker or a component type with that responsibility.
Its associated `TextualRepresentation` and `Capsule` types make the relation
single-valued under Rust coherence: one association implementation chooses
exactly one Capsule. Different association owners may choose the same Capsule,
so several textual syntaxes can coexist without manufacturing new semantic
identities.

The textual representation is intentionally unconstrained by structural-codec.
It may be source text, a whole-schema document, or another textual view.
Structural-codec therefore never learns about Capsule, while protos retains its
allowed one-way dependency for the Capsule encoded-form contract.

Both directions are explicit:

- `view_capsule` purely produces the associated textual representation;
- `unview_capsule` purely recovers the fixed Capsule from that representation.

Each direction has its own typed error. The contract provides no mint, hidden
authority, parser, printer, evaluator, scoped structural identifier, or default
construction policy.

## Testing boundary

Runtime fixtures prove verification success and every typed failure, required
pin types, the three-kind closure, canonical `ShortCode` reuse, two projection
association types targeting one non-`Clone` Capsule, and both directions over
independently owned source/document text. Their fixture parsers reconstruct the
Capsule fields and names from that text, and report typed malformed and
incompatible-input errors. The projection representations do not implement
`structural_codec::Textual`; compile-fail documentation also proves that a
caller cannot select a different Capsule type.
Compile-fail documentation proves that no `Rust` kind exists, an association
owner cannot implement two associations, and callers cannot select a different
Capsule type at a call site.

The repository cannot enumerate all future downstream implementations without
reversing dependency direction. It therefore proves its closed types and
coherence laws locally and leaves component inventories to their owners.
