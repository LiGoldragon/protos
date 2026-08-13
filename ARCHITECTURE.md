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

The first pass is lexical. Parenthesized and curly-quoted string carriers keep
their interiors opaque to other delimiters; parentheses balance until their
final unbalanced closer. Interpretation of those interiors is a dialect seam.

## Out of scope

This crate does not add Meaning, Signal, archive identity, numeric registries,
Capsules, component contracts, a parser for any dialect, or a daemon. It does
not depend on Datom; Datom's earlier walk is read-only donor evidence.
