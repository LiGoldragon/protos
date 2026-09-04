# protos

Universal structural substrate for the Protos family. This crate owns
the sole character reader (delineation) and the sole character writer
(canonical print). All dialects ride on it.

## Four layers

Text, Protoform, Concept, Corporate. Descent may fault; ascent cannot.
Spans are found on the way in and computed on the way out.

## Types

Intrinsic scalars: `Text` (String), `Integer` (i64), `Decimal` (f64),
`Boolean` (bool), `Symbol` (Text).

Structural types: `Extent`, `Path`, `Situation`, `Separator`,
`Enclosure` (Braced, Bracketed, Angled), `Boundary` (CurlyQuotes,
Parentheses), `Head` (Bare, Qualified), `Protoform` (Headed, Enclosed,
Opaque, Bare), `Delineation`, `Fault`, `Problem`, `Potential<T>`,
`Situated<F>`.

## Kinds (traits)

`Textualizable` (textualize), `Protosizable` (protosize),
`Conceivable<C>` (conceive), `Incorporable<C>` (incorporate),
`Actualizable<T>` (actualize), `Situating` (situate).

## Delimiters

Five pairs in two families. Structural: `{ }` braces, `[ ]` brackets,
`< >` angles. Opaque: curly quotes (content-opaque) and `( )`
parentheses (read by balance). A single `;` opens a comment to end of
line; comments are never printed.

## Canonical print

Spaced: `{ a b }`, `[ a b ]`. Empty: `{}`, `[]`, `<>`. Angled always
tight: `<a b>`. Head.body with separator glyph directly. Opaque
verbatim. One line.
