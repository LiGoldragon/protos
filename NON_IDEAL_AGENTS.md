# protos — known non-idealities

No known non-idealities are currently recorded.

The `structural-codec-derive` `field_meta` drift (it emitted a stale explicit
`name.Type` field-name constructor alongside the elided form) was resolved: the derive
now emits ONLY the elided form, matching `core-schema`'s one-constructor authored
`Field` entry, per the total field-name ban (psyche ruling 2026-07-19). The
derived-vs-authored law-5 cross-check is seated downstream in `core-schema`, where the
machinery and the derive resolve at one protos pin and the entry types unify.
