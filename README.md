# protos

`protos` is the universal structural substrate shared by Protos-family
dialects. It owns universal text shapes, the first lexical block pass, text
carriers, the Head carried by dotted blocks, headless guillemet structural
blocks, and the one frame discipline used when text becomes real values or real
values become text.

## Trait roster

| Trait | Main types | Duty |
| --- | --- | --- |
| `ShapeDefined` | dialect-owned discriminated types | exposes `shapes()` and selects from `Shape` plus `Head`; it never sees a block interior |
| `Headed` | `Block` | reveals an optional Head |
| `BlockScanning` | `SourceText` | separates text into lexical `Block`s |
| `StringCarrying` | `StringCarrier` | borrows a carrier's lexical body |
| `Realize` | `SourceText` | turns textual data into real blocks |
| `Textualize` | `Block` | projects a real block into textual data; lexical carriers are not real values |
| `Walk` | `StructuralWalk`, `RealizeWalk`, `TextualizeWalk` | owns `enter`, `close`, `position`, and `resume` |
| `WalkObserving` / `ObservationViewing` | structural walks and observations | expose a copied append-only transition history |
| `TransitionObserving` / `FrameObserving` / `ParentObserving` / `IdentityObserving` | transition records, frames, parent facts, identities | disclose transition kind, absolute span, frame identity, and parent position without mutation |
| `CursorObserving` | direction drivers | exposes source/output byte cursors |
| `RealizeDriving` | `RealizeWalk` | scopes source root and nested bodies: scan, enter, dialect callback, close, then exactly-one resume |
| `TextualizeDriving` | `TextualizeWalk` | scopes output root and blocks: Head/delimiters, enter, dialect callback, closer, then exactly-one resume |

`ShapeDefined` is deliberately discrimination-only. A dialect's selected type
owns its own context and interior. The substrate does not define Datom,
Meaning, Signal, storage, a daemon, or a component vocabulary.

The first pass retains block bodies and UTF-8 byte extents, not inter-block
trivia. Parenthesis and curly-quote strings remain opaque inside every
brace, square, or guillemet structural block until their own balance-aware
close.
`TextualizeWalk` emits one space between adjacent blocks: its result is
canonical block projection, not a byte-identical source formatter.

The scoped driver callbacks are the only recursive dialect seam. Typed dialect
contexts are bounded by `RealizeScoping` or `TextualizeScoping`, never by the
concrete drivers: the data-bearing `RealizeScope` and `TextualizeScope` do not
implement `Walk`, do not expose observations or cursors, and keep their driver
field private. Each realization scope is branded by the driver with the actual
live block body and its absolute UTF-8 extent; `realize_body` takes no source,
origin, or Block argument. A callback may inspect or clone its separately
provided `Block` for discrimination, but cannot submit it as recursive
provenance. A textual scope can open another structural block or emit only
scalar/carrier body text. While a callback is live, its block frame remains
private in the shared neutral walk. The driver itself closes that frame and
resumes the parent; a failed callback faults its driver after cleanup, rather
than inventing a parent advancement.
`SourceText::blocks()` reports `Block::span` and `Block::body_span` relative
to that `SourceText`. Scoped `RealizeDriving` callbacks rebase both to the
root source before exposing a block to a dialect, so a recursive witness can
tie context work to actual root text without gaining mutable frame access.

Every reusable `RealizeWalk` or `TextualizeWalk` root run resets its transition
history and frame identities to zero before its document-root `enter`. The
returned `WalkObservation` is therefore evidence for that one source or output
run. A successful root closes without a parent or resume. A failed run retains
its actual close-only history, is marked faulted, and cannot be reused.

`TextualizeScoping::textualize_block` rejects a mismatched `Shape` and Head
before it mutates output or walk state: dotted shapes require a Head and every
other structural/string shape requires none. Scalar text remains unrestricted
content, including load-bearing symbols, only after a valid scope is live.

## Validation

```sh
cargo test --locked
nix flake check -L
```
