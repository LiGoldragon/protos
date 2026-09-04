# protos

Universal structural substrate for the Protos family. This crate owns
the sole character reader (delineation) and the sole character writer
(canonical print). All dialects ride on it.

## Four layers

Text, Protoform, Concept, Corporal. Descent (realization) may fault;
ascent (textualization) cannot. Extents are found on the way in and
computed when printing; a Protoform carries no extent.

## Types

Intrinsic scalars: `Text` (String), `Integer` (i64), `Decimal` (f64),
`Boolean` (bool), `Symbol` (Text).

Structural types: `Extent`, `Path`, `Situation`, `Separator`,
`Enclosure`, `Boundary`, `Protoform` (Headed / Enclosed / Opaque / Bare),
`Delineation`, `Fault`, `Problem`, `Potential<T>`.

## Kinds (traits)

`Structural` (delineate), `Printing` (print), `Protosizable` (protosize),
`Conceptual<C>` (conceive), `Actualizable<T>` (actualize),
`Situating` (situate), `Embodied` (blanket bound).

## Ethos declaration

See `protos.ethos` at the repository root for the crate's own
declaration in the Library anatomy.

## Delineation rules

A single `;` opens a comment to end of line. Six structural delimiter
pairs: `{ }` `[ ]` `\u{00AB} \u{00BB}` `< >`. Two opaque boundary pairs:
`\u{201C} \u{201D}` (curly quotes, content-opaque) and `( )` (parentheses,
read by balance). A Bare is a maximal run of non-whitespace,
non-delimiter characters. Inside a run, a separator (`.` `!` `:`) splits
head from body when a character follows.

## Canonical print

Spaced: `{ a b }`, `[ a b ]`, `\u{00AB} k v \u{00BB}`. Empty: `{}`, `[]`, `\u{00AB}\u{00BB}`.
Angled always tight: `<a b>`. Head.body with separator glyph directly.
Opaque verbatim. Comments never printed. One line.

## Meaning

Parentheses bound Meaning in the datom dialect. The content is read by
balance: nested `( )` pairs are structure inside the Meaning. An
unbalanced `)` is escaped as `\)` on output.
