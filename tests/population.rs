use content_identity::PortableArchive;
use protos::EncodedPopulation;

#[derive(Clone, Debug, Eq, PartialEq, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize)]
struct FixtureEncodedForm(Vec<u16>);

#[derive(Clone, Debug, Eq, PartialEq, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize)]
struct FixtureNameTree(Vec<(u16, String)>);

#[test]
fn population_retains_both_complete_positional_values() {
    let encoded_form = FixtureEncodedForm(vec![1, 3, 9]);
    let name_tree = FixtureNameTree(vec![(9, "Widget".to_owned())]);
    let population = EncodedPopulation::new(encoded_form.clone(), name_tree.clone());

    assert_eq!(population.encoded_form(), &encoded_form);
    assert_eq!(population.name_tree(), &name_tree);
    assert_eq!(population.into_parts(), (encoded_form, name_tree));
}

#[test]
fn population_has_one_validated_portable_archive_round_trip() {
    type FixturePopulation = EncodedPopulation<FixtureEncodedForm, FixtureNameTree>;

    let original = FixturePopulation::new(
        FixtureEncodedForm(vec![2, 6, 18]),
        FixtureNameTree(vec![(18, "ArchiveWitness".to_owned())]),
    );
    let bytes = original
        .to_archive_bytes()
        .expect("archive neutral population carrier");
    let restored = FixturePopulation::from_archive_bytes(&bytes)
        .expect("validate and restore neutral population carrier");

    assert_eq!(restored, original);
    assert!(FixturePopulation::from_archive_bytes(&bytes[..bytes.len() - 1]).is_err());
}
