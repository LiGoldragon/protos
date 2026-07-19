//! Composed slices borrow source identifiers instead of cloning their table.

use name_table::{Identifier, IdentifierNamespace, Name, NameTable, NameTableError};

#[test]
fn composition_keeps_borrowed_schema_identifiers_stable() {
    let mut schema = NameTable::new(IdentifierNamespace::Schema);
    let sequence = schema.intern(Name::new("CommitSequence"));
    let field = schema.intern(Name::new("Field"));

    let logos = NameTable::new(IdentifierNamespace::Logos)
        .compose(&schema)
        .expect("compose schema slice");

    assert_eq!(sequence, Identifier::Schema(0));
    assert_eq!(
        logos.resolve(sequence).unwrap(),
        schema.resolve(sequence).unwrap()
    );
    assert_eq!(logos.resolve(field).unwrap().as_str(), "Field");
}

#[test]
fn target_names_allocate_only_in_the_target_home_namespace() {
    let mut schema = NameTable::new(IdentifierNamespace::Schema);
    schema.intern(Name::new("CommitSequence"));
    let mut logos = NameTable::new(IdentifierNamespace::Logos)
        .compose(&schema)
        .expect("compose schema slice");

    let logos_only = logos.intern(Name::new("LogosOnly"));

    assert_eq!(logos_only, Identifier::Logos(0));
    assert_eq!(logos.resolve(logos_only).unwrap().as_str(), "LogosOnly");
    assert!(schema.resolve(logos_only).is_err());
}

#[test]
fn a_borrowed_name_resolves_without_copying_or_reinterning() {
    let mut schema = NameTable::new(IdentifierNamespace::Schema);
    let field = schema.intern(Name::new("Field"));
    let mut logos = NameTable::new(IdentifierNamespace::Logos)
        .compose(&schema)
        .expect("compose schema slice");

    assert_eq!(logos.intern(Name::new("Field")), field);
    assert_eq!(
        logos.len(),
        0,
        "borrowing did not append a copied schema name"
    );
}

#[test]
fn a_namespace_can_only_be_composed_once() {
    let schema = NameTable::new(IdentifierNamespace::Schema);
    let logos = NameTable::new(IdentifierNamespace::Logos)
        .compose(&schema)
        .expect("first compose");

    assert!(matches!(
        logos.compose(&schema),
        Err(NameTableError::DuplicateNamespace(
            IdentifierNamespace::Schema
        ))
    ));
}
