use proptest::prelude::*;
use protos::{
    Bare, Boundary, ContentHashable, Delineatable, Delineation, DialectBoundary, Embodiable,
    Embodied, Enclosed, EnclosedContents, Enclosure, Extent, Fault, FaultProblem, Headed, Layout,
    Portion, Printing, Prospective, Separator, ShapeDefined, Symbol, Text, Textualizable,
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
    match spec {
        Spec::Bare(value) => Portion::Bare(
            extent,
            Bare {
                symbol: Symbol::from(value.as_str()),
            },
        ),
        Spec::Headed(head, separator, body) => {
            let body_start = start + head.len() + separator_text(*separator).len();
            Portion::Headed(
                extent,
                Headed {
                    head: Symbol::from(head.as_str()),
                    separator: *separator,
                    body: Box::new(expected(body, body_start)),
                },
            )
        }
        Spec::Enclosed(enclosure, children) => {
            let mut cursor = start + enclosure_text(*enclosure).0.len();
            let portions = children
                .iter()
                .map(|child| {
                    let portion = expected(child, cursor);
                    cursor = portion.as_ref().end + 1;
                    portion
                })
                .collect::<Vec<_>>();
            Portion::Enclosed(
                extent,
                Enclosed {
                    boundary: Boundary::Universal(*enclosure),
                    arity: portions.len(),
                    contents: EnclosedContents::Portions(portions),
                },
            )
        }
        Spec::Curly(value) => Portion::Enclosed(
            extent,
            Enclosed {
                boundary: Boundary::Universal(Enclosure::CurlyQuote),
                arity: 0,
                contents: EnclosedContents::Opaque(value.clone()),
            },
        ),
        Spec::Parentheses(value) => Portion::Enclosed(
            extent,
            Enclosed {
                boundary: Boundary::Dialect(DialectBoundary::Parentheses),
                arity: 0,
                contents: EnclosedContents::Opaque(value.clone()),
            },
        ),
    }
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
fn parentheses_balance_and_escape_opaque_content_with_utf8_extents() {
    let source = "(α(β)γ\\))";
    let delineation = Text::from(source).delineate().unwrap();
    let portion = &delineation.portions[0];
    match portion {
        Portion::Enclosed(extent, enclosed) => {
            assert_eq!(extent.start, 0);
            assert_eq!(extent.end, source.len());
            assert_eq!(
                enclosed.boundary,
                Boundary::Dialect(DialectBoundary::Parentheses)
            );
            assert_eq!(
                enclosed.contents,
                EnclosedContents::Opaque("α(β)γ)".to_owned())
            );
        }
        _ => panic!("parenthetical text must delineate as an enclosed Portion"),
    }
    assert_eq!(portion.print(Layout::Flat).as_ref(), source);
}

#[test]
fn structural_faults_locate_closers_openers_and_malformed_heads() {
    let mismatched = Text::from("{alpha]").delineate().unwrap_err();
    assert_eq!(mismatched.problem, FaultProblem::UnexpectedCloser);
    assert_eq!(mismatched.extent.start, 6);
    assert_eq!(mismatched.extent.end, 7);

    let curly = Text::from("“α").delineate().unwrap_err();
    assert_eq!(curly.problem, FaultProblem::UnclosedDelimiter);
    assert_eq!(curly.extent.start, 0);
    assert_eq!(curly.extent.end, 5);

    let parentheses = Text::from("(α").delineate().unwrap_err();
    assert_eq!(parentheses.problem, FaultProblem::UnclosedDelimiter);
    assert_eq!(parentheses.extent.start, 0);
    assert_eq!(parentheses.extent.end, 3);

    let head = Text::from(".alpha").delineate().unwrap_err();
    assert_eq!(head.problem, FaultProblem::MissingHead);
    assert_eq!(head.extent.start, 0);
    assert_eq!(head.extent.end, 1);
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
        let enclosed = match portion {
            Portion::Enclosed(_, enclosed)
                if enclosed.boundary == Boundary::Universal(Enclosure::Braced) =>
            {
                enclosed
            }
            _ => {
                return Err(Fault {
                    extent: Extent {
                        start: portion.as_ref().start,
                        end: portion.as_ref().end,
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
                        start: portion.as_ref().start,
                        end: portion.as_ref().end,
                    },
                    problem: FaultProblem::ExpectedShape,
                });
            }
        };
        let name = match &fields[0] {
            Portion::Bare(_, bare) => bare.symbol.as_ref().to_owned(),
            _ => {
                return Err(Fault {
                    extent: Extent {
                        start: fields[0].as_ref().start,
                        end: fields[0].as_ref().end,
                    },
                    problem: FaultProblem::ExpectedShape,
                });
            }
        };
        let count = match &fields[1] {
            Portion::Bare(_, bare) => bare.symbol.as_ref().to_owned(),
            _ => {
                return Err(Fault {
                    extent: Extent {
                        start: fields[1].as_ref().start,
                        end: fields[1].as_ref().end,
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
        Portion::Enclosed(
            Extent { start: 0, end: 0 },
            Enclosed {
                boundary: Boundary::Universal(Enclosure::Braced),
                arity: 2,
                contents: EnclosedContents::Portions(vec![
                    Portion::Bare(
                        Extent { start: 0, end: 0 },
                        Bare {
                            symbol: Symbol::from(self.name.as_str()),
                        },
                    ),
                    Portion::Bare(
                        Extent { start: 0, end: 0 },
                        Bare {
                            symbol: Symbol::from(self.count.as_str()),
                        },
                    ),
                ]),
            },
        )
    }
}

impl ShapeDefined for ToyRecord {
    fn matches(portion: &Portion) -> bool {
        matches!(portion, Portion::Enclosed(_, enclosed) if enclosed.boundary == Boundary::Universal(Enclosure::Braced) && enclosed.arity == 2)
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
