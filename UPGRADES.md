# Upgrades

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
