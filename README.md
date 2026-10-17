# protos

The universal structural substrate. Protos is only about structure: it owns the
one character reader and the one character writer every dialect shares, and it
knows nothing of struct, vector, integer or string. What a structure means is
said by the dialect.

## Layers

Text, Protos, Concept, Composition. Text descends into structure and a
composition ascends back to text. A capability is named by the layer it goes to.

| capability | kind | goes to | borne by |
|---|---|---|---|
| `protosize` | `Protosizable` | Protos | `str`, `String` (may error); a dialect's concept (cannot) |
| `protosize_with` | `BoundedProtosizable` | Protos | `str`, `String`, under a caller's `ReaderBudget`, which is `Spendable` |
| `textualize` | `Textualizable` | Text, expanded vertically | `Protos`; a dialect's concept |
| `compact` | `Compactable` | Text on one line | `Protos` |
| `canonicalize` | `Canonicalizable` | — | `Protos`: assign the extents of the text it will print |

Descent may error; ascent cannot.

## Structure

`Protos` is every unit of the text: `Headed` (a head, separator and body),
`Enclosed` (structures between `{ }`, `[ ]`, `< >` or, on the proposal branch, `⟦ ⟧` and `⦃ ⦄`), `Opaque` (content between
`« »` or `( )`), and `Bare` (a run alone). Every node carries its own `Extent`,
the fact of where it sits in the text — read in by the reader, or assigned by
`canonicalize` to a tree that was built rather than read.

A `Headed` node has a `Symbol` head and an optional extent-bearing angled
`Enclosed` node in `constraints`. That holds the anatomy of
`Processable<[ Clonable Sendable ] Serializable>.[ ... ]` with no type or kind
meaning. `Vector<String>` with no heading separator after it stays the adjacent
nodes `Bare("Vector")` and an angled `Enclosed`, for its conceptual reader to
relate.

One text is one structure: a second structure at the top level is a `Multiple`
error, and an empty text is `Empty`.

## Reading

Whitespace separates; `;` opens a comment to the end of the line. A bare run is
a maximal run of plain and separator (`.` `!` `:`) glyphs; a symbol is a
non-empty run with no separator.

- A run is a chain (right-associative `Headed`) only when every segment between
  its separators is a symbol: `a:b:c`, `Some.42`. Any other run is one bare
  word: `a.`, `.a`, `a..b`, `2026-09-03`.
- A run ending in exactly one separator, immediately followed by an opener,
  opens that structure as the chain's body: `Reviewer.{ 2024 17 }`,
  `Observed.Locks.[]`, `Some.(x)`.
- A run ending in exactly one separator, followed by any whitespace (spaces,
  tabs, line breaks, comments) and then a structure, takes that structure as
  its body: `Observed.\n   Locks.\n      []` reads as `Observed.Locks.[]`, and
  `Library.\n{ … }` as `Library.{ … }`. With nothing after it but a closer or
  the end of the text, the run stays one bare word (`[ a. ]`). (Proposal
  branch `ethos-proposals`; on main such a run is always a bare word.)
- A run of symbols immediately followed by `<` constrains its head:
  `A<B>.{ 1 }`, `A<B>.C`.
- Every other adjacency is a second structure: `a..{ 1 }`, `a<b>c`,
  `a.{ 1 }.b`, `Vector<Text>` alone.

Errors are structural only and carry their extent. The reader has a fixed
structural-depth boundary independent of its public node budget, so a read tree
is never deeper than that boundary; a built tree may be as deep as what built
it, so every traversal of `Protos` walks an explicit stack. Printing, showing
and canonical measurement are one machine over one step type, differing only in
what a node renders into and where the pieces go; cloning, comparing and
dropping are one statement of what a node holds besides its children.

## Opaque regions

Guillemets and parentheses are opaque: every glyph inside is content. A
backslash escapes the boundary's own glyphs and itself, and nothing else — `\X`
for any other X is a literal backslash followed by X.

| boundary | escapable | nests |
|---|---|---|
| `« »` | `\\` `\»` | no, so every `»` inside is escaped |
| `( )` | `\\` `\(` `\)` | yes, so only an unbalanced parenthesis is escaped |

Escaping is minimal: the writer prefixes a backslash only where leaving the
glyph bare would read back as something else, so `«C:\Users\ada»` and
`(a (b) c)` are written verbatim while `«a\»` and `(a\)` escape the backslash
that would otherwise have consumed the closer. Every content round-trips —
built, printed, and read back to the same structure.

## Writing

The canonical text expands vertically. A `{ }` or `[ ]` enclosure with more
than one element, one of which has a next layer (headed, or a non-empty brace
or bracket enclosure), opens on its line and its elements hang beneath the
first, aligned; the closer ends the last element's line. Elements that are all
leaves sit on one line, and so does a lone element with a next layer:

```
Locked.{ 442
         MyLock
         [ /abs/path ]
         «why I hold it» }
[ flow:[ FlowId Voice Event ] ]
```

`{ a b }` and `[ a b ]` spaced, `{}` `[]` empty; `<a b>` tight, and tight
against the element it follows (`Vector<Event>`); `Head.body` with nothing
around the separator; opaque regions verbatim with their glyphs. `textualize`
writes it, and `canonicalize` assigns its extents. `compact` (`Compactable`)
writes the same structure on one line, siblings one space apart, for a reader
that takes one line at a time. Writing cannot error.

## Anatomy

| module | what | kind |
|---|---|---|
| `core` | the types and the reader | `Protosizable`, `BoundedProtosizable`, `Spendable`, `Escaping`, `Glyphing` |
| `rendering` | one stack machine for printing, compacting, showing and canonical extents | `Textualizable`, `Compactable`, `Canonicalizable`; `Rendering`, `Sinking`, `Settling` |
| `layout` | the vertical layout: the current column, elements, next layers | `Advancing`, `Grouping`, `Layered` |
| `traversing` | iterative `Clone`, `PartialEq` and `Drop` | `Structuring` |

No free functions, no inherent impls, no zero-sized bearers, no variant rosters:
`nix flake check` carries the guards, with build, test, fmt, clippy and doc.

The `rkyv` feature archives `Extent`, `Separator`, `Error` and `Problem`, the
types a generated ethos contract may hold in a position: every generated type
archives, so a contract holding them crosses a wire. The `archival` check runs
`tests/archival.rs` under it.

`protos.ethos` and `protos-kinds.ethos` state the datom anatomy of the public
types and kinds. The crate is hand-written because Ethos cannot yet state what
it needs: `usize` extents, `char` payloads, borrowed receivers, and the
iterative `Clone`, `PartialEq` and `Drop` of `traversing`; and a generated
contract of `protos.ethos` would derive datom-codec's kinds, which depends on
this crate. The kinds file declares only kinds, so its Rust is generated into
`generated/protos-kinds.rs` and committed: `tests/kinds.rs` compiles it against
this crate's types, each associated type bearing the generated kind through the
hand-written one, and the `generated-kinds` Nix check regenerates it with the
pinned ethos-zero and holds the committed file to it; `checked-anatomy` checks
`protos.ethos` with the same ethos-zero. A capability's input is a
kind (`protosize_with` takes a `Spendable`), never a concrete type.
