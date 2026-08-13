# protos architecture

## Repository boundary

`protos` is the quick-new universal substrate. It has no Cargo, Nix, runtime,
or build edge to legacy or frozen Protos-family repositories. Its only product
surface is the Rust standard library.

It supplies the fixed `Shape` vocabulary, `ShapeDefined`, `Head`, lexical
`Block` scanning, `SourceText` and `StringCarrier`, the form directions
`Realize` and `Textualize`, and the single neutral `StructuralWalk` behind
`RealizeWalk` and `TextualizeWalk`.

## Structural division

```text
SourceText --Realize--> lexical Blocks --dialect-owned selection/context--> real values
real values --dialect-owned projection--> Blocks --Textualize--> SourceText

                    RealizeWalk / TextualizeWalk
                              |
                       StructuralWalk
                 enter · close · position · resume
```

`ShapeDefined::select` receives only a universal shape and optional Head. It
cannot consume a block body. A selected dialect type owns the next context;
the walk keeps the parent's frame untouched until the child closes, then
resumes the parent exactly once.

The first pass is lexical. All block extents and driver cursors are UTF-8 byte
offsets; `SourceSlicing` is the safe access capability. Parenthesized and curly-quoted string carriers keep
their interiors opaque to other delimiters; parentheses balance until their
final unbalanced closer. Interpretation of those interiors is a dialect seam.

Inter-block trivia is not a block. The substrate therefore provides canonical
block projection with one space between blocks, not byte-identical preservation
of a source document's formatting. Dialects that require source fidelity own a
separate textual concern.

## Out of scope

This crate does not add Meaning, Signal, archive identity, numeric registries,
Capsules, component contracts, a parser for any dialect, or a daemon. It does
not depend on Datom; Datom's earlier walk is read-only donor evidence.
