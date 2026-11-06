# Upgrades

## 0.32.0

Breaking: the canonical print expands vertically.

- `Textualizable::textualize` writes the vertical print: a `{ }` or `[ ]`
  enclosure with more than one element, one of which has a next layer, hangs
  its elements aligned beneath the first, the closer on the last line. An
  angled enclosure is written tight against the element it follows
  (`Vector<Event>`, formerly `Vector <Event>`). Text that held no such
  enclosure is unchanged.
- `Canonicalizable::canonicalize` assigns the extents of that vertical text.
- The former one-line print is `Compactable::compact`. A consumer that needs one
  line (a line-oriented log, a reply matched as a single line) replaces
  `.textualize()` with `.compact()` and imports `protos::Compactable`.
- `ReaderBudgeting` (never exported) is the exported kind `Spendable`;
  `ReaderBudget` bears it.

To deploy: repin protos, then run the consumer's tests. Every expected text
that asserted a one-line print of a nested structure fails; either update it
to the vertical print or call `compact`. Reading is unchanged: both prints read
back to the same structure.

## 0.32.1

An angled enclosure is written tight after a headed element too
(`Generated.Vector<String>`), unless the element's last leaf ends on a
separator, where tight angles would read back as a head's constraints. No
reading changes; deploy by repinning.

## 0.33.0

Breaking: a guillemet string is read by balance, as a parenthesized region is: a « »
pair inside it is content, nesting to any depth, and the string closes at the
guillemet that balances the one that opened it. `\«` is now an escape, as `\»`
is. The writer leaves balanced pairs bare and escapes only an unbalanced
guillemet, so content `a » b` is still written `«a \» b»`, content `a « b` is
now written `«a \« b»`, and `«a «b» c»` reads as content `a «b» c`.

To deploy: repin protos, then run the consumer's tests. Text that held a bare
unbalanced `«` inside a guillemet string, or a literal `\«`, now reads
differently; such text is rewritten with the escape.
