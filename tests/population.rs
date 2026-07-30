use protos::EncodedPopulation;

#[derive(
    Clone, Debug, Eq, PartialEq, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize,
)]
struct FixtureEncodedForm(Vec<u16>);

#[derive(
    Clone, Debug, Eq, PartialEq, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize,
)]
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
