use proptest::prelude::*;
use protos::{
    Bare, Boundary, Delineatable, Delineation, Enclosed, EnclosedContents, Enclosure, Extent,
    Headed, Layout, Portion, PortionForm, Printing, Separator, Symbol, Text,
};

#[derive(Clone, Debug)]
enum Spec {
    Bare(String),
    Headed(String, Separator, Box<Spec>),
    Enclosed(Enclosure, Vec<Spec>),
    Curly(String),
    Parentheses(String),
}

fn render(spec: &Spec) -> String {
    match spec {
        Spec::Bare(value) => value.clone(),
        Spec::Headed(head, separator, body) => {
            format!("{head}{}{}", separator_text(*separator), render(body))
        }
        Spec::Enclosed(enclosure, children) => {
            let (open, close) = enclosure_text(*enclosure);
            format!(
                "{open}{}{close}",
                children.iter().map(render).collect::<Vec<_>>().join(" ")
            )
        }
        Spec::Curly(value) => format!("“{value}”"),
        Spec::Parentheses(value) => format!("({value})"),
    }
}

fn expected(spec: &Spec, start: usize) -> Portion {
    let rendered = render(spec);
    let extent = Extent {
        start,
        end: start + rendered.len(),
    };
    let form = match spec {
        Spec::Bare(value) => PortionForm::Bare(Bare {
            symbol: Symbol::from(value.as_str()),
        }),
        Spec::Headed(head, separator, body) => {
            let body_start = start + head.len() + separator_text(*separator).len();
            PortionForm::Headed(Headed {
                head: Symbol::from(head.as_str()),
                separator: *separator,
                body: Box::new(expected(body, body_start)),
            })
        }
        Spec::Enclosed(enclosure, children) => {
            let mut cursor = start + enclosure_text(*enclosure).0.len();
            let portions = children
                .iter()
                .map(|child| {
                    let portion = expected(child, cursor);
                    cursor = portion.extent.end + 1;
                    portion
                })
                .collect::<Vec<_>>();
            PortionForm::Enclosed(Enclosed {
                boundary: Boundary::Universal(*enclosure),
                arity: portions.len(),
                contents: EnclosedContents::Portions(portions),
            })
        }
        Spec::Curly(value) => PortionForm::Enclosed(Enclosed {
            boundary: Boundary::Universal(Enclosure::CurlyQuote),
            arity: 0,
            contents: EnclosedContents::Opaque(value.clone()),
        }),
        Spec::Parentheses(value) => PortionForm::Enclosed(Enclosed {
            boundary: Boundary::Parentheses,
            arity: 0,
            contents: EnclosedContents::Opaque(value.clone()),
        }),
    };
    Portion { extent, form }
}

fn separator_text(separator: Separator) -> &'static str {
    match separator {
        Separator::Period => ".",
        Separator::Exclamation => "!",
        Separator::Colon => ":",
    }
}

fn enclosure_text(enclosure: Enclosure) -> (&'static str, &'static str) {
    match enclosure {
        Enclosure::Braced => ("{", "}"),
        Enclosure::Bracketed => ("[", "]"),
        Enclosure::Guillemets => ("«", "»"),
        Enclosure::Angled => ("<", ">"),
        Enclosure::CurlyQuote => ("“", "”"),
    }
}

fn spec_strategy() -> impl Strategy<Value = Spec> {
    let bare = "[a-z]{1,5}".prop_map(Spec::Bare);
    bare.prop_recursive(4, 64, 5, |inner| {
        prop_oneof![
            (
                "[A-Z][a-z]{0,4}",
                prop_oneof![
                    Just(Separator::Period),
                    Just(Separator::Exclamation),
                    Just(Separator::Colon)
                ],
                inner.clone()
            )
                .prop_map(|(head, separator, body)| Spec::Headed(
                    head,
                    separator,
                    Box::new(body)
                )),
            (
                prop_oneof![
                    Just(Enclosure::Braced),
                    Just(Enclosure::Bracketed),
                    Just(Enclosure::Guillemets),
                    Just(Enclosure::Angled)
                ],
                prop::collection::vec(inner, 0..4)
            )
                .prop_map(|(enclosure, children)| Spec::Enclosed(enclosure, children)),
            "[a-z .!:\\[\\]{}<>]{0,12}".prop_map(Spec::Curly),
            "[a-z{}\\[\\] ]{0,12}".prop_map(Spec::Parentheses),
        ]
    })
}

proptest! {
    #[test]
    fn delineates_every_throwaway_printed_portion_tree(spec in spec_strategy()) {
        let source = render(&spec);
        let actual = Text::from(source.as_str()).delineate().unwrap();
        let expected = Delineation { portions: vec![expected(&spec, 0)] };
        prop_assert_eq!(actual, expected);
    }

    #[test]
    fn printing_and_delineation_are_identical_on_every_portion_tree(spec in spec_strategy()) {
        let source = render(&spec);
        let delineation = Text::from(source.as_str()).delineate().unwrap();
        let printed = delineation.print(Layout::Flat);
        prop_assert_eq!(printed.as_ref(), source.as_str());
        prop_assert_eq!(printed.delineate().unwrap(), delineation);
    }
}

#[test]
fn faults_report_half_open_utf8_extents() {
    let fault = Text::from("«α").delineate().unwrap_err();
    assert_eq!(fault.extent.start, 0);
    assert_eq!(fault.extent.end, 4);
}

#[test]
fn printer_canonicalizes_non_structural_whitespace() {
    let source = Text::from("  { alpha   [ beta gamma ] }  ");
    let printed = source.delineate().unwrap().print(Layout::Flat);
    assert_eq!(printed.as_ref(), "{alpha [beta gamma]}");
}
