//! A built tree is as deep as what built it. Every traversal of `Protos` must
//! therefore be iterative, and this runs each of them past any stack that
//! recursion would have.

use protos::{Extent, ExtentAccess, Position, Protos, Separator, Symbol, Textualizable};

const DEPTH: usize = 100_000;
fn nowhere() -> Extent {
    Extent::new(
        Position::try_from(0).unwrap(),
        Position::try_from(0).unwrap(),
    )
    .unwrap()
}

fn deep(depth: usize) -> Protos {
    let mut form = Protos::Bare {
        extent: nowhere(),
        text: String::from("leaf"),
    };
    for _ in 0..depth {
        form = Protos::Headed {
            extent: nowhere(),
            head: Symbol(String::from("Node")),
            constraints: None,
            separator: Separator::Period,
            body: Box::new(form),
        };
    }
    form
}

/// Depth alone, through every derived-shape trait at once.
#[test]
fn cloning_comparing_and_showing_a_deep_tree_are_iterative() {
    let form = deep(DEPTH);
    let copied = form.clone();
    assert_eq!(form, copied);
    let shown = format!("{form:?}");
    assert!(shown.starts_with("Headed { extent: Extent { start: Position(0), end: Position(0) }, head: Symbol(\"Node\"), constraints: None, separator: Period, body: "));
    assert!(shown.contains(
        "Bare { extent: Extent { start: Position(0), end: Position(0) }, text: \"leaf\" }"
    ));
    assert!(shown.ends_with(&" }".repeat(DEPTH)));
    assert_eq!(form.textualize(), format!("{}leaf", "Node.".repeat(DEPTH)));
    drop(copied);
    drop(form);
}

/// Inequality must be decided without descending recursively either, and a
/// difference at the very bottom is the case that descends furthest.
#[test]
fn comparing_deep_trees_that_differ_at_the_leaf_is_iterative() {
    let form = deep(DEPTH);
    let mut other = deep(DEPTH);
    let mut node = &mut other;
    while let Protos::Headed { body, .. } = node {
        node = body;
    }
    *node = Protos::Bare {
        extent: nowhere(),
        text: String::from("other"),
    };
    assert_ne!(form, other);
}

/// Width, not depth: the same traversals over one very wide enclosure.
#[test]
fn cloning_and_comparing_a_wide_tree_hold_their_shape() {
    let wide = Protos::Enclosed {
        extent: nowhere(),
        enclosure: protos::Enclosure::Bracketed,
        children: (0..50_000)
            .map(|index| Protos::Bare {
                extent: nowhere(),
                text: index.to_string(),
            })
            .collect(),
    };
    let copied = wide.clone();
    assert_eq!(wide, copied);
    let Protos::Enclosed { children, .. } = &copied else {
        panic!("enclosed")
    };
    assert_eq!(children.len(), 50_000);
    assert_eq!(
        children.last(),
        Some(&Protos::Bare {
            extent: nowhere(),
            text: String::from("49999"),
        })
    );
}

/// Constrained heads carry a second child; cloning must keep it on the right
/// node and in the right place when the traversal is a flat stack.
#[test]
fn a_deep_tree_of_constrained_heads_clones_into_the_same_tree() {
    let mut form = Protos::Bare {
        extent: nowhere(),
        text: String::from("leaf"),
    };
    for index in 0..10_000 {
        form = Protos::Headed {
            extent: nowhere(),
            head: Symbol(String::from("Node")),
            constraints: Some(Box::new(Protos::Enclosed {
                extent: nowhere(),
                enclosure: protos::Enclosure::Angled,
                children: vec![Protos::Bare {
                    extent: nowhere(),
                    text: index.to_string(),
                }],
            })),
            separator: Separator::Period,
            body: Box::new(form),
        };
    }
    let copied = form.clone();
    assert_eq!(form, copied);
    assert_eq!(copied.textualize(), form.textualize());
}
