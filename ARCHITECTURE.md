# Architecture

## Four layers

| Layer | Type | Descent (may fault) | Ascent (cannot fault) |
|---|---|---|---|
| Text | `Text`, `Potential<T>` | `Structural::delineate` | -- |
| Protoform | `Protoform`, `Delineation` | `Conceptual<C>::conceive` | `Printing::print` |
| Concept | dialect data model | `Datomic::incorporate` | `Protosizable::protosize` |
| Corporal | Rust value | -- | `Datomic::datomize` |

## Kinds (traits)

All behavior lives under traits. No free functions.

- **Structural**: `fn delineate(&self) -> Result<Delineation, Fault>`.
  Borne by Text. The sole character reader.
- **Printing**: `fn print(&self) -> Text`.
  Borne by Protoform and Delineation. The sole character writer.
- **Protosizable**: `fn protosize(&self) -> Protoform`.
  Borne by every concept type.
- **Conceptual<C: Protosizable>**: `fn conceive(&self) -> Result<C, Self::Fault>`.
  Borne by Protoform, once per dialect.
- **Actualizable<T: Embodied>**: `fn actualize(&self) -> Result<T, Self::Fault>`.
  Borne by Potential<T>.
- **Situating**: `fn situate(&self, path: &[Integer]) -> Option<Extent>`.
  Borne by Delineation.
- **Embodied**: Sized. Blanket-implemented for all Sized types.

## Delineation rules

The source text is read without modification (no normalization pass).

### Comments
A single `;` opens a comment to end of line. Comments are stripped.

### Delimiters
Six structural pairs and two opaque pairs:

| Glyph pair | Kind | Protoform variant |
|---|---|---|
| `{ }` | Structural (Braced) | Enclosed |
| `[ ]` | Structural (Bracketed) | Enclosed |
| `\u{00AB} \u{00BB}` | Structural (Guillemets) | Enclosed |
| `< >` | Structural (Angled) | Enclosed |
| `\u{201C} \u{201D}` | Opaque (CurlyQuotes) | Opaque |
| `( )` | Opaque (Parentheses) | Opaque |

### Bare runs and separators
A Bare is a maximal run of non-whitespace, non-delimiter, non-comment
characters. Inside a run, a separator (`.` `!` `:`) splits head from
body when a non-whitespace, non-closing character follows. Chained:
`a.b.c` is Headed(a, Period, Headed(b, Period, Bare(c))).

### Extents
Extents are byte offsets (Integer) into the source text. The Situation
in a Delineation maps each protoform's Path to its Extent.

## Canonical print rules

```
; ethos example of a Library declaration
Library.{0 15 0}
[]
[ Text Integer Decimal Boolean ]
```

```rust
// The target Rust of the types above
pub type Text = String;
pub type Integer = i64;
pub type Decimal = f64;
pub type Boolean = bool;
```

Canonical print produces a single line:
- Structural enclosures: `{ a b }` `[ a b ]` `\u{00AB} k v \u{00BB}` with space
  inside at both ends when non-empty; `{}` `[]` `\u{00AB}\u{00BB}` when empty.
- Angled: always tight, `<a b>`, no inner space.
- Headed: `Head.body` with separator glyph directly adjacent.
- Siblings: one space apart.
- Opaque: verbatim with their glyphs.
- Comments: never printed.
