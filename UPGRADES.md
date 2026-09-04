# Upgrading from protos 0.15.0 to 0.15.1

## Non-breaking additions

### `Situated<F>` derives `Clone, Debug, PartialEq, Eq`

`Situated<F>` now derives `Clone`, `Debug`, `PartialEq`, and `Eq`
conditionally on `F`. No migration needed; existing code continues to
compile. Code that previously required manual `PartialEq` implementations
for `Situated` types can now use the derived equality.

---

# Upgrading from protos 0.14 to 0.15

## Breaking changes

### Portion is now Protoform

The `Portion` enum is renamed to `Protoform`. Its variants changed:

| 0.14 | 0.15 |
|---|---|
| `Portion::Headed(Extent, Headed)` | `Protoform::Headed(Symbol, Separator, Box<Protoform>)` |
| `Portion::Enclosed(Extent, Enclosed)` | `Protoform::Enclosed(Enclosure, Vec<Protoform>)` |
| `Portion::Bare(Extent, Bare)` | `Protoform::Bare(Symbol)` |
| -- | `Protoform::Opaque(Boundary, Text)` |

Protoforms no longer carry extents. Extents are tracked in the
`Delineation::situation` map, keyed by `Path`.

### Text is now a type alias

`Text` is `String`, not a wrapper struct. No more `ContentHash`,
`Delineation` inside Text, normalization on construction, or
`BareExpectation`.

### Prospective is now Potential

`Prospective<T>` (which was `Text<T>`) is now `Potential<T>`.
It wraps a String and is `From<Text>` and `From<&str>`.

### Trait renames

| 0.14 | 0.15 |
|---|---|
| `Delineatable` | `Structural` |
| `Embodied` (from_portion) | removed; use `Datomic::incorporate` |
| `Textualizable` (to_portion) | removed; use `Datomic::datomize` |
| `Printing::print(Layout)` | `Printing::print()` |

`Structural::delineate` is now on `Text` (String), not on a wrapper.
There is no Layout parameter; print always produces canonical flat text.

### Comment marker

The comment marker changed from `;;` to `;` (single semicolon).

### Canonical print spacing

Canonical print now puts a space inside `{ }`, `[ ]`, and guillemets
at both ends when non-empty. Empty enclosures are tight. Angled
brackets are always tight.

### Removed types

`ContentHash`, `BareExpectation`, `BareSafe`, `Layout`, `ScalarAnatomy`,
`PortionText`, `EnclosedAnatomy`, `EnclosedArity`, `ShapeDefined`,
`DelineatedText`, `ContentHashable`, `Symbol` (as a struct),
`StructuralEnclosure`, `StructuralEnclosed`, `OpaqueEnclosed`,
`OpaqueBoundary`, `DialectBoundary`, `Boundary` (as Structural/Opaque),
`Enclosure` (as the old 5-variant form including CurlyQuote).

### Migration steps

1. Replace `Portion` with `Protoform` everywhere.
2. Replace `Text<T>` with `Potential<T>`.
3. Replace `Delineatable::delineate()` with `Structural::delineate()`.
4. Replace `Embodied::from_portion` with `Datomic::incorporate`.
5. Replace `Textualizable::to_portion` with `Datomic::datomize`.
6. Replace `Printing::print(Layout::Flat)` with `Printing::print()`.
7. Replace `;;` comments with `;`.
8. Update tests expecting tight delimiters to expect spaced delimiters.
