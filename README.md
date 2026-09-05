# protos

The universal structural substrate. Protos is only about structure: it owns the
one character reader and the one character writer every dialect shares, and it
knows nothing of struct, vector, integer or string. What a structure means is
said by the dialect.

## Layers

Text, Protoform, Concept, Corporate. Text arrives as a `Potential` value and
descends; a corporate value ascends. A capability is named by the layer it goes
to.

| capability | kind | goes to | borne by |
|---|---|---|---|
| `protosize` | `Protosizable` | Protoform | `str`, `String` (the delineation; may fault); a dialect's concept (cannot) |
| `conceive` | `Conceivable<C>` | Concept | `Situated<Protoform>`, `Delineation` (in the dialect) |
| `incorporate` | `Incorporable<T>` | Corporate | the dialect's concept |
| `actualize` | `Actualizable<T>` | Corporate | `Potential<T, C>`: the whole descent |
| `textualize` | `Textualizable` | Text | `Protoform`, `Delineation`; the dialect's concept |
| `situate` | `Situating` | Text | `Protoform`: text and situation in one pass |

## Structure

`Protoform` is every unit of the text: `Headed` (a head, a separator, a body),
`Enclosed` (structures between `{ }`, `[ ]` or `< >`), `Opaque` (content between
`“ ”` or `( )`), `Bare` (a head alone). A `Head` is a `Symbol`, or a symbol
`Qualified` by constraints in angle brackets: `Vector<Text>`.

## Situation

Extents are not intrinsic to structures. A `Situation` is a tree parallel to the
structure: the structure's `Extent` and the situations of its children, in path
order. `Situated<T>` pairs a value with its situation; a `Delineation` is the
text's top-level structures, each situated. The reader finds the situation on
the way in; the writer computes it on the way out, from the offsets it writes
at. Memory is one situation node per structure: linear in the text.

The path convention, stated on `Pathed`: a headed structure's head is child 0
and its body child 1; an enclosure's children are in order; a qualified head's
constraints are the head's children. `Locating::locate` looks a situation up by
path.

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
- A run of symbols immediately followed by `<` qualifies its last symbol:
  `Vector<Text>`, `A<B>.{ 1 }`, `A<B>.C`.
- Every other adjacency yields siblings: `a..{ 1 }`, `a<b>c`, `a.{ 1 }.b`.
- Curly quotes are opaque to the first `”`; parentheses are read by balance,
  with `\(` `\)` `\\` unescaped.

Faults are structural only: `Unclosed`, `Unopened`, `Unterminated`, `Stray`,
each at its extent. There is no depth limit: every walk (read, write, drop) is
iterative with an explicit stack, so depth is bounded by the text's size.

## Writing

Canonical text: `{ a b }` and `[ a b ]` spaced, `{}` `[]` empty; `<a b>` tight;
`“x”` never spaced; `Head.body`; siblings one space apart; one line. Inside
parentheses only an unbalanced parenthesis and a backslash are escaped, so a
balanced inner pair is written verbatim. Writing cannot fault: `Text` is a
string that cannot carry `”`, refused at construction with a `Refusal` naming
the glyph and its offset.

## Anatomy

| module | what | kind |
|---|---|---|
| `anatomy` | the types of every layer | |
| `kinds` | the kinds | |
| `glyph` | each delimiter's glyphs; classification by walking the variants | `Glyphing`, `Delimiting`, `Serial`, `Classifying` |
| `text` | `Text` and its refusal | |
| `run` | a bare run split into its pieces | |
| `delineation` | the reader: frames, runs, heads, enclosures | `Protosizable` |
| `opaque` | the opaque regions: quotes, parentheses by balance | |
| `textualization` | the writer, with the situation | `Situating`, `Textualizable` |
| `situation` | lookup by path; iterative drop | `Locating` |
| `actualization` | `Potential` and the descent | `Actualizable` |
| `dropping` | iterative drop of the protoform tree | |

No free functions, no inherent impls, no zero-sized bearers, no variant rosters:
`nix flake check` carries the guards, with build, test, fmt, clippy and doc.
