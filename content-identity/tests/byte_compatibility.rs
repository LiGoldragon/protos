//! Byte-compatibility with sema-engine's existing on-disk digests.
//!
//! These tests prove the storage-safe reconciliation: sema-engine's blake3
//! conventions are reproducible bit-for-bit through this crate's types, so
//! migrating sema-engine onto `content-identity` in a later train slice does not
//! move a single stored digest.
//!
//! Two conventions are covered:
//!   1. The plain-hasher composite digest — `RecordKey::update_digest`
//!      (`sema-engine/src/record.rs:109-112`), locked by the byte tests at
//!      `record.rs:200-263` — reproduced through [`IdentityHasher`].
//!   2. The freeform magic-prefix domain — `StoreSchemaHash::from_inventory`
//!      (`sema-engine/src/versioning.rs:127-137`) — reproduced through a
//!      `FrozenMagic` [`HashDomain`].
//!
//! Each digest is asserted two ways: against an independent reconstruction of
//! sema-engine's exact byte layout (the oracle), and against an absolute locked
//! literal (guarding this crate's own folding primitive from silent drift).

use content_identity::{DomainSeparation, HashDomain, IdentityHasher, LayoutVersion};

/// sema-engine's exact store-schema hash domain, reproduced as a `FrozenMagic`
/// domain: `blake3::Hasher::new()` primed with the length-prefixed magic string
/// `sema-engine-store-schema-hash-v1` (versioning.rs:130), the version baked into
/// the string exactly as sema-engine wrote it.
struct StoreSchemaDomain;

impl HashDomain for StoreSchemaDomain {
    fn separation() -> DomainSeparation {
        DomainSeparation::FrozenMagic {
            magic: b"sema-engine-store-schema-hash-v1",
            layout: LayoutVersion::new(1),
        }
    }
}

/// Independent reconstruction of the byte sequence `RecordKey::update_digest`
/// folds (record.rs:109-112): the kind tag byte unframed, then the canonical
/// string length-prefixed little-endian. This mirrors sema-engine's own
/// `expected_digest_input` (record.rs:212-217).
fn sema_engine_record_key_input(tag: u8, canonical: &str) -> Vec<u8> {
    let mut bytes = vec![tag];
    bytes.extend_from_slice(&(canonical.len() as u64).to_le_bytes());
    bytes.extend_from_slice(canonical.as_bytes());
    bytes
}

fn plain_blake3(input: &[u8]) -> [u8; 32] {
    *blake3::hash(input).as_bytes()
}

/// Independent reconstruction of `StoreSchemaHash::from_inventory` for an empty
/// catalog (versioning.rs:127-137): the magic string length-prefixed, then the
/// inventory length `0` as raw little-endian `u64`, with no entries following.
fn sema_engine_empty_store_schema() -> [u8; 32] {
    let mut hasher = blake3::Hasher::new();
    let magic: &[u8] = b"sema-engine-store-schema-hash-v1";
    hasher.update(&(magic.len() as u64).to_le_bytes());
    hasher.update(magic);
    hasher.update(&0u64.to_le_bytes());
    *hasher.finalize().as_bytes()
}

// Absolute locked literals — sema-engine's digests, frozen here so a change to
// this crate's folding primitive can never silently move them.
const RECORD_KEY_DOMAIN_ALPHA: [u8; 32] = [
    0xad, 0xdd, 0x52, 0x2b, 0x7e, 0x1c, 0xc9, 0x52, 0xa4, 0x94, 0x8d, 0x73, 0x87, 0x63, 0x21, 0xaf,
    0xd1, 0xb9, 0x46, 0x3a, 0x4b, 0xf4, 0xc0, 0x89, 0x13, 0x02, 0x2d, 0x09, 0xb3, 0xb9, 0x24, 0xd6,
];
const RECORD_KEY_IDENTIFIER_FORTY_TWO: [u8; 32] = [
    0x88, 0x2e, 0x7f, 0x96, 0xbd, 0xd0, 0x55, 0xaa, 0xd7, 0xfe, 0xc8, 0x3e, 0x2c, 0x00, 0x60, 0xba,
    0x10, 0xa2, 0x5f, 0x06, 0xa4, 0xdd, 0x57, 0x38, 0x89, 0x3f, 0xee, 0x17, 0x2a, 0x80, 0xf9, 0xe3,
];
const STORE_SCHEMA_EMPTY: [u8; 32] = [
    0x1e, 0x15, 0xb6, 0xa4, 0x33, 0xf4, 0xdc, 0xd3, 0xd0, 0x0c, 0x9a, 0x9e, 0xd9, 0xed, 0xca, 0x2d,
    0x41, 0x30, 0x67, 0xd9, 0x38, 0x6d, 0x5a, 0x2f, 0x32, 0x5d, 0x57, 0x4c, 0x46, 0xb6, 0x27, 0xce,
];

#[test]
fn record_key_domain_digest_reproduces_sema_engine_locked_bytes() {
    // Through this crate's reconciled folding primitive.
    let mut hasher = IdentityHasher::unprimed();
    hasher.update_raw(&[1]); // RecordKeyKind::Domain digest tag (record.rs:35)
    hasher.update_length_prefixed(b"alpha");
    let produced = hasher.finalize_bytes();

    // Oracle: sema-engine's own expected byte layout.
    assert_eq!(
        produced,
        plain_blake3(&sema_engine_record_key_input(1, "alpha"))
    );
    // Absolute lock.
    assert_eq!(produced, RECORD_KEY_DOMAIN_ALPHA);
}

#[test]
fn record_key_identifier_digest_reproduces_decimal_string_bytes() {
    // An Identifier key hashes its DECIMAL-string bytes, never the raw u64
    // little-endian bytes (record.rs:238-243).
    let mut hasher = IdentityHasher::unprimed();
    hasher.update_raw(&[2]); // RecordKeyKind::Identifier digest tag
    hasher.update_length_prefixed(b"42");
    let produced = hasher.finalize_bytes();

    assert_eq!(
        produced,
        plain_blake3(&sema_engine_record_key_input(2, "42"))
    );
    assert_eq!(produced, RECORD_KEY_IDENTIFIER_FORTY_TWO);
}

#[test]
fn frozen_magic_domain_reproduces_store_schema_hash_bytes() {
    // Through a FrozenMagic HashDomain: begin() folds the magic length-prefixed,
    // then the empty inventory's raw count.
    let mut hasher = StoreSchemaDomain::separation().begin();
    hasher.update_raw(&0u64.to_le_bytes());
    let produced = hasher.finalize_bytes();

    assert_eq!(produced, sema_engine_empty_store_schema());
    assert_eq!(produced, STORE_SCHEMA_EMPTY);
}

/// Prints the three digests so the locked literals above can be regenerated.
/// Ignored by default; run with `cargo test -- --ignored --nocapture`.
#[test]
#[ignore]
fn print_locked_digests() {
    let hex = |bytes: [u8; 32]| {
        bytes
            .iter()
            .map(|b| format!("0x{b:02x}"))
            .collect::<Vec<_>>()
            .join(", ")
    };
    eprintln!(
        "RECORD_KEY_DOMAIN_ALPHA = {}",
        hex(plain_blake3(&sema_engine_record_key_input(1, "alpha")))
    );
    eprintln!(
        "RECORD_KEY_IDENTIFIER_FORTY_TWO = {}",
        hex(plain_blake3(&sema_engine_record_key_input(2, "42")))
    );
    eprintln!(
        "STORE_SCHEMA_EMPTY = {}",
        hex(sema_engine_empty_store_schema())
    );
}
