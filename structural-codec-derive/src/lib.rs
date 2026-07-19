//! # structural-codec-derive
//!
//! The generated-codec side of `structural-codec`'s conformance law 5, standalone.
//!
//! One `#[structural_form(...)]` authority is lowered THREE ways (the accepted
//! design decision 7): to the authoritative `structural_codec::StructuralEntry`
//! (the form as data — byte-identical to a hand-authored entry), to an optimized
//! `structural_codec::conformance::GeneratedCodec` (a straight-line,
//! type-specialized decode/encode that never consults the trusted evaluator), and
//! to the typed capture the codec fills. `structural-codec`'s `ConformanceHarness`
//! then proves the generated codec and the evaluator agree on all four outputs —
//! the Core value, the NameTable delta, the canonical output, and the typed error.
//!
//! This crate proves the pattern `nota-derive` will later absorb, collision-free
//! and without touching `nota`: the companion `structural-codec-derive-fixtures`
//! crate mirrors `core-schema`'s `FixtureFamily` entirely through this macro and
//! runs law 5 over it.
//!
//! ## What a user writes, and what is generated
//!
//! ```ignore
//! use structural_codec_derive::structural_form;
//!
//! #[structural_form(id = 10, leaf(Integer))]
//! pub struct Integer;
//!
//! #[structural_form(id = 1, newtype_declaration(inner = Integer, delimiter = Brace))]
//! pub struct CommitSequence;
//! ```
//!
//! Each attribute names a scoped Core-type `id` (in the fixture universe) and one
//! structural `kind`. The macro replaces the named placeholder with the typed
//! capture, an inherent `structural_entry()` returning the authoritative form data,
//! and a `GeneratedCodec` implementation. The five kinds — `leaf`, `delegate`,
//! `newtype_declaration`, `struct_declaration`, `field_meta` — cover the whole
//! fixture family; each mirrors one constructor shape of `structural-codec`'s kernel
//! algebra.

use proc_macro::TokenStream;

mod generate;
mod spec;

/// Lower one structural-form authority to its typed capture, its authoritative
/// `StructuralEntry`, and its optimized `GeneratedCodec`. See the crate docs for
/// the attribute grammar and the five supported kinds.
#[proc_macro_attribute]
pub fn structural_form(attribute: TokenStream, item: TokenStream) -> TokenStream {
    match spec::TypeSpec::parse(attribute.into(), item.into()) {
        Ok(specification) => specification.expand().into(),
        Err(error) => error.to_compile_error().into(),
    }
}
