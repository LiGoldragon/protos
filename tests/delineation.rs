use proptest::prelude::*;
use protos::{
    Bare, Boundary, ContentHashable, Delineatable, Delineation, Embodiable, Embodied, Enclosed,
    EnclosedContents, Enclosure, Extent, Fault, FaultProblem, Headed, Layout, Portion, PortionForm,
    Printing, Prospective, Separator, ShapeDefined, Symbol, Text, Textualizable,
};
use std::fmt;

enum Spec {
    Bare(String),
    Headed(String, Separator, Box<Spec>),
    Enclosed(Enclosure, Vec<Spec>),
    Curly(String),
    Parentheses(String),
}

impl Clone for Spec {
    fn clone(&self) -> Self {
        match self {
            Self::Bare(value) => Self::Bare(value.to_owned()),
            Self::Headed(head, separator, body) => {
                Self::Headed(head.to_owned(), *separator, Box::new((**body).clone()))
            }
            Self::Enclosed(enclosure, children) => {
                Self::Enclosed(*enclosure, children.iter().map(Clone::clone).collect())
            }
            Self::Curly(value) => Self::Curly(value.to_owned()),
            Self::Parentheses(value) => Self::Parentheses(value.to_owned()),
        }
    }
}

impl fmt::Debug for Spec {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "throwaway Portion specification")
    }
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

#[test]
fn text_normalizes_only_non_structural_whitespace_and_hashes_the_normalized_content() {
    let normalized = Text::from("  { alpha   [ beta gamma ] }  ");
    let canonical = Text::from("{alpha [beta gamma]}");
    let opaque = Text::from("“ keep   these spaces ”");
    assert_eq!(normalized.as_ref(), canonical.as_ref());
    assert_eq!(normalized.content_hash(), canonical.content_hash());
    assert_eq!(opaque.as_ref(), "“ keep   these spaces ”");
}

struct ToyRecord {
    name: String,
    count: String,
}

impl Embodied for ToyRecord {
    fn from_portion(portion: &Portion) -> Result<Self, Fault> {
        let enclosed = match &portion.form {
            PortionForm::Enclosed(enclosed)
                if enclosed.boundary == Boundary::Universal(Enclosure::Braced) =>
            {
                enclosed
            }
            _ => {
                return Err(Fault {
                    extent: Extent {
                        start: portion.extent.start,
                        end: portion.extent.end,
                    },
                    problem: FaultProblem::ExpectedShape,
                });
            }
        };
        let fields = match &enclosed.contents {
            EnclosedContents::Portions(fields) if fields.len() == 2 => fields,
            _ => {
                return Err(Fault {
                    extent: Extent {
                        start: portion.extent.start,
                        end: portion.extent.end,
                    },
                    problem: FaultProblem::ExpectedShape,
                });
            }
        };
        let name = match &fields[0].form {
            PortionForm::Bare(bare) => bare.symbol.as_ref().to_owned(),
            _ => {
                return Err(Fault {
                    extent: Extent {
                        start: fields[0].extent.start,
                        end: fields[0].extent.end,
                    },
                    problem: FaultProblem::ExpectedShape,
                });
            }
        };
        let count = match &fields[1].form {
            PortionForm::Bare(bare) => bare.symbol.as_ref().to_owned(),
            _ => {
                return Err(Fault {
                    extent: Extent {
                        start: fields[1].extent.start,
                        end: fields[1].extent.end,
                    },
                    problem: FaultProblem::ExpectedShape,
                });
            }
        };
        Ok(Self { name, count })
    }
}

impl Textualizable for ToyRecord {
    fn to_portion(&self) -> Portion {
        Portion {
            extent: Extent { start: 0, end: 0 },
            form: PortionForm::Enclosed(Enclosed {
                boundary: Boundary::Universal(Enclosure::Braced),
                arity: 2,
                contents: EnclosedContents::Portions(vec![
                    Portion {
                        extent: Extent { start: 0, end: 0 },
                        form: PortionForm::Bare(Bare {
                            symbol: Symbol::from(self.name.as_str()),
                        }),
                    },
                    Portion {
                        extent: Extent { start: 0, end: 0 },
                        form: PortionForm::Bare(Bare {
                            symbol: Symbol::from(self.count.as_str()),
                        }),
                    },
                ]),
            }),
        }
    }
}

impl ShapeDefined for ToyRecord {
    fn matches(portion: &Portion) -> bool {
        matches!(&portion.form, PortionForm::Enclosed(enclosed) if enclosed.boundary == Boundary::Universal(Enclosure::Braced) && enclosed.arity == 2)
    }
}

#[test]
fn a_dialect_embodies_and_textualizes_through_portions_without_handling_characters() {
    let prospective: Prospective<ToyRecord> = Text::from(" { north 42 } ").into();
    let record = prospective.embody().unwrap();
    assert_eq!(record.name, "north");
    assert_eq!(record.count, "42");
    assert_eq!(record.textualize().as_ref(), "{north 42}");
    let portion = Text::from("{north 42}")
        .delineate()
        .unwrap()
        .portions
        .remove(0);
    assert!(<ToyRecord as ShapeDefined>::matches(&portion));
}
