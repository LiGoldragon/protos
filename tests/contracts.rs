use std::convert::Infallible;

use protos::{
    Capsule, CapsuleArchiveError, CapsuleIdentityVariant, ContentAddressedHash, Ethos, Logos,
    Nomos, TextualCapsuleAssociation,
};
use static_assertions::assert_type_ne_all;

#[derive(Clone, Copy, Debug, Eq, PartialEq, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize)]
struct FixtureCompleteNameTreePin([u8; 32]);

const HASH_BYTES: [u8; 32] = [0x11; 32];
const PIN_BYTES: [u8; 32] = [0xa5; 32];

// Version 1 fixes the two positional stored fields introduced by protos 0.3.
// An intentional representation change replaces this literal under a new
// archive-version name.
const CAPSULE_ARCHIVE_V1: &[u8] = &[
    0x02, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11,
    0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11,
    0x11, 0xa5, 0xa5, 0xa5, 0xa5, 0xa5, 0xa5, 0xa5, 0xa5, 0xa5, 0xa5, 0xa5, 0xa5, 0xa5, 0xa5, 0xa5,
    0xa5, 0xa5, 0xa5, 0xa5, 0xa5, 0xa5, 0xa5, 0xa5, 0xa5, 0xa5, 0xa5, 0xa5, 0xa5, 0xa5, 0xa5, 0xa5,
    0xa5,
];

const CAPSULE_IDENTITY_DISCRIMINANT_OFFSET_V1: usize = 0;
const CAPSULE_CONTENT_HASH_OFFSET_V1: usize = 1;
const CAPSULE_COMPLETE_NAMETREE_PIN_OFFSET_V1: usize = 33;

fn hash() -> ContentAddressedHash {
    ContentAddressedHash::from_bytes(HASH_BYTES)
}

fn logos_capsule() -> Capsule<Logos, FixtureCompleteNameTreePin> {
    Capsule::new(hash(), FixtureCompleteNameTreePin(PIN_BYTES))
}

#[test]
fn markers_make_the_three_capsule_kinds_distinct_types() {
    assert_type_ne_all!(
        Capsule<Ethos, FixtureCompleteNameTreePin>,
        Capsule<Nomos, FixtureCompleteNameTreePin>,
        Capsule<Logos, FixtureCompleteNameTreePin>
    );
}

#[test]
fn construction_maps_each_kind_to_its_only_identity_variant() {
    let ethos = Capsule::<Ethos, _>::new(hash(), FixtureCompleteNameTreePin(PIN_BYTES));
    let nomos = Capsule::<Nomos, _>::new(hash(), FixtureCompleteNameTreePin(PIN_BYTES));
    let logos = logos_capsule();

    assert_eq!(
        ethos.content_identity().variant(),
        CapsuleIdentityVariant::Ethos
    );
    assert_eq!(
        nomos.content_identity().variant(),
        CapsuleIdentityVariant::Nomos
    );
    assert_eq!(
        logos.content_identity().variant(),
        CapsuleIdentityVariant::WholeLogos
    );
    assert_eq!(ethos.content_addressed_hash(), hash());
    assert_eq!(nomos.content_addressed_hash(), hash());
    assert_eq!(logos.content_addressed_hash(), hash());
    assert_eq!(
        logos.complete_nametree_pin(),
        &FixtureCompleteNameTreePin(PIN_BYTES)
    );

    let (identity, complete_nametree_pin) = logos.into_parts();
    assert_eq!(identity.variant(), CapsuleIdentityVariant::WholeLogos);
    assert_eq!(complete_nametree_pin, FixtureCompleteNameTreePin(PIN_BYTES));
}

#[test]
fn capsule_archive_v1_is_positional_and_round_trips() {
    let original = logos_capsule();
    let bytes = original.to_archive_bytes().expect("archive Capsule");

    assert_eq!(
        bytes, CAPSULE_ARCHIVE_V1,
        "an intentional Capsule representation change needs a new archive-version lock"
    );

    let restored = Capsule::<Logos, FixtureCompleteNameTreePin>::from_archive_bytes(&bytes)
        .expect("restore Capsule");
    assert_eq!(restored, original);
}

#[test]
fn archive_field_mutations_reach_their_positional_fields() {
    let original = logos_capsule();
    let bytes = original.to_archive_bytes().expect("archive Capsule");

    let mut hash_mutation = bytes.clone();
    hash_mutation[CAPSULE_CONTENT_HASH_OFFSET_V1] = 0x22;
    let changed_hash =
        Capsule::<Logos, FixtureCompleteNameTreePin>::from_archive_bytes(&hash_mutation)
            .expect("restore changed hash");
    let mut expected_hash = HASH_BYTES;
    expected_hash[0] = 0x22;
    assert_eq!(
        changed_hash.content_addressed_hash(),
        ContentAddressedHash::from_bytes(expected_hash)
    );
    assert_eq!(
        changed_hash.complete_nametree_pin(),
        original.complete_nametree_pin()
    );

    let mut pin_mutation = bytes;
    pin_mutation[CAPSULE_COMPLETE_NAMETREE_PIN_OFFSET_V1] = 0x33;
    let changed_pin =
        Capsule::<Logos, FixtureCompleteNameTreePin>::from_archive_bytes(&pin_mutation)
            .expect("restore changed pin");
    let mut expected_pin = PIN_BYTES;
    expected_pin[0] = 0x33;
    assert_eq!(changed_pin.content_addressed_hash(), hash());
    assert_eq!(
        changed_pin.complete_nametree_pin(),
        &FixtureCompleteNameTreePin(expected_pin)
    );
}

#[test]
fn invalid_identity_discriminant_is_refused_during_archive_validation() {
    let mut bytes = logos_capsule().to_archive_bytes().expect("archive Capsule");
    assert_eq!(
        bytes[CAPSULE_IDENTITY_DISCRIMINANT_OFFSET_V1], 2,
        "the version-1 archive lock places the outer discriminant here"
    );
    bytes[CAPSULE_IDENTITY_DISCRIMINANT_OFFSET_V1] = u8::MAX;

    assert!(matches!(
        Capsule::<Logos, FixtureCompleteNameTreePin>::from_archive_bytes(&bytes),
        Err(CapsuleArchiveError::Archive(_))
    ));
}

#[test]
fn valid_wrong_kind_archive_is_refused_with_the_typed_mismatch() {
    let nomos = Capsule::<Nomos, _>::new(hash(), FixtureCompleteNameTreePin(PIN_BYTES));
    let bytes = nomos.to_archive_bytes().expect("archive Nomos Capsule");

    match Capsule::<Logos, FixtureCompleteNameTreePin>::from_archive_bytes(&bytes) {
        Err(CapsuleArchiveError::KindMismatch(mismatch)) => {
            assert_eq!(mismatch.expected(), CapsuleIdentityVariant::WholeLogos);
            assert_eq!(mismatch.actual(), CapsuleIdentityVariant::Nomos);
        }
        other => panic!("expected valid wrong-kind refusal, got {other:?}"),
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct OpaqueSource(ContentAddressedHash, FixtureCompleteNameTreePin);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct OpaqueDocument(ContentAddressedHash, FixtureCompleteNameTreePin);

struct SourceAssociation;
struct DocumentAssociation;

impl TextualCapsuleAssociation for SourceAssociation {
    type TextualRepresentation = OpaqueSource;
    type Kind = Logos;
    type CompleteNameTreePin = FixtureCompleteNameTreePin;
    type ViewError = Infallible;
    type UnviewError = Infallible;

    fn view_capsule(
        capsule: &Capsule<Self::Kind, Self::CompleteNameTreePin>,
    ) -> Result<Self::TextualRepresentation, Self::ViewError> {
        Ok(OpaqueSource(
            capsule.content_addressed_hash(),
            *capsule.complete_nametree_pin(),
        ))
    }

    fn unview_capsule(
        textual: &Self::TextualRepresentation,
    ) -> Result<Capsule<Self::Kind, Self::CompleteNameTreePin>, Self::UnviewError> {
        Ok(Capsule::new(textual.0, textual.1))
    }
}

impl TextualCapsuleAssociation for DocumentAssociation {
    type TextualRepresentation = OpaqueDocument;
    type Kind = Logos;
    type CompleteNameTreePin = FixtureCompleteNameTreePin;
    type ViewError = Infallible;
    type UnviewError = Infallible;

    fn view_capsule(
        capsule: &Capsule<Self::Kind, Self::CompleteNameTreePin>,
    ) -> Result<Self::TextualRepresentation, Self::ViewError> {
        Ok(OpaqueDocument(
            capsule.content_addressed_hash(),
            *capsule.complete_nametree_pin(),
        ))
    }

    fn unview_capsule(
        textual: &Self::TextualRepresentation,
    ) -> Result<Capsule<Self::Kind, Self::CompleteNameTreePin>, Self::UnviewError> {
        Ok(Capsule::new(textual.0, textual.1))
    }
}

#[test]
fn association_owners_fix_one_capsule_type_while_sharing_is_allowed() {
    fn requires_logos_association<Association>()
    where
        Association: TextualCapsuleAssociation<
                Kind = Logos,
                CompleteNameTreePin = FixtureCompleteNameTreePin,
            >,
    {
    }

    requires_logos_association::<SourceAssociation>();
    requires_logos_association::<DocumentAssociation>();

    let original = logos_capsule();
    let source = SourceAssociation::view_capsule(&original).expect("source view");
    let from_source = SourceAssociation::unview_capsule(&source).expect("source unview");
    assert_eq!(from_source, original);

    let document = DocumentAssociation::view_capsule(&original).expect("document view");
    let from_document = DocumentAssociation::unview_capsule(&document).expect("document unview");
    assert_eq!(from_document, original);
}
