# Upgrades

## 0.14.0

`Portion` and its nested anatomy now implement `Clone`. Retained inbound
extents are valid provenance; a later writer projection recomputes output
extents. Use `Portion::from_signed_i64`, `from_decimal_f64`, and
`from_expected_string` for infallible integer or fallible decimal/String
outbound construction. The String constructor chooses one unquoted Portion or
validated balanced-curly opaque content; unbalanced curly content is rejected.

## 0.13.0

`Portion` implements `ScalarAnatomy`: `signed_i64()` accepts one canonical
signed-integer `Bare` Portion, while `decimal_f64()` accepts one finite,
point-mandatory Period-headed decimal. The methods return extent-bearing
`Fault`s for invalid spelling, integer range, and non-finite decimal values.
Use these rather than reading symbols in a dialect. `DelineatedText::retag()`
changes a writer-produced `Text` target type without re-delineating it.

## 0.12.0

`BareSafe::is_bare_safe()` is replaced by
`is_bare_safe_for(BareExpectation)`. Use `BareExpectation::Symbol` for the
former one-`Bare` rule. A dialect expecting a String uses
`BareExpectation::String`, which permits any exactly-one canonical `Portion`,
including headed forms such as `a.b`, `a!b`, and `a:b`, but rejects sibling text
such as `a b`. `PortionText::canonical_text()` recovers a delineated Portion's
canonical content without a dialect character scan.

## 0.11.0

`Text` is now `Text<T = ()>` and itself implements `Embodiable` for its typed
target; `Prospective<T>` is an alias for `Text<T>`. Replace a separate
prospective carrier with a typed `Text<T>` where the inbound type association
is useful.

`Enclosed` now separates `StructuralEnclosed` from `OpaqueEnclosed`. Structural
construction uses `(StructuralEnclosure, Vec<Portion>)`; opaque construction is
fallible through `OpaqueEnclosed::try_from((OpaqueBoundary, String))`. The old
single `Boundary`/contents construction path is gone. `EnclosedArity::arity()`
is computed from structural children, and opaque values have arity zero.

`Symbol` construction is fallible (`Symbol::try_from`), and public `Portion`
construction materializes its UTF-8 extents through the printer. This release
introduced Protos-owned bare-safety; 0.12 names its current context-aware API.
`Text::from` now projects valid input canonically: it drops `;;` comments and
spaces adjacent sibling Portions.

## 0.10.0

`Portion` is now the `Headed` / `Enclosed` / `Bare` union directly; each
variant carries its one `Extent`. Replace `Portion { extent, form }` and
`PortionForm` matches with `Portion::{Headed, Enclosed, Bare}` matches, and use
`AsRef<Extent>` where a common extent is needed.

`Boundary::Parentheses` is replaced by
`Boundary::Dialect(DialectBoundary::Parentheses)`. Parentheses remain
dialect-owned, and are not a sixth universal `Enclosure`. Parenthetical opaque
payloads use balanced parentheses; `\\` is a literal backslash and `\)` is an
unmatched literal close. Printing emits that canonical escaping.

## 0.8.0

`Shape` now includes `DottedBare`, written `Head.Unit` (for example,
`Observe.Locks`). Update exhaustive `Shape` matches. The scanner exposes the
prefix as `Block::head` and the suffix as `Block::body`; dialects still assign
the type-directed meaning of that block.

## 0.7.0

`Shape` now includes the headless `Guillemeted` structural block, written
`« … »`. Update exhaustive `Shape` matches to handle it. A dotted prefix is
not valid on this shape.
