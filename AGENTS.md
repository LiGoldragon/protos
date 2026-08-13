# Protos agent boundary

This repository holds the quick-new universal structural substrate.

Keep every Rust method under a trait. The trait and main-type roster precedes
implementation. `ShapeDefined` discriminates only: it exposes `shapes()` and
selects from Shape plus Head, while selected dialect types own their contexts.
`Realize` is text to real and is called on the textual type; `Textualize` is
real to text and is called on the real type.

Do not add Meaning, Signal, component, archive-identity, numeric-registry, or
legacy/frozen imports. The first scanner is lexical only; it owns universal
string carriers and opacity, never a dialect's interpretation.
