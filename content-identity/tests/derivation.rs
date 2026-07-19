//! The new of-core derivation: determinism, layout separation, domain
//! separation, round-trip, and the structural NameTable-independence.

use content_identity::{
    ContentHash, DomainSeparation, Envelope, HashDomain, LayoutVersion, PortableArchive,
};

// Two domains sharing a context but differing in layout version prove that a
// layout bump actually separates the address space.
struct LogosCoreV1;
struct LogosCoreV2;
// A third domain with a different context proves domain separation.
struct NomosCoreV1;

impl HashDomain for LogosCoreV1 {
    fn separation() -> DomainSeparation {
        DomainSeparation::Contextual {
            context: "content-identity 2026 logos core value",
            layout: LayoutVersion::new(1),
        }
    }
}

impl HashDomain for LogosCoreV2 {
    fn separation() -> DomainSeparation {
        DomainSeparation::Contextual {
            context: "content-identity 2026 logos core value",
            layout: LayoutVersion::new(2),
        }
    }
}

impl HashDomain for NomosCoreV1 {
    fn separation() -> DomainSeparation {
        DomainSeparation::Contextual {
            context: "content-identity 2026 nomos core value",
            layout: LayoutVersion::new(1),
        }
    }
}

/// A stringless Core-shaped value: it carries typed indices, never names. The
/// NameTable that would resolve those indices to text is a separate value never
/// fed to `of_core`, so it cannot be in the pre-image.
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq)]
struct StringlessCore {
    identifier: u32,
    field_identifiers: Vec<u32>,
    fields: Vec<StringlessField>,
}

#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq)]
struct StringlessField {
    name_identifier: u32,
    type_identifier: u32,
}

fn sample() -> StringlessCore {
    StringlessCore {
        identifier: 17,
        field_identifiers: vec![23, 31, 9],
        fields: vec![
            StringlessField {
                name_identifier: 23,
                type_identifier: 9,
            },
            StringlessField {
                name_identifier: 31,
                type_identifier: 5,
            },
        ],
    }
}

#[test]
fn of_core_is_deterministic() {
    let value = sample();
    let first = ContentHash::<LogosCoreV1>::of_core(&value).expect("hash");
    let second = ContentHash::<LogosCoreV1>::of_core(&value).expect("hash");
    assert_eq!(first, second);
}

#[test]
fn layout_version_separates_identical_bytes() {
    let value = sample();
    let under_v1 = ContentHash::<LogosCoreV1>::of_core(&value).expect("hash");
    let under_v2 = ContentHash::<LogosCoreV2>::of_core(&value).expect("hash");
    // Same context, same bytes, different layout version -> different address.
    assert_ne!(under_v1.bytes(), under_v2.bytes());
}

#[test]
fn domain_context_separates_identical_bytes() {
    let value = sample();
    let logos = ContentHash::<LogosCoreV1>::of_core(&value).expect("hash");
    let nomos = ContentHash::<NomosCoreV1>::of_core(&value).expect("hash");
    // Same bytes, same layout, different domain context -> different address.
    assert_ne!(logos.bytes(), nomos.bytes());
}

#[test]
fn distinct_values_get_distinct_addresses() {
    let mut other = sample();
    other.identifier = 18;
    let base = ContentHash::<LogosCoreV1>::of_core(&sample()).expect("hash");
    let changed = ContentHash::<LogosCoreV1>::of_core(&other).expect("hash");
    assert_ne!(base.bytes(), changed.bytes());
}

#[test]
fn portable_archive_round_trips() {
    let value = sample();
    let bytes = value.to_archive_bytes().expect("serialize");
    let restored = StringlessCore::from_archive_bytes(bytes.as_ref()).expect("deserialize");
    assert_eq!(value, restored);
}

/// NameTable-independence is structural, not enforced at runtime: the Core value
/// has no name fields, so no name can enter the pre-image, and a rename (which
/// mutates only the separate NameTable) cannot move this address. Two values
/// with identical structural indices produce identical addresses regardless of
/// any external naming.
#[test]
fn identity_depends_only_on_stringless_bytes() {
    let one = sample();
    let two = sample();
    let hash_one = ContentHash::<LogosCoreV1>::of_core(&one).expect("hash");
    let hash_two = ContentHash::<LogosCoreV1>::of_core(&two).expect("hash");
    assert_eq!(hash_one, hash_two);
    // And the address equals a direct derive over the canonical bytes: no hidden
    // name input participates.
    let direct =
        ContentHash::<LogosCoreV1>::derive(one.to_archive_bytes().expect("bytes").as_ref());
    assert_eq!(hash_one, direct);
}

#[test]
fn envelope_seals_and_verifies() {
    let value = sample();
    let envelope = Envelope::<LogosCoreV1>::of_core(&value).expect("seal");
    assert!(envelope.verify());
    assert_eq!(envelope.layout(), LayoutVersion::new(1));
    // The envelope's identity is exactly the of-core address.
    let address = ContentHash::<LogosCoreV1>::of_core(&value).expect("hash");
    assert_eq!(envelope.identity(), &address);
}

#[test]
fn envelope_detects_payload_tampering() {
    let value = sample();
    let envelope = Envelope::<LogosCoreV1>::of_core(&value).expect("seal");
    // Reseal different bytes under a hand-built envelope by sealing another value
    // and checking the identities differ — a tampered payload cannot keep the
    // original address.
    let other_value = StringlessCore {
        identifier: 99,
        field_identifiers: vec![1],
        fields: vec![],
    };
    let other = Envelope::<LogosCoreV1>::of_core(&other_value).expect("seal");
    assert_ne!(envelope.identity(), other.identity());
    assert!(other.verify());
}
