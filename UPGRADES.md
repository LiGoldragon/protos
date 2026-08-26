# Upgrades

## 0.8.0

`Shape` now includes `DottedBare`, written `Head.Unit` (for example,
`Observe.Locks`). Update exhaustive `Shape` matches. The scanner exposes the
prefix as `Block::head` and the suffix as `Block::body`; dialects still assign
the type-directed meaning of that block.

## 0.7.0

`Shape` now includes the headless `Guillemeted` structural block, written
`« … »`. Update exhaustive `Shape` matches to handle it. A dotted prefix is
not valid on this shape.
