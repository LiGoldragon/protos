# Protos

The universal structural substrate. Every dialect shares the
context-switching parse, the delimiters, the heads, and the recursive
structure. Protos is only about structure: anatomy, not interpretation.

## Layers

| Layer | Type | Descent (may fault) | Ascent (cannot fault) |
|---|---|---|---|
| Text | `Text` | `Protosizable::protosize` | |
| Protoform | `Protoform`, `Delineation` | `Conceivable<C>::conceive` | `Textualizable::textualize` |
| Concept | the dialect's data model | `Incorporable<T>::incorporate` | `Protosizable::protosize` |
| Corporate | the Rust value | | `Conceivable<C>::conceive` |

`Actualizable<T>::actualize` chains the whole descent.

## Delimiters

Five delimiter pairs: three structural (braces, brackets, angles) and
two opaque (curly quotes, parentheses).

## Separators

Period `.`, exclamation `!`, colon `:`. A separator splits a head from
its body when both sides are non-empty and neither neighbor is a
separator. Otherwise the run stays bare.
