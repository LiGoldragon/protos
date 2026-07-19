//! Portable archive round-trip of one owned namespace slice.

use name_table::{IdentifierNamespace, Name, NameTable};

fn populated() -> NameTable {
    let mut table = NameTable::new(IdentifierNamespace::Schema);
    table.intern(Name::new("CommitSequence"));
    table.intern(Name::new("Field"));
    table.intern(Name::new("TypeReference"));
    table
}

#[test]
fn a_populated_home_slice_round_trips_through_portable_archive() {
    let table = populated();
    let bytes = table.to_archive_bytes().expect("serialize");
    let restored = NameTable::from_archive_bytes(bytes.as_ref()).expect("deserialize");
    assert_eq!(table, restored);
    assert_eq!(restored.namespace(), IdentifierNamespace::Schema);
}

#[test]
fn round_trip_preserves_every_variant_identifier() {
    let table = populated();
    let restored =
        NameTable::from_archive_bytes(table.to_archive_bytes().unwrap().as_ref()).unwrap();
    for local in 0..u16::try_from(table.len()).unwrap() {
        let identifier = IdentifierNamespace::Schema.identifier(local);
        assert_eq!(
            table.resolve(identifier).unwrap(),
            restored.resolve(identifier).unwrap()
        );
    }
}

#[test]
fn identity_is_stable_across_a_round_trip() {
    let table = populated();
    let restored =
        NameTable::from_archive_bytes(table.to_archive_bytes().unwrap().as_ref()).unwrap();
    assert_eq!(table.identity().unwrap(), restored.identity().unwrap());
}

#[test]
fn tables_with_different_names_have_different_identities() {
    let table = populated();
    let mut other = NameTable::new(IdentifierNamespace::Schema);
    other.intern(Name::new("CommitSequence"));
    other.intern(Name::new("Field"));
    other.intern(Name::new("Renamed"));
    assert_ne!(table.identity().unwrap(), other.identity().unwrap());
}
