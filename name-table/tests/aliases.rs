//! Transparent aliases are NameTree data, not encoded declaration nodes.

use name_table::{IdentifierNamespace, Name, NameTable};

#[test]
fn a_transparent_alias_decodes_to_its_target_identifier_and_remains_emittable() {
    let mut schema = NameTable::new(IdentifierNamespace::Schema);
    let commit_log = schema.intern(Name::new("CommitLog"));
    schema
        .add_alias(commit_log, Name::new("Journal"))
        .expect("the schema owns the target");

    // Decode resolves either spelling to the same structural identifier: the
    // encoded reference graph contains no alias declaration or alias identifier.
    assert_eq!(schema.lookup(&Name::new("CommitLog")), Some(commit_log));
    assert_eq!(schema.lookup(&Name::new("Journal")), Some(commit_log));

    // The canonical name and extra NameTree name are both retained, so a Rust
    // textual projection has the full transparent alias relation to emit.
    assert_eq!(schema.resolve(commit_log).unwrap().as_str(), "CommitLog");
    assert_eq!(
        schema
            .aliases(commit_log)
            .unwrap()
            .iter()
            .map(Name::as_str)
            .collect::<Vec<_>>(),
        vec!["Journal"]
    );
}

#[test]
fn a_composed_consumer_borrows_alias_resolution_without_copying_it() {
    let mut schema = NameTable::new(IdentifierNamespace::Schema);
    let target = schema.intern(Name::new("CommitLog"));
    schema
        .add_alias(target, Name::new("Journal"))
        .expect("the schema owns the target");
    let logos = NameTable::new(IdentifierNamespace::Logos)
        .compose(&schema)
        .expect("borrow schema name slice");

    assert_eq!(logos.lookup(&Name::new("Journal")), Some(target));
    assert_eq!(
        logos.aliases(target).unwrap(),
        schema.aliases(target).unwrap()
    );
}
