use std::convert::Infallible;
use std::error::Error;
use std::fmt;

use content_identity::{
    CapsuleNameTreeDomain, ContentHash, DomainSeparation, HashDomain, LayoutVersion, ShortCode,
};
use name_table::{IdentifierNamespace, NameTable};
use protos::{
    Capsule, CapsuleKind, CapsuleVerificationError, ShortIdentifier, TextualCapsuleAssociation,
};
use structural_codec::EncodedForm;

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
    assert_eq!(exposed.value(), 42);
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

#[derive(Clone)]
struct SourceRepresentation(FixtureCapsule);

#[derive(Clone)]
struct DocumentRepresentation(FixtureCapsule);

struct SourceAssociation;
struct DocumentAssociation;

impl TextualCapsuleAssociation for SourceAssociation {
    type TextualRepresentation = SourceRepresentation;
    type Capsule = FixtureCapsule;
    type ViewError = Infallible;
    type UnviewError = Infallible;

    fn view_capsule(
        capsule: &Self::Capsule,
    ) -> Result<Self::TextualRepresentation, Self::ViewError> {
        Ok(SourceRepresentation(capsule.clone()))
    }

    fn unview_capsule(
        textual: &Self::TextualRepresentation,
    ) -> Result<Self::Capsule, Self::UnviewError> {
        Ok(textual.0.clone())
    }
}

impl TextualCapsuleAssociation for DocumentAssociation {
    type TextualRepresentation = DocumentRepresentation;
    type Capsule = FixtureCapsule;
    type ViewError = Infallible;
    type UnviewError = Infallible;

    fn view_capsule(
        capsule: &Self::Capsule,
    ) -> Result<Self::TextualRepresentation, Self::ViewError> {
        Ok(DocumentRepresentation(capsule.clone()))
    }

    fn unview_capsule(
        textual: &Self::TextualRepresentation,
    ) -> Result<Self::Capsule, Self::UnviewError> {
        Ok(textual.0.clone())
    }
}

#[test]
fn two_projection_associations_round_trip_one_capsule_without_textual() {
    fn requires_fixed_capsule<Association>()
    where
        Association: TextualCapsuleAssociation<Capsule = FixtureCapsule>,
    {
    }

    requires_fixed_capsule::<SourceAssociation>();
    requires_fixed_capsule::<DocumentAssociation>();

    let original = capsule();

    let source = SourceAssociation::view_capsule(&original).expect("source view");
    let source_round_trip = SourceAssociation::unview_capsule(&source).expect("source unview");
    source_round_trip.verify().expect("source Capsule");
    assert_eq!(source_round_trip, original);

    let document = DocumentAssociation::view_capsule(&original).expect("document view");
    let document_round_trip =
        DocumentAssociation::unview_capsule(&document).expect("document unview");
    document_round_trip.verify().expect("document Capsule");
    assert_eq!(document_round_trip, original);
}
