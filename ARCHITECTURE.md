# protos architecture

`Text<T>` is the canonical text carrier. The only route from characters to
structure is `Text::delineate`; the only route from structure to characters is
`Printing::print`.

```text
input text --delineator--> Delineation[Portion] --printer--> canonical Text<T>
                         \                                  /
                          \-- dialect Embodied / Textualizable --/
```

`Portion` is its own inline union: `Headed(Extent, Headed)`,
`Enclosed(Extent, Enclosed)`, or `Bare(Extent, Bare)`. Extents are half-open
UTF-8 byte offsets and are always computed by the delineator or printer.
There is no separate form, block, shape, walk, normalizer, or second parser.

Structural enclosure and opaque content are distinct anatomy. `{}`, `[]`,
`«»`, and `<>` contain nested `Portion` values. Curly quotes contain balanced,
asymmetric opaque content with no escapes. Parentheses are represented as
`OpaqueBoundary::Dialect(DialectBoundary::Parentheses)`: their content is
opaque and balanced by the universal delimiter machinery, with canonical
escaping: `\\` for a literal backslash, `\(` for an unmatched opening
parenthesis, and `\)` for an unmatched closing parenthesis. This keeps
parentheses dialect-owned without creating a sixth universal `Enclosure`.

The shared delimiter and separator tables are read by both delineator and
printer. Comments and whitespace are non-structural trivia. Valid text is
canonically projected by delineation then printing, including a single space
between adjacent siblings. Invalid `Text` retains its source spelling so its
delineation reports a precise fault.

Dialects see no character-level API. They implement `Embodied` from received
`Portion` anatomy and `Textualizable` to produce a valid `Portion`. `Text<T>`
implements `Embodiable` when `T: Embodied`; `Prospective<T>` names the same
association. `BareSafe` answers with the expected context: `Symbol` requires
one `Bare` Portion, while `String` requires exactly one Portion and therefore
retains load-bearing separators. `PortionText::canonical_text` recovers that
Portion's canonical text through the writer, not a dialect character scan.
Multiple root siblings are unsafe in either context. `ShapeDefined` is only a
structural predicate.

`ScalarAnatomy` answers numeric expectations from existing Portion anatomy:
a signed integer is one `Bare` Portion, and a decimal is a Period-headed
Portion with a bare fractional body. The universal question checks canonical
sign/zero spelling, range, finite value, mandatory point, and absence of an
exponent; failure carries the Portion's computed UTF-8 extent. Dialects do not
read a `Symbol` or reconstruct a headed body. `DelineatedText::retag` moves a
printer-produced canonical `Text` to a dialect target type while retaining its
already-computed delineation, without a read.
