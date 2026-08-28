# Upgrades

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
construction materializes its UTF-8 extents through the printer. Use
`Text::is_bare_safe()` for a dialect's bare-safety question instead of scanning
characters. `Text::from` now projects valid input canonically: it drops `;;`
comments and spaces adjacent sibling Portions.

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
