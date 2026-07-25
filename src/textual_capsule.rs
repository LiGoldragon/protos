use std::error::Error;

use content_identity::ShortCode;
use name_table::NameTable;
use structural_codec::{ScopedEncodedTypeId, Textual, TextualForm};

use crate::Capsule;

/// Component-owned state used while a textual view becomes a Capsule.
///
/// The mutable nametree is explicit. Implementations may carry additional typed
/// construction state, but this contract provides no mint or hidden authority.
pub trait CapsuleUnviewContext {
    /// The explicit nametree mutated while the component constructs a Capsule.
    fn nametree(&mut self) -> &mut NameTable;
}

impl CapsuleUnviewContext for NameTable {
    fn nametree(&mut self) -> &mut NameTable {
        self
    }
}

/// Associates one textual projection type with one fixed Capsule type.
///
/// Rust coherence permits only one implementation for a projection type.
/// Several different projection types may select the same Capsule. The encoded
/// equality prevents a projection-specific semantic identity.
///
/// A second association implementation for one projection conflicts by Rust
/// coherence:
///
/// ```compile_fail,E0119
/// use std::convert::Infallible;
/// use std::error::Error;
/// use std::fmt;
///
/// use content_identity::{
///     CapsuleNameTreeDomain, ContentHash, DomainSeparation, HashDomain, LayoutVersion, ShortCode,
/// };
/// use name_table::{IdentifierNamespace, NameTable, NameTableError, NameTransaction};
/// use protos::{Capsule, CapsuleKind, ShortIdentifier, TextualCapsuleAssociation};
/// use raw_discovery::SealedTokenProfile;
/// use structural_codec::error::SingleChunkRequired;
/// use structural_codec::{
///     AddressedStructuralTable, DecodeError, EncodeError, EncodedForm, ScopedEncodedTypeId,
///     StructuralValue, Textual, TextualForm,
/// };
///
/// struct TestLanguage;
///
/// struct TestEncoded;
///
/// impl EncodedForm for TestEncoded {
///     type Language = TestLanguage;
/// }
///
/// struct TestDomain;
///
/// impl HashDomain for TestDomain {
///     fn separation() -> DomainSeparation {
///         DomainSeparation::Contextual {
///             context: "protos duplicate association doctest",
///             layout: LayoutVersion::new(1),
///         }
///     }
/// }
///
/// #[derive(Debug)]
/// struct TestError;
///
/// impl fmt::Display for TestError {
///     fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
///         formatter.write_str("test error")
///     }
/// }
///
/// impl Error for TestError {}
///
/// impl From<DecodeError> for TestError {
///     fn from(_: DecodeError) -> Self {
///         Self
///     }
/// }
///
/// impl From<EncodeError> for TestError {
///     fn from(_: EncodeError) -> Self {
///         Self
///     }
/// }
///
/// impl From<NameTableError> for TestError {
///     fn from(_: NameTableError) -> Self {
///         Self
///     }
/// }
///
/// impl From<SingleChunkRequired> for TestError {
///     fn from(_: SingleChunkRequired) -> Self {
///         Self
///     }
/// }
///
/// struct TestCapsule {
///     encoded: TestEncoded,
///     names: NameTable,
///     short_identifier: ShortCode,
///     content_pin: ContentHash<TestDomain>,
///     nametree_pin: ContentHash<CapsuleNameTreeDomain>,
/// }
///
/// impl TestCapsule {
///     fn sealed(names: NameTable, short_identifier: ShortCode) -> Self {
///         Self {
///             encoded: TestEncoded,
///             names,
///             short_identifier,
///             content_pin: ContentHash::from_bytes([1; 32]),
///             nametree_pin: ContentHash::from_bytes([2; 32]),
///         }
///     }
/// }
///
/// impl ShortIdentifier for TestCapsule {
///     fn short_identifier(&self) -> ShortCode {
///         self.short_identifier
///     }
/// }
///
/// impl Capsule for TestCapsule {
///     const KIND: CapsuleKind = CapsuleKind::Logos;
///
///     type EncodedForm = TestEncoded;
///     type ContentDomain = TestDomain;
///     type NameTree = NameTable;
///     type ContentIdentityError = Infallible;
///     type NameTreeIdentityError = Infallible;
///
///     fn encoded_form(&self) -> &Self::EncodedForm {
///         &self.encoded
///     }
///
///     fn nametree(&self) -> &Self::NameTree {
///         &self.names
///     }
///
///     fn content_identity_pin(&self) -> ContentHash<Self::ContentDomain> {
///         self.content_pin
///     }
///
///     fn nametree_identity_pin(&self) -> ContentHash<CapsuleNameTreeDomain> {
///         self.nametree_pin
///     }
///
///     fn rederive_content_identity(
///         &self,
///     ) -> Result<ContentHash<Self::ContentDomain>, Self::ContentIdentityError> {
///         Ok(self.content_pin)
///     }
///
///     fn rederive_nametree_identity(
///         &self,
///     ) -> Result<ContentHash<CapsuleNameTreeDomain>, Self::NameTreeIdentityError> {
///         Ok(self.nametree_pin)
///     }
/// }
///
/// struct OneProjection;
///
/// impl Textual for OneProjection {
///     type Encoded = TestEncoded;
///     type Language = TestLanguage;
///     type Error = TestError;
///
///     fn structuretree(&self) -> &AddressedStructuralTable {
///         unreachable!("the coherence witness does not execute the structural engine")
///     }
///
///     fn token_profile(&self) -> &SealedTokenProfile {
///         unreachable!("the coherence witness does not execute the structural engine")
///     }
///
///     fn missing_root_object(&self) -> Self::Error {
///         TestError
///     }
///
///     fn reify(
///         &self,
///         _: ScopedEncodedTypeId,
///         _: &StructuralValue,
///         _: &mut NameTransaction<'_>,
///     ) -> Result<Self::Encoded, Self::Error> {
///         Ok(TestEncoded)
///     }
///
///     fn reflect(
///         &self,
///         _: ScopedEncodedTypeId,
///         _: &Self::Encoded,
///         _: &NameTable,
///     ) -> Result<StructuralValue, Self::Error> {
///         unreachable!("the coherence witness does not execute the structural engine")
///     }
/// }
///
/// impl TextualCapsuleAssociation for OneProjection {
///     type Capsule = TestCapsule;
///     type UnviewContext = NameTable;
///     type AssociationError = TestError;
///
///     fn unview_capsule(
///         &self,
///         _: ScopedEncodedTypeId,
///         _: &TextualForm<Self::Language>,
///         context: Self::UnviewContext,
///         short_identifier: ShortCode,
///     ) -> Result<Self::Capsule, Self::AssociationError> {
///         Ok(TestCapsule::sealed(context, short_identifier))
///     }
///
///     fn view_capsule(
///         &self,
///         _: ScopedEncodedTypeId,
///         _: &Self::Capsule,
///     ) -> Result<TextualForm<Self::Language>, Self::AssociationError> {
///         Ok(TextualForm::single(String::new()))
///     }
/// }
///
/// impl TextualCapsuleAssociation for OneProjection {
///     type Capsule = TestCapsule;
///     type UnviewContext = NameTable;
///     type AssociationError = TestError;
///
///     fn unview_capsule(
///         &self,
///         _: ScopedEncodedTypeId,
///         _: &TextualForm<Self::Language>,
///         context: Self::UnviewContext,
///         short_identifier: ShortCode,
///     ) -> Result<Self::Capsule, Self::AssociationError> {
///         Ok(TestCapsule::sealed(context, short_identifier))
///     }
///
///     fn view_capsule(
///         &self,
///         _: ScopedEncodedTypeId,
///         _: &Self::Capsule,
///     ) -> Result<TextualForm<Self::Language>, Self::AssociationError> {
///         Ok(TextualForm::single(String::new()))
///     }
/// }
///
/// let projection = OneProjection;
/// let names = NameTable::new(IdentifierNamespace::Logos);
/// let code = ShortCode::from_value(1).expect("valid code");
/// let capsule = projection
///     .unview_capsule(
///         ScopedEncodedTypeId::new(structural_codec::FIXTURE_UNIVERSE, 1),
///         &TextualForm::single(String::new()),
///         names,
///         code,
///     )
///     .expect("valid association");
/// let _ = projection
///     .view_capsule(
///         ScopedEncodedTypeId::new(structural_codec::FIXTURE_UNIVERSE, 1),
///         &capsule,
///     )
///     .expect("valid association");
/// ```
///
/// The associated Capsule's encoded truth cannot differ from the projection's
/// `Textual::Encoded` type:
///
/// ```compile_fail
/// use protos::{Capsule, TextualCapsuleAssociation};
/// use structural_codec::Textual;
///
/// fn incompatible<Projection>(
///     capsule: &<Projection as TextualCapsuleAssociation>::Capsule,
/// )
/// where
///     Projection: TextualCapsuleAssociation + Textual<Encoded = u8>,
/// {
///     let _: &u16 = capsule.encoded_form();
/// }
/// ```
pub trait TextualCapsuleAssociation: Textual {
    /// The one Capsule type selected by this projection.
    type Capsule: Capsule<EncodedForm = <Self as Textual>::Encoded>;
    /// Explicit component-owned state needed to construct that Capsule.
    type UnviewContext: CapsuleUnviewContext;
    /// The component's typed association failure.
    type AssociationError: Error + From<<Self as Textual>::Error> + Send + Sync + 'static;

    /// Build a Capsule from a textual view using only explicit context and an
    /// already-issued short identifier.
    fn unview_capsule(
        &self,
        expected: ScopedEncodedTypeId,
        view: &TextualForm<Self::Language>,
        context: Self::UnviewContext,
        short_identifier: ShortCode,
    ) -> Result<Self::Capsule, Self::AssociationError>;

    /// Produce this projection's textual view of a Capsule.
    fn view_capsule(
        &self,
        expected: ScopedEncodedTypeId,
        capsule: &Self::Capsule,
    ) -> Result<TextualForm<Self::Language>, Self::AssociationError>;
}
