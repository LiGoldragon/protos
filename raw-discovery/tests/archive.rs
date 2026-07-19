//! Witness that a recognized [`Document`] round-trips through the portable rkyv
//! archive discipline — the exact little-endian / 32-bit-pointer / unaligned
//! feature set, with validation on read. That discipline is content-identity's
//! shared [`PortableArchive`] bound, which this crate adopts: `Document` earns the
//! blanket impl and the round trip runs through `to_archive_bytes` /
//! `from_archive_bytes`.

use content_identity::PortableArchive;
use raw_discovery::{Document, Recognizer};

fn round_trips(source: &str) {
    let document = Recognizer::standard()
        .recognize(source)
        .expect("valid nota structure");
    let bytes = document
        .to_archive_bytes()
        .expect("document serializes to bytes");
    let restored =
        Document::from_archive_bytes(&bytes).expect("archived bytes validate and deserialize");
    assert_eq!(
        restored, document,
        "round trip preserves the structure for {source:?}"
    );
}

#[test]
fn a_nested_document_round_trips_through_rkyv() {
    // Every block shape at once: delimiters nested three deep, a
    // right-associative dotted chain, pipe text, and bare atoms.
    round_trips(
        "Public.Newtype.( CommitSequence [ rkyv.Archive Clone ] (|literal ] body|) ) trailing",
    );
}

#[test]
fn each_block_shape_round_trips() {
    for source in [
        "alpha",
        "(a b c)",
        "[a b c]",
        "{a b c}",
        "head.payload",
        "a.b.c",
        "(|pipe text|)",
    ] {
        round_trips(source);
    }
}
