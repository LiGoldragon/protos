//! Lowering the one authority three ways. `TypeSpec::expand` turns a parsed
//! `#[structural_form(...)]` authority into: the authoritative `StructuralEntry`
//! constructor (form-as-data, byte-identical to a hand-authored entry), the
//! optimized `GeneratedCodec` (a straight-line, type-specialized walk that never
//! consults the evaluator), and the typed capture the codec fills. Every emitted
//! codec is a fast path the conformance harness proves equal to the evaluator.
//!
//! All generation is expressed as methods on the spec's own data-bearing types,
//! so the verbs live on the nouns that carry the meaning.

use proc_macro2::{Literal, TokenStream};
use quote::quote;

use crate::spec::{Delimiter, Kind, ScalarKind, TypeSpec};

impl ScalarKind {
    /// The native Rust field the typed capture stores.
    fn native(self) -> TokenStream {
        match self {
            Self::Integer => quote! { i64 },
            Self::Float => quote! { f64 },
            Self::Text => quote! { ::std::string::String },
            Self::Boolean => quote! { bool },
        }
    }

    /// The `ScalarLeaf` variant the authoritative form carries.
    fn leaf_variant(self) -> TokenStream {
        match self {
            Self::Integer => quote! { Integer },
            Self::Float => quote! { Float },
            Self::Text => quote! { Text },
            Self::Boolean => quote! { Boolean },
        }
    }

    /// Build the mirror `ScalarValue` from a `self.value` access, inverting the
    /// flatten-then-parse so the evaluator and this codec render the same block.
    fn scalar_value(self, access: &TokenStream) -> TokenStream {
        match self {
            Self::Integer => quote! { ::structural_codec::ScalarValue::Integer(#access) },
            Self::Float => quote! { ::structural_codec::ScalarValue::Float(#access) },
            Self::Text => quote! { ::structural_codec::ScalarValue::Text(#access.clone()) },
            Self::Boolean => quote! { ::structural_codec::ScalarValue::Boolean(#access) },
        }
    }

    /// The decode body: flatten to text, then parse to the native value.
    fn decode_value(self) -> TokenStream {
        match self {
            Self::Integer => quote! {
                let value = text.parse::<i64>().map_err(|error: ::std::num::ParseIntError| {
                    ::structural_codec::DecodeError::ScalarParse(error.to_string())
                })?;
            },
            Self::Float => quote! {
                let value = text.parse::<f64>().map_err(|error: ::std::num::ParseFloatError| {
                    ::structural_codec::DecodeError::ScalarParse(error.to_string())
                })?;
            },
            Self::Text => quote! {
                let value = text;
            },
            Self::Boolean => quote! {
                let value = match text.as_str() {
                    "true" => true,
                    "false" => false,
                    other => {
                        return ::core::result::Result::Err(
                            ::structural_codec::DecodeError::ScalarParse(::std::format!(
                                "not a boolean keyword: {other}"
                            )),
                        );
                    }
                };
            },
        }
    }
}

impl Delimiter {
    /// The `raw_discovery::Delimiter` this maps to.
    fn tokens(self) -> TokenStream {
        match self {
            Self::Parenthesis => quote! { ::raw_discovery::Delimiter::Parenthesis },
            Self::SquareBracket => quote! { ::raw_discovery::Delimiter::SquareBracket },
            Self::Brace => quote! { ::raw_discovery::Delimiter::Brace },
        }
    }

    /// The human name used in a mismatch diagnostic.
    fn description(self) -> &'static str {
        match self {
            Self::Parenthesis => "parenthesis",
            Self::SquareBracket => "square bracket",
            Self::Brace => "brace",
        }
    }
}

impl TypeSpec {
    /// The scoped Core-type id expression for this type, in the fixture universe.
    fn core_type(&self) -> TokenStream {
        let id = Literal::u32_unsuffixed(self.id);
        quote! { ::structural_codec::ids::ScopedCoreTypeId::fixture(#id) }
    }

    /// A discovered block's kind name, for a mismatch diagnostic's `found` field.
    fn block_kind(expression: &TokenStream) -> TokenStream {
        quote! {
            match #expression {
                ::raw_discovery::Block::Atom(_) => "atom",
                ::raw_discovery::Block::Application { .. } => "application",
                ::raw_discovery::Block::Delimited { .. } => "delimited",
                ::raw_discovery::Block::PipeText(_) => "pipe text",
            }
        }
    }

    /// Assemble the common output: the typed capture item, its inherent
    /// `structural_entry`/`decode_within`, and the `GeneratedCodec` impl.
    fn assemble(
        &self,
        item: TokenStream,
        entry_body: TokenStream,
        decode_within: TokenStream,
        encode_fn: TokenStream,
        to_structural_body: TokenStream,
    ) -> TokenStream {
        let attrs = &self.attrs;
        let name = &self.name;
        let core_type = self.core_type();
        quote! {
            #[derive(Clone, Debug, PartialEq)]
            #(#attrs)*
            #item

            impl #name {
                /// The authoritative `StructuralEntry` for this type — the same
                /// form-as-data a table author writes by hand, generated from the
                /// single form authority.
                pub fn structural_entry() -> ::structural_codec::StructuralEntry {
                    #entry_body
                }

                #decode_within
            }

            impl ::structural_codec::conformance::GeneratedCodec for #name {
                const CORE_TYPE: ::structural_codec::ids::ScopedCoreTypeId = #core_type;

                fn decode(
                    block: &::raw_discovery::Block,
                    names: &mut ::name_table::NameTable,
                ) -> ::core::result::Result<Self, ::structural_codec::DecodeError> {
                    names.try_intern(|transaction| Self::decode_within(block, transaction))
                }

                #encode_fn

                fn to_structural(&self) -> ::structural_codec::StructuralValue {
                    #to_structural_body
                }
            }
        }
    }

    /// Lower the authority to the full generated output.
    pub fn expand(&self) -> TokenStream {
        match &self.kind {
            Kind::Leaf(scalar) => self.expand_leaf(*scalar),
            Kind::Delegate { inner } => self.expand_delegate(inner),
            Kind::NewtypeDeclaration { inner, delimiter } => self.expand_newtype(inner, *delimiter),
            Kind::StructDeclaration {
                field_type,
                delimiter,
                fields,
            } => self.expand_struct(field_type, *delimiter, fields),
            Kind::FieldMeta => self.expand_field_meta(),
        }
    }

    fn expand_leaf(&self, scalar: ScalarKind) -> TokenStream {
        let name = &self.name;
        let vis = &self.visibility;
        let core_type = self.core_type();
        let native = scalar.native();
        let leaf_variant = scalar.leaf_variant();
        let decode_value = scalar.decode_value();
        let scalar_value = scalar.scalar_value(&quote! { self.value });

        let item = quote! { #vis struct #name { value: #native } };

        let entry_body = quote! {
            let core_type = #core_type;
            let form = ::structural_codec::StructuralForm::Leaf(
                ::structural_codec::LeafForm::scalar(::structural_codec::ScalarLeaf::#leaf_variant),
            );
            ::structural_codec::StructuralEntry::new(
                core_type,
                ::std::vec![::structural_codec::ConstructorCodec::new(
                    ::structural_codec::ids::CoreConstructorId::new(core_type, 0),
                    ::std::vec![form.clone()],
                    form,
                    ::structural_codec::ids::PositionalSignature::default(),
                )],
            )
        };

        let decode_within = quote! {
            fn decode_within<Interner: ::name_table::NameInterner + ?Sized>(
                block: &::raw_discovery::Block,
                _interner: &mut Interner,
            ) -> ::core::result::Result<Self, ::structural_codec::DecodeError> {
                let text = block
                    .dotted_text()
                    .ok_or(::structural_codec::DecodeError::LeafNotFlattenable)?;
                #decode_value
                ::core::result::Result::Ok(Self { value })
            }
        };

        let encode_fn = quote! {
            fn encode(
                &self,
                _resolver: &dyn ::name_table::NameResolver,
            ) -> ::core::result::Result<::raw_discovery::Block, ::structural_codec::EncodeError> {
                ::core::result::Result::Ok(#scalar_value.render_block())
            }
        };

        let to_structural_body = quote! {
            ::structural_codec::StructuralValue::chosen(
                0,
                ::structural_codec::StructuralValue::Scalar(#scalar_value),
            )
        };

        self.assemble(
            item,
            entry_body,
            decode_within,
            encode_fn,
            to_structural_body,
        )
    }

    fn expand_delegate(&self, inner: &syn::Path) -> TokenStream {
        let name = &self.name;
        let vis = &self.visibility;
        let core_type = self.core_type();

        let item = quote! { #vis struct #name(#inner); };

        let entry_body = quote! {
            let core_type = #core_type;
            let inner = <#inner as ::structural_codec::conformance::GeneratedCodec>::CORE_TYPE;
            let form = ::structural_codec::StructuralForm::Delegate(inner);
            ::structural_codec::StructuralEntry::new(
                core_type,
                ::std::vec![::structural_codec::ConstructorCodec::new(
                    ::structural_codec::ids::CoreConstructorId::new(core_type, 0),
                    ::std::vec![form.clone()],
                    form,
                    ::structural_codec::ids::PositionalSignature::new(::std::vec![inner]),
                )],
            )
        };

        let decode_within = quote! {
            fn decode_within<Interner: ::name_table::NameInterner + ?Sized>(
                block: &::raw_discovery::Block,
                interner: &mut Interner,
            ) -> ::core::result::Result<Self, ::structural_codec::DecodeError> {
                ::core::result::Result::Ok(Self(<#inner>::decode_within(block, interner)?))
            }
        };

        let encode_fn = quote! {
            fn encode(
                &self,
                resolver: &dyn ::name_table::NameResolver,
            ) -> ::core::result::Result<::raw_discovery::Block, ::structural_codec::EncodeError> {
                ::structural_codec::conformance::GeneratedCodec::encode(&self.0, resolver)
            }
        };

        let to_structural_body = quote! {
            ::structural_codec::StructuralValue::chosen(
                0,
                ::structural_codec::StructuralValue::Delegated(::std::boxed::Box::new(
                    ::structural_codec::conformance::GeneratedCodec::to_structural(&self.0),
                )),
            )
        };

        self.assemble(
            item,
            entry_body,
            decode_within,
            encode_fn,
            to_structural_body,
        )
    }

    fn expand_newtype(&self, inner: &syn::Path, delimiter: Delimiter) -> TokenStream {
        let name = &self.name;
        let vis = &self.visibility;
        let core_type = self.core_type();
        let delimiter_tokens = delimiter.tokens();
        let delimiter_description = delimiter.description();
        let block_kind = Self::block_kind(&quote! { block });
        let head_kind = Self::block_kind(&quote! { head });
        let payload_kind = Self::block_kind(&quote! { payload });

        let item = quote! {
            #vis struct #name {
                object: ::name_table::Identifier,
                inner: ::name_table::Identifier,
            }
        };

        let entry_body = quote! {
            let core_type = #core_type;
            let inner = <#inner as ::structural_codec::conformance::GeneratedCodec>::CORE_TYPE;
            let form = ::structural_codec::authoring::AuthoringForm::ObjectPrefixed(
                ::structural_codec::authoring::ObjectSymbolPrefixedBlock {
                    object: ::structural_codec::AtomForm::with_case(
                        ::structural_codec::CaseExpectation::PascalCase,
                    ),
                    delimiter: #delimiter_tokens,
                    sequence: ::structural_codec::SequenceForm::Product(::std::vec![
                        ::structural_codec::StructuralForm::pascal_atom()
                    ]),
                },
            )
            .normalize();
            ::structural_codec::StructuralEntry::new(
                core_type,
                ::std::vec![::structural_codec::ConstructorCodec::new(
                    ::structural_codec::ids::CoreConstructorId::new(core_type, 0),
                    ::std::vec![form.clone()],
                    form,
                    ::structural_codec::ids::PositionalSignature::new(::std::vec![inner]),
                )],
            )
        };

        let decode_within = quote! {
            fn decode_within<Interner: ::name_table::NameInterner + ?Sized>(
                block: &::raw_discovery::Block,
                interner: &mut Interner,
            ) -> ::core::result::Result<Self, ::structural_codec::DecodeError> {
                let (head, payload) = block.as_application().ok_or_else(|| {
                    ::structural_codec::DecodeError::BlockKindMismatch {
                        expected: "application",
                        found: #block_kind,
                    }
                })?;
                let object_atom = head.atom().ok_or_else(|| {
                    ::structural_codec::DecodeError::BlockKindMismatch {
                        expected: "atom",
                        found: #head_kind,
                    }
                })?;
                if ::raw_discovery::AtomCase::of(object_atom) != ::raw_discovery::AtomCase::PascalCase {
                    return ::core::result::Result::Err(::structural_codec::DecodeError::CaseMismatch);
                }
                let children = payload.as_delimited(#delimiter_tokens).ok_or_else(|| {
                    ::structural_codec::DecodeError::BlockKindMismatch {
                        expected: #delimiter_description,
                        found: #payload_kind,
                    }
                })?;
                if children.len() != 1 {
                    return ::core::result::Result::Err(
                        ::structural_codec::DecodeError::ProductArity { form: 1, blocks: children.len() },
                    );
                }
                let inner_block = &children[0];
                let inner_atom = inner_block.atom().ok_or_else(|| {
                    ::structural_codec::DecodeError::BlockKindMismatch {
                        expected: "atom",
                        found: "other",
                    }
                })?;
                if ::raw_discovery::AtomCase::of(inner_atom) != ::raw_discovery::AtomCase::PascalCase {
                    return ::core::result::Result::Err(::structural_codec::DecodeError::CaseMismatch);
                }
                // Shape fully validated; only now intern, in the evaluator's DFS
                // order: the object name first, then the wrapped-type name.
                let object = interner.intern(::name_table::Name::new(object_atom.text()))?;
                let inner = interner.intern(::name_table::Name::new(inner_atom.text()))?;
                ::core::result::Result::Ok(Self { object, inner })
            }
        };

        let encode_fn = quote! {
            fn encode(
                &self,
                resolver: &dyn ::name_table::NameResolver,
            ) -> ::core::result::Result<::raw_discovery::Block, ::structural_codec::EncodeError> {
                let object = resolver.resolve(self.object)?.as_str().to_owned();
                let inner = resolver.resolve(self.inner)?.as_str().to_owned();
                ::core::result::Result::Ok(::raw_discovery::Block::Application {
                    head: ::std::boxed::Box::new(::raw_discovery::Block::Atom(
                        ::raw_discovery::Atom::new(object),
                    )),
                    payload: ::std::boxed::Box::new(::raw_discovery::Block::Delimited {
                        delimiter: #delimiter_tokens,
                        root_objects: ::std::vec![::raw_discovery::Block::Atom(
                            ::raw_discovery::Atom::new(inner),
                        )],
                    }),
                })
            }
        };

        let to_structural_body = quote! {
            ::structural_codec::StructuralValue::chosen(
                0,
                ::structural_codec::StructuralValue::Application(
                    ::std::boxed::Box::new(::structural_codec::StructuralValue::Atom(self.object)),
                    ::std::boxed::Box::new(::structural_codec::StructuralValue::Delimited(::std::vec![
                        ::structural_codec::StructuralValue::Atom(self.inner)
                    ])),
                ),
            )
        };

        self.assemble(
            item,
            entry_body,
            decode_within,
            encode_fn,
            to_structural_body,
        )
    }

    fn expand_struct(
        &self,
        field_type: &syn::Path,
        delimiter: Delimiter,
        fields: &[syn::Path],
    ) -> TokenStream {
        let name = &self.name;
        let vis = &self.visibility;
        let core_type = self.core_type();
        let delimiter_tokens = delimiter.tokens();
        let delimiter_description = delimiter.description();
        let arity = fields.len();
        let arity_lit = Literal::usize_unsuffixed(arity);
        let block_kind = Self::block_kind(&quote! { block });
        let head_kind = Self::block_kind(&quote! { head });
        let payload_kind = Self::block_kind(&quote! { payload });
        let field_signatures = fields.iter().map(|field| {
            quote! { <#field as ::structural_codec::conformance::GeneratedCodec>::CORE_TYPE }
        });

        let item = quote! {
            #vis struct #name {
                object: ::name_table::Identifier,
                fields: ::std::vec::Vec<#field_type>,
            }
        };

        let entry_body = quote! {
            let core_type = #core_type;
            let field_type = <#field_type as ::structural_codec::conformance::GeneratedCodec>::CORE_TYPE;
            let form = ::structural_codec::StructuralForm::application(
                ::structural_codec::StructuralForm::pascal_atom(),
                ::structural_codec::StructuralForm::Delimited {
                    delimiter: #delimiter_tokens,
                    sequence: ::structural_codec::SequenceForm::Product(::std::vec![
                        ::structural_codec::StructuralForm::Delegate(field_type); #arity_lit
                    ]),
                },
            );
            let signature = ::structural_codec::ids::PositionalSignature::new(::std::vec![
                #(#field_signatures),*
            ]);
            ::structural_codec::StructuralEntry::new(
                core_type,
                ::std::vec![::structural_codec::ConstructorCodec::new(
                    ::structural_codec::ids::CoreConstructorId::new(core_type, 0),
                    ::std::vec![form.clone()],
                    form,
                    signature,
                )],
            )
        };

        let decode_within = quote! {
            fn decode_within<Interner: ::name_table::NameInterner + ?Sized>(
                block: &::raw_discovery::Block,
                interner: &mut Interner,
            ) -> ::core::result::Result<Self, ::structural_codec::DecodeError> {
                let (head, payload) = block.as_application().ok_or_else(|| {
                    ::structural_codec::DecodeError::BlockKindMismatch {
                        expected: "application",
                        found: #block_kind,
                    }
                })?;
                let object_atom = head.atom().ok_or_else(|| {
                    ::structural_codec::DecodeError::BlockKindMismatch {
                        expected: "atom",
                        found: #head_kind,
                    }
                })?;
                if ::raw_discovery::AtomCase::of(object_atom) != ::raw_discovery::AtomCase::PascalCase {
                    return ::core::result::Result::Err(::structural_codec::DecodeError::CaseMismatch);
                }
                let children = payload.as_delimited(#delimiter_tokens).ok_or_else(|| {
                    ::structural_codec::DecodeError::BlockKindMismatch {
                        expected: #delimiter_description,
                        found: #payload_kind,
                    }
                })?;
                if children.len() != #arity_lit {
                    return ::core::result::Result::Err(
                        ::structural_codec::DecodeError::ProductArity {
                            form: #arity_lit,
                            blocks: children.len(),
                        },
                    );
                }
                let object = interner.intern(::name_table::Name::new(object_atom.text()))?;
                let mut fields = ::std::vec::Vec::with_capacity(#arity_lit);
                for child in children {
                    fields.push(<#field_type>::decode_within(child, interner)?);
                }
                ::core::result::Result::Ok(Self { object, fields })
            }
        };

        let encode_fn = quote! {
            fn encode(
                &self,
                resolver: &dyn ::name_table::NameResolver,
            ) -> ::core::result::Result<::raw_discovery::Block, ::structural_codec::EncodeError> {
                let object = resolver.resolve(self.object)?.as_str().to_owned();
                let mut root_objects = ::std::vec::Vec::with_capacity(self.fields.len());
                for field in &self.fields {
                    root_objects.push(
                        ::structural_codec::conformance::GeneratedCodec::encode(field, resolver)?,
                    );
                }
                ::core::result::Result::Ok(::raw_discovery::Block::Application {
                    head: ::std::boxed::Box::new(::raw_discovery::Block::Atom(
                        ::raw_discovery::Atom::new(object),
                    )),
                    payload: ::std::boxed::Box::new(::raw_discovery::Block::Delimited {
                        delimiter: #delimiter_tokens,
                        root_objects,
                    }),
                })
            }
        };

        let to_structural_body = quote! {
            ::structural_codec::StructuralValue::chosen(
                0,
                ::structural_codec::StructuralValue::Application(
                    ::std::boxed::Box::new(::structural_codec::StructuralValue::Atom(self.object)),
                    ::std::boxed::Box::new(::structural_codec::StructuralValue::Delimited(
                        self.fields
                            .iter()
                            .map(|field| ::structural_codec::StructuralValue::Delegated(
                                ::std::boxed::Box::new(
                                    ::structural_codec::conformance::GeneratedCodec::to_structural(field),
                                ),
                            ))
                            .collect(),
                    )),
                ),
            )
        };

        self.assemble(
            item,
            entry_body,
            decode_within,
            encode_fn,
            to_structural_body,
        )
    }

    fn expand_field_meta(&self) -> TokenStream {
        let name = &self.name;
        let vis = &self.visibility;
        let core_type = self.core_type();
        let block_kind = Self::block_kind(&quote! { block });

        // Field names are illegal in every Protos surface (psyche ruling 2026-07-19:
        // field names are COMPLETELY illegal everywhere), so a field is nothing but
        // the bare `Type` standing at its position. There is ONE constructor — the
        // elided form — and the explicit `name.Type` application no longer parses.
        let item = quote! {
            #vis struct #name {
                type_name: ::name_table::Identifier,
            }
        };

        let entry_body = quote! {
            let core_type = #core_type;
            let type_only = ::structural_codec::StructuralForm::pascal_atom();
            ::structural_codec::StructuralEntry::new(
                core_type,
                ::std::vec![::structural_codec::ConstructorCodec::new(
                    ::structural_codec::ids::CoreConstructorId::new(core_type, 0),
                    ::std::vec![type_only.clone()],
                    type_only,
                    ::structural_codec::ids::PositionalSignature::default(),
                )],
            )
        };

        let decode_within = quote! {
            fn decode_within<Interner: ::name_table::NameInterner + ?Sized>(
                block: &::raw_discovery::Block,
                interner: &mut Interner,
            ) -> ::core::result::Result<Self, ::structural_codec::DecodeError> {
                // The sole constructor: a bare PascalCase atom (name elided, derived
                // from the type). An explicit `name.Type` application is illegal.
                let atom = block.atom().ok_or_else(|| {
                    ::structural_codec::DecodeError::BlockKindMismatch {
                        expected: "atom",
                        found: #block_kind,
                    }
                })?;
                if ::raw_discovery::AtomCase::of(atom) != ::raw_discovery::AtomCase::PascalCase {
                    return ::core::result::Result::Err(::structural_codec::DecodeError::CaseMismatch);
                }
                let type_name = interner.intern(::name_table::Name::new(atom.text()))?;
                ::core::result::Result::Ok(Self { type_name })
            }
        };

        let encode_fn = quote! {
            fn encode(
                &self,
                resolver: &dyn ::name_table::NameResolver,
            ) -> ::core::result::Result<::raw_discovery::Block, ::structural_codec::EncodeError> {
                let text = resolver.resolve(self.type_name)?.as_str().to_owned();
                ::core::result::Result::Ok(::raw_discovery::Block::Atom(
                    ::raw_discovery::Atom::new(text),
                ))
            }
        };

        let to_structural_body = quote! {
            ::structural_codec::StructuralValue::chosen(
                0,
                ::structural_codec::StructuralValue::Atom(self.type_name),
            )
        };

        self.assemble(
            item,
            entry_body,
            decode_within,
            encode_fn,
            to_structural_body,
        )
    }
}
