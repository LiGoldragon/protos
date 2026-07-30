# protos architecture

## Repository boundary

The micro-repositories are canonical. `protos` is one ordinary Cargo package,
not a workspace or implementation bundle. It owns only cross-component
contracts that cannot live in one mechanism repository.

Dependency direction is one-way:

```text
protos
  ├── content-identity
  ├── rkyv
  └── signal-frame
```

`content-identity` and `signal-frame` do not depend on `protos`. There is no
dependency on `name-table`, `signal-sema-translator`, `sema-translator`,
structural parsing, or a component engine.

## Wire-contract allocation

`protos` owns the append-only allocation data while `signal-frame` owns the
numeric types and frame encoding. The proven active families are:

- ordinary `signal-spirit`: `ContractId 1`, current `WireRevision 1`;
- owner-only `meta-signal-spirit`: `ContractId 2`, current `WireRevision 1`;
- dedicated `signal-spirit-judge`: `ContractId 3`, current `WireRevision 1`;
- `signal-sema-translator`: `ContractId 4`, current `WireRevision 1`.

The active table is compile-time data. Each record retains its complete ordered
set of explicitly supported decoder bindings and identifies one current binding
for new encoders. A breaking archived-body change appends a higher revision,
retains the old binding for its old decoder, and moves the current binding.
Contract IDs and revisions are nonzero. IDs are globally unique, never reused,
and remain permanently reserved in the typed retired table after retirement.

The family enum is the closed construction surface for build scripts. Every
family resolves to a typed `Active` or `Retired` allocation, so a tombstoned
family is never mistaken for an absent lookup. Record fields and constructors
are private, so consumers cannot choose arbitrary numeric identities. Alias
owners have no allocation entry here and must consume a canonical family
binding in their own component. Legacy unbound frames remain legacy and are not
relabelled as revision 1. The table does not allocate unproven census entries.

There is no mutable registry, discovery, file I/O, mint, daemon, hash-derived
wire identity, wire string, parser, printer, allocator, or component
implementation in this package.

## Capsule contract

`Capsule<Kind, CompleteNameTreePin>` is the generic component container.
`CapsuleKind` is public so downstream signatures can name the bound, but it is
private-sealed. Only the uninhabited marker types `Ethos`, `Nomos`, and `Logos`
implement it. This gives the three Capsules distinct Rust types and prevents a
downstream crate from inventing another kind. Rust remains a textual projection
of Logos and has no Capsule marker.

The Capsule has two stored positional fields:

1. one `CapsuleIdentity`;
2. one caller-defined, opaque complete NameTree pin.

Normal construction accepts only the inner `ContentAddressedHash`. The marker
maps that hash to `CapsuleIdentity::Ethos`, `CapsuleIdentity::Nomos`, or
`CapsuleIdentity::WholeLogos`. Callers cannot supply a conflicting outer
variant, and there is no second runtime kind field.

The pin is carried without a trait that would expose its topology. `protos`
does not compose the pin, verify it, resolve through it, or use it to derive the
hash. It also provides no Capsule content verifier. Those mechanisms remain
unwired pending their owning design and implementation surfaces.

Borrowing accessors expose the stored identity, inner hash, and opaque pin.
`into_parts` consumes the Capsule into the same positional identity-and-pin
pair; it adds no interpretation of either value.

### Checked archive construction

`Capsule` deliberately does not derive rkyv `Archive`, `Serialize`, or
`Deserialize`. A private positional archive carrier stores only the identity
and opaque pin. `to_archive_bytes` applies the shared portable-archive
discipline to that carrier.

`from_archive_bytes` first performs rkyv validation and deserialization into
the private carrier. It then compares the stored identity variant with the
variant fixed by `Kind`. An invalid enum discriminant is an archive failure; a
valid identity for another kind is the distinct typed `CapsuleKindMismatch`.
Only after both checks does it construct the requested Capsule. Therefore an
unchecked generic `Capsule` deserialization path is not part of the public
contract.

The marker is absent from the archive. The outer `CapsuleIdentity` discriminant
is the only stored kind datum, and the version-1 archive witness fixes that
layout. Changing it requires a new archive-version lock and a coordinated
consumer migration.

This contract intentionally does not decide:

- how module-owned tables relate to the complete composed NameTree pin;
- whether a Capsule identity is minted or derived;
- whether encoded content is recursively hashed per thing;
- how the pin itself is composed or verified.

## Encoded population

`EncodedPopulation<EncodedForm, NameTree>` is the neutral positional pair used
when a component boundary must carry a complete encoded form together with its
complete NameTree value. Both type parameters remain opaque. The carrier only
constructs, borrows, archives, and returns the two values in their original
positions.

The generic rkyv derives make concrete populations eligible for the shared
validated portable-archive discipline when their two component types meet that
discipline. The carrier itself does not validate that either input is complete,
compose or infer a Capsule pin, derive identity, assign slots, authorize a
caller, or define deployment and storage behavior. Each owning component must
provide those semantics at its own typed boundary.

## Textual association

`TextualCapsuleAssociation` belongs to an association owner, not to its textual
representation. Its associated `TextualRepresentation`, `Kind`, and
`CompleteNameTreePin` types make the relation single-valued under Rust
coherence: one association implementation chooses exactly one Capsule type.
Different association owners may choose the same Capsule, so several textual
syntaxes can coexist without manufacturing new semantic identities.

Both directions are explicit:

- `view_capsule` purely produces the associated textual representation;
- `unview_capsule` purely recovers the fixed Capsule from that representation.

Each direction has its own typed error. The contract provides no mint, hidden
authority, parser, printer, evaluator, name lookup, or default construction
policy.

## Testing boundary

The Capsule witnesses prove:

- the three marker-instantiated Capsule types are distinct;
- normal construction maps each marker to its one outer identity variant;
- the version-1 archive has an absolute positional byte lock and round-trips;
- mutations at the identity-hash and complete-pin positions affect only their
  respective restored fields;
- an invalid enum discriminant is rejected during archive validation;
- a valid archive restored as the wrong kind fails with the typed expected and
  actual variants;
- fields are private, the kind trait is sealed, and no Rust marker exists;
- two association owners can target one fixed Capsule while coherence rejects
  two selections by one owner.

The encoded-population witness separately proves accessor and consuming
position preservation, a validated portable-archive round trip, and refusal of
a truncated archive.

Wire-allocation tests separately prove the exact constants and short-header
bits, global active/tombstone uniqueness, nonzero identities, one current
binding per active declared family, monotonic explicit revision history,
distinct ordinary, owner-only, and Judge identities under the same local route,
private record construction, and the single-package exact-dependency boundary.
