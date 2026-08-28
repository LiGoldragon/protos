# protos

`protos` is the universal structural substrate for Protos-family dialects. It
owns the only character reader (`Delineatable`) and the only character writer
(`Printing`). A dialect receives already-delineated `Portion` anatomy and
supplies its own embodied type.

`Text<T>` is canonical Protos text associated with the type it can embody.
`Prospective<T>` is its compatibility name. Constructing `Text` projects valid
input through the same delineator and printer: trivia and `;;` comments are
dropped, and adjacent root or structural siblings receive one space.

`Portion` is directly one of `Headed`, `Enclosed`, or `Bare`, and every value
owns one half-open UTF-8 byte `Extent`. Public construction is structural:
`Symbol` is fallible, `Bare`, `Headed`, and structural enclosures materialize
their extents through the writer. Opaque enclosures are split from structural
ones, so their boundary/content combinations are checked through the same
reader/writer pipeline. `EnclosedArity` computes arity from structural
children; no stale arity is stored.

The universal structural enclosures are `{}`, `[]`, `«»`, and `<>`; curly
quotes `“”` are opaque, balanced universal content. Parentheses are the
dialect-owned opaque boundary, handled by the same delimiter table rather than
becoming a sixth `Enclosure`. Parenthetical content has canonical escapes:
`\\` is a literal backslash, `\(` is an unmatched literal opening parenthesis,
and `\)` is an unmatched literal closing parenthesis. Curly quotes balance
asymmetrically and have no escapes.

Dialects never rescan strings to decide whether something is a bare value. Ask
`Text::is_bare_safe_for(BareExpectation::Symbol)` for a symbol, or
`BareExpectation::String` for any text that delineates to exactly one canonical
`Portion`. The latter accepts load-bearing separators such as `a.b`, `a!b`, and
`a:b`; it rejects multiple sibling portions such as `a b`. Given a delineated
portion, `PortionText::canonical_text` returns its canonical text without a
dialect character scan. `ShapeDefined` is a predicate over received anatomy,
not another parser. `Textualizable` owns infallible outbound `Portion` anatomy;
Protos owns its printing.

`ScalarAnatomy` is the corresponding universal numeric question over an
already delineated Portion: `signed_i64` accepts canonical signed integers;
`decimal_f64` accepts finite, point-mandatory Period-headed decimals without an
exponent. It returns an extent-bearing Protos `Fault` for malformed, out-of-
range, or non-finite input. A printer result can be retagged as `Text<T>` via
`DelineatedText::retag` without another delineator pass; it preserves the
writer-computed delineation.

## Validation

```sh
cargo test --locked
cargo clippy --all-targets -- -D warnings
nix flake check -L
```
