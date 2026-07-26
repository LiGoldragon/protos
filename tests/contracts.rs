use std::convert::Infallible;
use std::error::Error;
use std::fmt;

use content_identity::{
    CapsuleNameTreeDomain, CapsuleNameTreeIdentityPreimage, CapsuleNameTreeIdentitySlice,
    ContentHash, DomainSeparation, HashDomain, LayoutVersion,
};
use name_table::{IdentifierNamespace, Name, NameTable};
use protos::{Capsule, CapsuleKind, CapsuleVerificationError, TextualCapsuleAssociation};
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

#[derive(Debug, Eq, PartialEq)]
struct FixtureCapsule {
    encoded: FixtureEncoded,
    names: NameTable,
    content_pin: ContentHash<FixtureContentDomain>,
    nametree_pin: ContentHash<CapsuleNameTreeDomain>,
    fail_content_derivation: bool,
    fail_nametree_derivation: bool,
}

impl FixtureCapsule {
    fn sealed(value: u32, names: NameTable) -> Self {
        let encoded = FixtureEncoded(value);
        let content_pin = Self::derive_content(&encoded);
        let nametree_pin = Self::derive_nametree(&names).expect("fixture nametree identity");
        Self {
            encoded,
            names,
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
        let (home, borrowed) = names
            .slice_snapshots()
            .map_err(|_| DerivationFailure("fixture nametree snapshots"))?;
        let home = CapsuleNameTreeIdentitySlice::new(home.namespace(), home.identity());
        let borrowed = borrowed
            .into_iter()
            .map(|slice| CapsuleNameTreeIdentitySlice::new(slice.namespace(), slice.identity()))
            .collect();
        CapsuleNameTreeIdentityPreimage::try_new(home, borrowed)
            .map_err(|_| DerivationFailure("fixture nametree topology"))?
            .derive_identity()
            .map_err(|_| DerivationFailure("fixture nametree identity"))
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

fn complete_nametree() -> NameTable {
    let mut schema = NameTable::new(IdentifierNamespace::Schema);
    schema
        .intern(Name::new("FixtureSchema"))
        .expect("fixture schema name");

    let mut logos = NameTable::new(IdentifierNamespace::Logos);
    logos
        .intern(Name::new("FixtureLogos"))
        .expect("fixture logos name");
    logos.compose(&schema).expect("complete fixture nametree")
}

fn capsule() -> FixtureCapsule {
    FixtureCapsule::sealed(42, complete_nametree())
}

#[derive(Debug, Eq, PartialEq)]
struct OpaqueSource {
    value: u32,
    names: NameTable,
}

#[derive(Debug, Eq, PartialEq)]
struct OpaqueDocument {
    value: u32,
    names: NameTable,
}

struct SourceAssociation;
struct DocumentAssociation;

impl TextualCapsuleAssociation for SourceAssociation {
    type TextualRepresentation = OpaqueSource;
    type Capsule = FixtureCapsule;
    type ViewError = Infallible;
    type UnviewError = Infallible;

    fn view_capsule(
        capsule: &Self::Capsule,
    ) -> Result<Self::TextualRepresentation, Self::ViewError> {
        Ok(OpaqueSource {
            value: capsule.encoded_form().0,
            names: capsule.nametree().clone(),
        })
    }

    fn unview_capsule(
        textual: &Self::TextualRepresentation,
    ) -> Result<Self::Capsule, Self::UnviewError> {
        Ok(FixtureCapsule::sealed(textual.value, textual.names.clone()))
    }
}

impl TextualCapsuleAssociation for DocumentAssociation {
    type TextualRepresentation = OpaqueDocument;
    type Capsule = FixtureCapsule;
    type ViewError = Infallible;
    type UnviewError = Infallible;

    fn view_capsule(
        capsule: &Self::Capsule,
    ) -> Result<Self::TextualRepresentation, Self::ViewError> {
        Ok(OpaqueDocument {
            value: capsule.encoded_form().0,
            names: capsule.nametree().clone(),
        })
    }

    fn unview_capsule(
        textual: &Self::TextualRepresentation,
    ) -> Result<Self::Capsule, Self::UnviewError> {
        Ok(FixtureCapsule::sealed(textual.value, textual.names.clone()))
    }
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
fn capsule_implementation_requires_full_pins_without_a_display_accessor() {
    fn require_pins<CapsuleType: Capsule>(
        capsule: &CapsuleType,
    ) -> (
        ContentHash<CapsuleType::ContentDomain>,
        ContentHash<CapsuleNameTreeDomain>,
    ) {
        (
            capsule.content_identity_pin(),
            capsule.nametree_identity_pin(),
        )
    }

    let capsule = capsule();
    let (content_pin, nametree_pin) = require_pins(&capsule);
    assert_eq!(
        content_pin,
        FixtureCapsule::derive_content(capsule.encoded_form())
    );
    assert_eq!(
        nametree_pin,
        FixtureCapsule::derive_nametree(capsule.nametree()).expect("fixture nametree identity")
    );
    assert!(
        capsule
            .nametree()
            .resolve(IdentifierNamespace::Schema.identifier(0))
            .is_ok()
    );
    assert!(
        capsule
            .nametree()
            .resolve(IdentifierNamespace::Logos.identifier(0))
            .is_ok()
    );
    capsule.verify().expect("valid required pins");
}

#[test]
fn opaque_associations_round_trip_the_fixed_capsule_with_full_pins() {
    fn requires_fixed_capsule<Association>()
    where
        Association: TextualCapsuleAssociation<Capsule = FixtureCapsule>,
    {
    }

    fn assert_full_identity(original: &FixtureCapsule, recovered: &FixtureCapsule) {
        assert_eq!(recovered.encoded_form(), original.encoded_form());
        assert_eq!(recovered.nametree(), original.nametree());
        assert_eq!(
            recovered.content_identity_pin(),
            original.content_identity_pin()
        );
        assert_eq!(
            recovered.nametree_identity_pin(),
            original.nametree_identity_pin()
        );
        recovered.verify().expect("recovered full pins");
    }

    requires_fixed_capsule::<SourceAssociation>();
    requires_fixed_capsule::<DocumentAssociation>();

    let original = capsule();

    let source = SourceAssociation::view_capsule(&original).expect("opaque source view");
    let from_source = SourceAssociation::unview_capsule(&source).expect("opaque source unview");
    assert_full_identity(&original, &from_source);

    let document = DocumentAssociation::view_capsule(&original).expect("opaque document view");
    let from_document =
        DocumentAssociation::unview_capsule(&document).expect("opaque document unview");
    assert_full_identity(&original, &from_document);
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
                FixtureCapsule::derive_nametree(nametree_mismatch.nametree())
                    .expect("fixture nametree identity")
            );
        }
        other => panic!("expected nametree mismatch, got {other:?}"),
    }
}
