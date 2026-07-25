use std::error::Error;
use std::fmt;

use content_identity::{
    CapsuleNameTreeDomain, ContentHash, DomainSeparation, HashDomain, LayoutVersion, ShortCode,
};
use name_table::{IdentifierNamespace, NameTable, NameTableError, NameTransaction};
use protos::{
    Capsule, CapsuleKind, CapsuleVerificationError, ShortIdentifier, TextualCapsuleAssociation,
};
use raw_discovery::SealedTokenProfile;
use structural_codec::error::SingleChunkRequired;
use structural_codec::{
    AddressedStructuralTable, DecodeError, EncodeError, EncodedForm, ScopedEncodedTypeId,
    StructuralValue, Textual, TextualForm,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct FixtureLanguage;

#[derive(Clone, Debug, Eq, PartialEq)]
struct FixtureEncoded(u32);

impl EncodedForm for FixtureEncoded {
    type Language = FixtureLanguage;
}

struct FixtureContentDomain;

impl HashDomain for FixtureContentDomain {
    fn separation() -> DomainSeparation {
        DomainSeparation::Contextual {
            context: "protos contract fixture content",
            layout: LayoutVersion::new(1),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct DerivationFailure(&'static str);

impl fmt::Display for DerivationFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.0)
    }
}

impl Error for DerivationFailure {}

#[derive(Clone, Debug, Eq, PartialEq)]
struct FixtureCapsule {
    encoded: FixtureEncoded,
    names: NameTable,
    short_identifier: ShortCode,
    content_pin: ContentHash<FixtureContentDomain>,
    nametree_pin: ContentHash<CapsuleNameTreeDomain>,
    fail_content_derivation: bool,
    fail_nametree_derivation: bool,
}

impl FixtureCapsule {
    fn sealed(value: u32, names: NameTable, short_identifier: ShortCode) -> Self {
        let encoded = FixtureEncoded(value);
        let content_pin = Self::derive_content(&encoded);
        let nametree_pin = Self::derive_nametree(&names).expect("fixture nametree identity");
        Self {
            encoded,
            names,
            short_identifier,
            content_pin,
            nametree_pin,
            fail_content_derivation: false,
            fail_nametree_derivation: false,
        }
    }

    fn derive_content(encoded: &FixtureEncoded) -> ContentHash<FixtureContentDomain> {
        ContentHash::derive(&encoded.0.to_le_bytes())
    }

    fn derive_nametree(
        names: &NameTable,
    ) -> Result<ContentHash<CapsuleNameTreeDomain>, DerivationFailure> {
        let slice_identity = names
            .identity()
            .map_err(|_| DerivationFailure("fixture nametree identity"))?;
        Ok(ContentHash::derive(slice_identity.bytes()))
    }
}

impl ShortIdentifier for FixtureCapsule {
    fn short_identifier(&self) -> ShortCode {
        self.short_identifier
    }
}

impl Capsule for FixtureCapsule {
    const KIND: CapsuleKind = CapsuleKind::Logos;

    type EncodedForm = FixtureEncoded;
    type ContentDomain = FixtureContentDomain;
    type NameTree = NameTable;
    type ContentIdentityError = DerivationFailure;
    type NameTreeIdentityError = DerivationFailure;

    fn encoded_form(&self) -> &Self::EncodedForm {
        &self.encoded
    }

    fn nametree(&self) -> &Self::NameTree {
        &self.names
    }

    fn content_identity_pin(&self) -> ContentHash<Self::ContentDomain> {
        self.content_pin
    }

    fn nametree_identity_pin(&self) -> ContentHash<CapsuleNameTreeDomain> {
        self.nametree_pin
    }

    fn rederive_content_identity(
        &self,
    ) -> Result<ContentHash<Self::ContentDomain>, Self::ContentIdentityError> {
        if self.fail_content_derivation {
            return Err(DerivationFailure("content derivation"));
        }
        Ok(Self::derive_content(&self.encoded))
    }

    fn rederive_nametree_identity(
        &self,
    ) -> Result<ContentHash<CapsuleNameTreeDomain>, Self::NameTreeIdentityError> {
        if self.fail_nametree_derivation {
            return Err(DerivationFailure("nametree derivation"));
        }
        Self::derive_nametree(&self.names)
    }
}

fn code() -> ShortCode {
    ShortCode::from_value(42).expect("canonical short code")
}

fn capsule() -> FixtureCapsule {
    FixtureCapsule::sealed(42, NameTable::new(IdentifierNamespace::Logos), code())
}

#[test]
fn capsule_kind_is_exactly_the_three_component_roles() {
    assert_eq!(
        CapsuleKind::ALL,
        [CapsuleKind::Schema, CapsuleKind::Logos, CapsuleKind::Nomos,]
    );
    for kind in CapsuleKind::ALL {
        match kind {
            CapsuleKind::Schema | CapsuleKind::Logos | CapsuleKind::Nomos => {}
        }
    }
}

#[test]
fn short_identifier_exposes_the_canonical_content_identity_code() {
    let capsule = capsule();
    let exposed: ShortCode = capsule.short_identifier();
    assert_eq!(exposed, code());
    assert_eq!(exposed.to_base36(), "0016");
}

#[test]
fn pins_are_required_typed_values_and_verification_succeeds() {
    fn require_pins<C: Capsule>(capsule: &C) {
        let _: ContentHash<C::ContentDomain> = capsule.content_identity_pin();
        let _: ContentHash<CapsuleNameTreeDomain> = capsule.nametree_identity_pin();
    }

    let capsule = capsule();
    require_pins(&capsule);
    assert_eq!(capsule.encoded_form(), &FixtureEncoded(42));
    assert_eq!(
        capsule.nametree(),
        &NameTable::new(IdentifierNamespace::Logos)
    );
    capsule.verify().expect("valid required pins");
}

#[test]
fn verification_reports_both_derivation_failures() {
    let mut content_failure = capsule();
    content_failure.fail_content_derivation = true;
    assert!(matches!(
        content_failure.verify(),
        Err(CapsuleVerificationError::ContentIdentityDerivation(
            DerivationFailure("content derivation")
        ))
    ));

    let mut nametree_failure = capsule();
    nametree_failure.fail_nametree_derivation = true;
    assert!(matches!(
        nametree_failure.verify(),
        Err(CapsuleVerificationError::NameTreeIdentityDerivation(
            DerivationFailure("nametree derivation")
        ))
    ));
}

#[test]
fn verification_reports_both_mismatches_with_values() {
    let mut content_mismatch = capsule();
    let pinned_content = ContentHash::derive(b"different content");
    content_mismatch.content_pin = pinned_content;
    match content_mismatch.verify() {
        Err(CapsuleVerificationError::ContentMismatch { pinned, actual }) => {
            assert_eq!(pinned, pinned_content);
            assert_eq!(actual, FixtureCapsule::derive_content(&FixtureEncoded(42)));
        }
        other => panic!("expected content mismatch, got {other:?}"),
    }

    let mut nametree_mismatch = capsule();
    let pinned_nametree = ContentHash::derive(b"different nametree");
    nametree_mismatch.nametree_pin = pinned_nametree;
    match nametree_mismatch.verify() {
        Err(CapsuleVerificationError::NameTreeMismatch { pinned, actual }) => {
            assert_eq!(pinned, pinned_nametree);
            assert_eq!(
                actual,
                FixtureCapsule::derive_nametree(&NameTable::new(IdentifierNamespace::Logos))
                    .expect("fixture nametree identity")
            );
        }
        other => panic!("expected nametree mismatch, got {other:?}"),
    }
}

#[derive(Debug)]
struct ProjectionError;

impl fmt::Display for ProjectionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("fixture projection error")
    }
}

impl Error for ProjectionError {}

impl From<DecodeError> for ProjectionError {
    fn from(_: DecodeError) -> Self {
        Self
    }
}

impl From<EncodeError> for ProjectionError {
    fn from(_: EncodeError) -> Self {
        Self
    }
}

impl From<NameTableError> for ProjectionError {
    fn from(_: NameTableError) -> Self {
        Self
    }
}

impl From<SingleChunkRequired> for ProjectionError {
    fn from(_: SingleChunkRequired) -> Self {
        Self
    }
}

struct DecimalProjection;
struct HexadecimalProjection;

macro_rules! impl_textual_fixture {
    ($projection:ty) => {
        impl Textual for $projection {
            type Encoded = FixtureEncoded;
            type Language = FixtureLanguage;
            type Error = ProjectionError;

            fn structuretree(&self) -> &AddressedStructuralTable {
                unreachable!("association fixtures do not execute the structural engine")
            }

            fn token_profile(&self) -> &SealedTokenProfile {
                unreachable!("association fixtures do not execute the structural engine")
            }

            fn missing_root_object(&self) -> Self::Error {
                ProjectionError
            }

            fn reify(
                &self,
                _: ScopedEncodedTypeId,
                _: &StructuralValue,
                _: &mut NameTransaction<'_>,
            ) -> Result<Self::Encoded, Self::Error> {
                unreachable!("association fixtures provide their own contract direction")
            }

            fn reflect(
                &self,
                _: ScopedEncodedTypeId,
                _: &Self::Encoded,
                _: &NameTable,
            ) -> Result<StructuralValue, Self::Error> {
                unreachable!("association fixtures provide their own contract direction")
            }
        }
    };
}

impl_textual_fixture!(DecimalProjection);
impl_textual_fixture!(HexadecimalProjection);

impl TextualCapsuleAssociation for DecimalProjection {
    type Capsule = FixtureCapsule;
    type UnviewContext = NameTable;
    type AssociationError = ProjectionError;

    fn unview_capsule(
        &self,
        _: ScopedEncodedTypeId,
        view: &TextualForm<Self::Language>,
        context: Self::UnviewContext,
        short_identifier: ShortCode,
    ) -> Result<Self::Capsule, Self::AssociationError> {
        let value = view.sole_text()?.parse().map_err(|_| ProjectionError)?;
        Ok(FixtureCapsule::sealed(value, context, short_identifier))
    }

    fn view_capsule(
        &self,
        _: ScopedEncodedTypeId,
        capsule: &Self::Capsule,
    ) -> Result<TextualForm<Self::Language>, Self::AssociationError> {
        Ok(TextualForm::single(capsule.encoded.0.to_string()))
    }
}

impl TextualCapsuleAssociation for HexadecimalProjection {
    type Capsule = FixtureCapsule;
    type UnviewContext = NameTable;
    type AssociationError = ProjectionError;

    fn unview_capsule(
        &self,
        _: ScopedEncodedTypeId,
        view: &TextualForm<Self::Language>,
        context: Self::UnviewContext,
        short_identifier: ShortCode,
    ) -> Result<Self::Capsule, Self::AssociationError> {
        let value = u32::from_str_radix(view.sole_text()?, 16).map_err(|_| ProjectionError)?;
        Ok(FixtureCapsule::sealed(value, context, short_identifier))
    }

    fn view_capsule(
        &self,
        _: ScopedEncodedTypeId,
        capsule: &Self::Capsule,
    ) -> Result<TextualForm<Self::Language>, Self::AssociationError> {
        Ok(TextualForm::single(format!("{:x}", capsule.encoded.0)))
    }
}

#[test]
fn two_textual_projections_round_trip_one_capsule_type() {
    fn requires_same_capsule<Projection>()
    where
        Projection: TextualCapsuleAssociation<Capsule = FixtureCapsule>,
    {
    }

    requires_same_capsule::<DecimalProjection>();
    requires_same_capsule::<HexadecimalProjection>();

    let expected = ScopedEncodedTypeId::new(structural_codec::FIXTURE_UNIVERSE, 1);
    let original = capsule();

    let decimal_view = DecimalProjection
        .view_capsule(expected, &original)
        .expect("decimal view");
    assert_eq!(decimal_view.sole_text().expect("decimal text"), "42");
    let decimal_round_trip = DecimalProjection
        .unview_capsule(
            expected,
            &decimal_view,
            NameTable::new(IdentifierNamespace::Logos),
            code(),
        )
        .expect("decimal unview");
    decimal_round_trip.verify().expect("decimal Capsule");
    assert_eq!(decimal_round_trip.encoded_form(), original.encoded_form());

    let hexadecimal_view = HexadecimalProjection
        .view_capsule(expected, &original)
        .expect("hexadecimal view");
    assert_eq!(
        hexadecimal_view.sole_text().expect("hexadecimal text"),
        "2a"
    );
    let hexadecimal_round_trip = HexadecimalProjection
        .unview_capsule(
            expected,
            &hexadecimal_view,
            NameTable::new(IdentifierNamespace::Logos),
            code(),
        )
        .expect("hexadecimal unview");
    hexadecimal_round_trip
        .verify()
        .expect("hexadecimal Capsule");
    assert_eq!(
        hexadecimal_round_trip.encoded_form(),
        original.encoded_form()
    );
}
