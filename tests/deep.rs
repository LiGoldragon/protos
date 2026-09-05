use protos::{Bare, Delineation, Extent, Head, Protoform, Separator, Situated, Situation, Symbol};

const DEPTH: usize = 20_000;

fn symbol() -> Symbol {
    Symbol::try_from("Node").expect("Node is a symbol")
}

fn deep() -> (Situation, Protoform) {
    let mut situation = Situation {
        extent: Extent(0, 1),
        children: Vec::new(),
    };
    let mut form = Protoform::Bare(Bare::try_from("leaf").expect("leaf is bare"));
    for _ in 0..DEPTH {
        situation = Situation {
            extent: Extent(0, 1),
            children: vec![situation],
        };
        form = Protoform::Headed(Head::Symbol(symbol()), Separator::Period, Box::new(form));
    }
    (situation, form)
}

#[test]
fn recursive_structural_traits_are_iterative() {
    let (situation, form) = deep();
    let delineation = Delineation(vec![Situated(situation, form)]);
    let copied = delineation.clone();
    assert_eq!(delineation, copied);
    assert!(format!("{delineation:?}").starts_with("Delineation([Situated("));
}
