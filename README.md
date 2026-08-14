# protos

`protos` is the universal structural substrate shared by Protos-family
dialects. It owns universal text shapes, the first lexical block pass, text
carriers, the Head carried by dotted blocks, and the one frame discipline used
when text becomes real values or real values become text.

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
| `WalkObserving` / `FrameObserving` | structural walks and completed frames | expose read-only transition evidence |
| `CursorObserving` | direction drivers | exposes source/output byte cursors |
| `RealizeDriving` | `RealizeWalk` | scopes source root and nested bodies: scan, enter, dialect callback, close, then exactly-one resume |
| `TextualizeDriving` | `TextualizeWalk` | scopes output root and blocks: Head/delimiters, enter, dialect callback, closer, then exactly-one resume |

`ShapeDefined` is deliberately discrimination-only. A dialect's selected type
owns its own context and interior. The substrate does not define Datom,
Meaning, Signal, storage, a daemon, or a component vocabulary.

The first pass retains block bodies and UTF-8 byte extents, not inter-block
trivia. Parenthesis and curly-quote strings remain opaque inside every
brace/square structural block until their own balance-aware close.
`TextualizeWalk` emits one space between adjacent blocks: its result is
canonical block projection, not a byte-identical source formatter.

The scoped driver callbacks are the only recursive dialect seam. Typed dialect
contexts are bounded by `RealizeScoping` or `TextualizeScoping`, never by the
concrete drivers: the data-bearing `RealizeScope` and `TextualizeScope` do not
implement `Walk`, do not expose observations or cursors, and keep their driver
field private. A realization scope derives its nested body origin from the
parent `Block`; a textual scope can open another structural block or emit only
scalar/carrier body text. While a callback is live, its block frame remains
private in the shared neutral walk. The driver itself closes that frame and
resumes the parent; a failed callback faults its driver after cleanup, rather
than inventing a parent advancement.
`Block::span` and `Block::body_span` are UTF-8 extents rebased to the root
source, so a dialect witness can tie its own context work to actual source
text without gaining mutable frame access.

`TextualizeScoping::textualize_block` rejects a mismatched `Shape` and Head
before it mutates output or walk state: dotted shapes require a Head and every
other structural/string shape requires none. Scalar text remains unrestricted
content, including load-bearing symbols, only after a valid scope is live.

## Validation

```sh
cargo test --locked
nix flake check -L
```
