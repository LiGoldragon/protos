//! The parsed authority: one `#[structural_form(...)]` attribute plus the named
//! placeholder item it decorates. This is the SINGLE source the code generator
//! lowers three ways (design decision 7): to the authoritative `StructuralEntry`
//! data, to the optimized `GeneratedCodec`, and to the typed capture the codec
//! fills. Parsing owns no lowering; it only reads the authority into typed data.

use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{Attribute, Ident, LitInt, Path, Token, Visibility, bracketed, parenthesized};

/// A fully parsed derived-type authority.
pub struct TypeSpec {
    /// The placeholder's outer attributes (documentation is preserved onto the
    /// generated type).
    pub attrs: Vec<Attribute>,
    pub visibility: Visibility,
    pub name: Ident,
    /// The scoped Core-type local id in the fixture universe.
    pub id: u32,
    pub kind: Kind,
}

/// The closed set of structural kinds this proof-of-concept derives. Each kind
/// names a distinct constructor shape from the fixture family; the variant set
/// lives in the type system, never a string flag consulted at codegen.
pub enum Kind {
    /// A scalar leaf value type: flatten-then-parse.
    Leaf(ScalarKind),
    /// A transparent newtype value wrapper delegating to `inner`.
    Delegate { inner: Path },
    /// A newtype DECLARATION `Object.{ Inner }` over a scalar-or-named inner.
    NewtypeDeclaration { inner: Path, delimiter: Delimiter },
    /// A struct DECLARATION `Object.{ Field* }`: a fixed product of delegated
    /// fields, each decoded through the `field_type` meta-type.
    StructDeclaration {
        field_type: Path,
        delimiter: Delimiter,
        fields: Vec<Path>,
    },
    /// The `Field` meta-type: ONE constructor, the bare elided-name `Type`. Field
    /// names are illegal everywhere, so the explicit `name.Type` form no longer
    /// parses.
    FieldMeta,
}

/// Which scalar a leaf flattens to.
#[derive(Clone, Copy)]
pub enum ScalarKind {
    Integer,
    Float,
    Text,
    Boolean,
}

impl ScalarKind {
    fn from_ident(ident: &Ident) -> syn::Result<Self> {
        match ident.to_string().as_str() {
            "Integer" => Ok(Self::Integer),
            "Float" => Ok(Self::Float),
            "Text" => Ok(Self::Text),
            "Boolean" => Ok(Self::Boolean),
            other => Err(syn::Error::new(
                ident.span(),
                format!(
                    "unknown scalar leaf `{other}` (expected Integer, Float, Text, or Boolean)"
                ),
            )),
        }
    }
}

/// A block delimiter, mirrored onto `raw_discovery::Delimiter` at codegen.
#[derive(Clone, Copy)]
pub enum Delimiter {
    Parenthesis,
    SquareBracket,
    Brace,
}

impl Delimiter {
    fn from_ident(ident: &Ident) -> syn::Result<Self> {
        match ident.to_string().as_str() {
            "Parenthesis" => Ok(Self::Parenthesis),
            "SquareBracket" => Ok(Self::SquareBracket),
            "Brace" => Ok(Self::Brace),
            other => Err(syn::Error::new(
                ident.span(),
                format!(
                    "unknown delimiter `{other}` (expected Parenthesis, SquareBracket, or Brace)"
                ),
            )),
        }
    }
}

impl TypeSpec {
    /// Parse the attribute tokens and the placeholder item into one authority.
    pub fn parse(
        attribute: proc_macro2::TokenStream,
        item: proc_macro2::TokenStream,
    ) -> syn::Result<Self> {
        let arguments: FormArguments = syn::parse2(attribute)?;
        let placeholder: syn::ItemStruct = syn::parse2(item)?;
        Ok(Self {
            attrs: placeholder.attrs,
            visibility: placeholder.vis,
            name: placeholder.ident,
            id: arguments.id,
            kind: arguments.kind,
        })
    }
}

/// The attribute payload `id = N, <kind>(...)`.
struct FormArguments {
    id: u32,
    kind: Kind,
}

impl Parse for FormArguments {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        input.expect_key("id")?;
        input.parse::<Token![=]>()?;
        let id = input.parse::<LitInt>()?.base10_parse::<u32>()?;
        input.parse::<Token![,]>()?;

        let kind_ident: Ident = input.parse()?;
        let kind = match kind_ident.to_string().as_str() {
            "leaf" => {
                let content;
                parenthesized!(content in input);
                let scalar: Ident = content.parse()?;
                Kind::Leaf(ScalarKind::from_ident(&scalar)?)
            }
            "delegate" => {
                let content;
                parenthesized!(content in input);
                Kind::Delegate {
                    inner: content.named_path("inner")?,
                }
            }
            "newtype_declaration" => {
                let content;
                parenthesized!(content in input);
                let inner = content.named_path("inner")?;
                content.parse::<Token![,]>()?;
                let delimiter = content.named_delimiter("delimiter")?;
                Kind::NewtypeDeclaration { inner, delimiter }
            }
            "struct_declaration" => {
                let content;
                parenthesized!(content in input);
                let field_type = content.named_path("field_type")?;
                content.parse::<Token![,]>()?;
                let delimiter = content.named_delimiter("delimiter")?;
                content.parse::<Token![,]>()?;
                let fields = content.named_path_list("fields")?;
                Kind::StructDeclaration {
                    field_type,
                    delimiter,
                    fields,
                }
            }
            "field_meta" => Kind::FieldMeta,
            other => {
                return Err(syn::Error::new(
                    kind_ident.span(),
                    format!("unknown structural kind `{other}`"),
                ));
            }
        };
        Ok(Self { id, kind })
    }
}

/// Keyed-argument reading, hung on the parse buffer that owns the cursor rather
/// than on free functions: `key = <value>` pairs the attribute grammar uses.
trait KeyedArguments {
    fn expect_key(&self, key: &str) -> syn::Result<()>;
    fn named_path(&self, key: &str) -> syn::Result<Path>;
    fn named_delimiter(&self, key: &str) -> syn::Result<Delimiter>;
    fn named_path_list(&self, key: &str) -> syn::Result<Vec<Path>>;
}

impl KeyedArguments for syn::parse::ParseBuffer<'_> {
    fn expect_key(&self, key: &str) -> syn::Result<()> {
        let found: Ident = self.parse()?;
        if found == key {
            Ok(())
        } else {
            Err(syn::Error::new(found.span(), format!("expected `{key}`")))
        }
    }

    fn named_path(&self, key: &str) -> syn::Result<Path> {
        self.expect_key(key)?;
        self.parse::<Token![=]>()?;
        self.parse::<Path>()
    }

    fn named_delimiter(&self, key: &str) -> syn::Result<Delimiter> {
        self.expect_key(key)?;
        self.parse::<Token![=]>()?;
        Delimiter::from_ident(&self.parse::<Ident>()?)
    }

    fn named_path_list(&self, key: &str) -> syn::Result<Vec<Path>> {
        self.expect_key(key)?;
        self.parse::<Token![=]>()?;
        let content;
        bracketed!(content in self);
        let paths = Punctuated::<Path, Token![,]>::parse_terminated(&content)?;
        Ok(paths.into_iter().collect())
    }
}
