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
| `RealizeDriving` / `TextualizeDriving` | the two direction drivers | bind the neutral walk to source spans and emitted text |

`ShapeDefined` is deliberately discrimination-only. A dialect's selected type
owns its own context and interior. The substrate does not define Datom,
Meaning, Signal, storage, a daemon, or a component vocabulary.

The first pass retains block bodies and UTF-8 byte extents, not inter-block
trivia. Parenthesis and curly-quote strings remain opaque inside every
brace/square structural block until their own balance-aware close.
`TextualizeWalk` emits one space between adjacent blocks: its result is
canonical block projection, not a byte-identical source formatter.

## Validation

```sh
cargo test --locked
nix flake check -L
```
