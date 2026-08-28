use proptest::prelude::*;
use protos::{
    Bare, BareExpectation, BareSafe, ContentHashable, Delineatable, DialectBoundary, Embodiable,
    Embodied, Enclosed, EnclosedAnatomy, EnclosedArity, Extent, Fault, FaultProblem, Headed,
    Layout, OpaqueBoundary, OpaqueEnclosed, Portion, PortionText, Printing, Separator,
    ShapeDefined, StructuralEnclosed, StructuralEnclosure, Symbol, Text, Textualizable,
};
use std::fmt;

fn text(value: &str) -> Text {
    Text::<()>::from(value)
}

fn symbol(value: &str) -> Symbol {
    Symbol::try_from(value).expect("test symbols are Protos bare values")
}

enum Spec {
    Bare(String),
    Headed(String, Separator, Box<Spec>),
    Structural(StructuralEnclosure, Vec<Spec>),
}

impl Clone for Spec {
    fn clone(&self) -> Self {
        match self {
            Self::Bare(value) => Self::Bare(value.to_owned()),
            Self::Headed(head, separator, body) => {
                Self::Headed(head.to_owned(), *separator, Box::new((**body).clone()))
            }
            Self::Structural(enclosure, children) => {
                Self::Structural(*enclosure, children.iter().map(Clone::clone).collect())
            }
        }
    }
}

impl fmt::Debug for Spec {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "valid public Portion specification")
    }
}

fn portion(spec: &Spec) -> Portion {
    match spec {
        Spec::Bare(value) => Portion::from(Bare::from(symbol(value))),
        Spec::Headed(head, separator, body) => {
            Portion::from(Headed::from((symbol(head), *separator, portion(body))))
        }
        Spec::Structural(enclosure, children) => {
            let portions = children.iter().map(portion).collect();
            Portion::from(Enclosed::from(StructuralEnclosed::from((
                *enclosure, portions,
            ))))
        }
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
                    Just(Separator::Colon),
                ],
                inner.clone(),
            )
                .prop_map(|(head, separator, body)| Spec::Headed(
                    head,
                    separator,
                    Box::new(body)
                )),
            (
                prop_oneof![
                    Just(StructuralEnclosure::Braced),
                    Just(StructuralEnclosure::Bracketed),
                    Just(StructuralEnclosure::Guillemets),
                    Just(StructuralEnclosure::Angled),
                ],
                prop::collection::vec(inner, 0..4),
            )
                .prop_map(|(enclosure, children)| Spec::Structural(enclosure, children)),
        ]
    })
}

proptest! {
    #[test]
    fn every_publicly_constructed_portion_round_trips(spec in spec_strategy()) {
        let portion = portion(&spec);
        let printed = portion.print(Layout::Flat);
        let delineated = printed.delineate().expect("the sole reader accepts the sole writer");
        prop_assert_eq!(delineated.portions.as_slice(), &[portion]);
        let reprinted = delineated.print(Layout::Flat);
        prop_assert_eq!(reprinted.as_ref(), printed.as_ref());
    }
}

#[test]
fn normalization_is_a_canonical_projection_through_reader_and_writer() {
    let adjacent = text("{a[b]}");
    assert_eq!(adjacent.as_ref(), "{a [b]}");
    assert_eq!(
        adjacent.delineate().unwrap().print(Layout::Flat).as_ref(),
        "{a [b]}"
    );

    let roots = text("{}[]");
    assert_eq!(roots.as_ref(), "{} []");
    assert_eq!(
        roots.delineate().unwrap().print(Layout::Flat).as_ref(),
        "{} []"
    );

    let comments = text("alpha ;; dropped\nbeta");
    assert_eq!(comments.as_ref(), "alpha beta");
}

#[test]
fn all_delimiters_and_separators_have_external_canonical_examples() {
    for source in [
        "alpha.beta",
        "alpha!beta",
        "alpha:beta",
        "{alpha beta}",
        "[alpha beta]",
        "«alpha beta»",
        "<alpha beta>",
        "“alpha “beta” gamma”",
        "(alpha(β)gamma\\))",
    ] {
        let value = text(source);
        assert_eq!(
            value.delineate().unwrap().print(Layout::Flat).as_ref(),
            source
        );
    }
}

#[test]
fn opaque_construction_is_boundary_specific_and_validated_by_the_pipeline() {
    let parentheses = OpaqueEnclosed::try_from((
        OpaqueBoundary::Dialect(DialectBoundary::Parentheses),
        "α)β".to_owned(),
    ))
    .unwrap();
    let portion = Portion::from(Enclosed::from(parentheses));
    assert_eq!(portion.print(Layout::Flat).as_ref(), "(α\\)β)");
    assert_eq!(
        portion
            .print(Layout::Flat)
            .delineate()
            .unwrap()
            .portions
            .as_slice(),
        &[portion]
    );
    assert!(OpaqueEnclosed::try_from((OpaqueBoundary::CurlyQuote, "a “ b".to_owned())).is_err());
}

#[test]
fn faults_report_half_open_utf8_extents() {
    let mismatched = text("{alpha]").delineate().unwrap_err();
    assert_eq!(mismatched.problem, FaultProblem::UnexpectedCloser);
    assert_eq!(mismatched.extent, Extent { start: 6, end: 7 });

    let curly = text("“α").delineate().unwrap_err();
    assert_eq!(curly.problem, FaultProblem::UnclosedDelimiter);
    assert_eq!(curly.extent, Extent { start: 0, end: 5 });

    let parentheses = text("(α").delineate().unwrap_err();
    assert_eq!(parentheses.problem, FaultProblem::UnclosedDelimiter);
    assert_eq!(parentheses.extent, Extent { start: 0, end: 3 });

    let malformed_head = text(".alpha").delineate().unwrap_err();
    assert_eq!(malformed_head.problem, FaultProblem::MissingHead);
    assert_eq!(malformed_head.extent, Extent { start: 0, end: 1 });
}

#[test]
fn bare_safety_and_symbol_construction_are_protos_anatomy_questions() {
    assert!(text("alpha").is_bare_safe_for(BareExpectation::Symbol));
    assert!(!text("alpha beta").is_bare_safe_for(BareExpectation::Symbol));
    assert!(!text("[alpha]").is_bare_safe_for(BareExpectation::Symbol));
    assert!(!text("alpha.beta").is_bare_safe_for(BareExpectation::Symbol));
    assert!(Symbol::try_from("alpha beta").is_err());
    assert!(Symbol::try_from("alpha]").is_err());
}

#[test]
fn string_bare_safety_preserves_one_load_bearing_portion_without_dialect_scanning() {
    for source in ["alpha.beta", "alpha!beta", "alpha:beta"] {
        let value = text(source);
        assert!(value.is_bare_safe_for(BareExpectation::String));
        let delineation = value.delineate().unwrap();
        assert_eq!(delineation.portions.len(), 1);
        assert_eq!(delineation.portions[0].canonical_text().as_ref(), source);
        assert_eq!(
            delineation.portions[0]
                .canonical_text()
                .delineate()
                .unwrap(),
            delineation
        );
    }
    assert!(!text("alpha beta").is_bare_safe_for(BareExpectation::String));
}

struct ToyString(String);

impl Embodied for ToyString {
    fn from_portion(portion: &Portion) -> Result<Self, Fault> {
        Ok(Self(portion.canonical_text().as_ref().to_owned()))
    }
}

impl Textualizable for ToyString {
    fn to_portion(&self) -> Portion {
        let text = Text::<()>::from(self.0.as_str());
        assert!(text.is_bare_safe_for(BareExpectation::String));
        text.delineate().unwrap().portions.remove(0)
    }
}

#[test]
fn toy_string_dialect_embodies_and_reemits_load_bearing_portions_without_scanning() {
    let incoming: Text<ToyString> = Text::from("alpha.beta");
    let value = incoming.embody().unwrap();
    assert_eq!(value.0, "alpha.beta");
    assert_eq!(value.textualize().as_ref(), "alpha.beta");
}

#[test]
fn hashes_distinguish_distinct_normalized_content() {
    assert_ne!(text("alpha").content_hash(), text("beta").content_hash());
    assert_eq!(text(" alpha ").content_hash(), text("alpha").content_hash());
}

struct ToyRecord {
    name: String,
    count: String,
}

fn shape_fault(portion: &Portion) -> Fault {
    Fault {
        extent: Extent {
            start: portion.as_ref().start,
            end: portion.as_ref().end,
        },
        problem: FaultProblem::ExpectedShape,
    }
}

impl Embodied for ToyRecord {
    fn from_portion(portion: &Portion) -> Result<Self, Fault> {
        let enclosed = match portion {
            Portion::Enclosed(_, enclosed)
                if enclosed.structural_enclosure() == Some(StructuralEnclosure::Braced) =>
            {
                enclosed
            }
            _ => return Err(shape_fault(portion)),
        };
        let fields = enclosed
            .portions()
            .filter(|fields| fields.len() == 2)
            .ok_or_else(|| shape_fault(portion))?;
        let name = match &fields[0] {
            Portion::Bare(_, bare) => bare.symbol.as_ref().to_owned(),
            _ => return Err(shape_fault(&fields[0])),
        };
        let count = match &fields[1] {
            Portion::Bare(_, bare) => bare.symbol.as_ref().to_owned(),
            _ => return Err(shape_fault(&fields[1])),
        };
        Ok(Self { name, count })
    }
}

impl Textualizable for ToyRecord {
    fn to_portion(&self) -> Portion {
        let fields = vec![
            Portion::from(Bare::from(symbol(&self.name))),
            Portion::from(Bare::from(symbol(&self.count))),
        ];
        Portion::from(Enclosed::from(StructuralEnclosed::from((
            StructuralEnclosure::Braced,
            fields,
        ))))
    }
}

impl ShapeDefined for ToyRecord {
    fn matches(portion: &Portion) -> bool {
        matches!(portion, Portion::Enclosed(_, enclosed)
            if enclosed.structural_enclosure() == Some(StructuralEnclosure::Braced) && enclosed.arity() == 2)
    }
}

#[test]
fn typed_text_is_embodiable_and_dialects_never_rescan_characters() {
    let prospective: Text<ToyRecord> = Text::from(" { north 42 } ");
    let record = prospective.embody().unwrap();
    assert_eq!(record.name, "north");
    assert_eq!(record.count, "42");
    assert_eq!(record.textualize().as_ref(), "{north 42}");
    let portion = text("{north 42}").delineate().unwrap().portions.remove(0);
    assert!(<ToyRecord as ShapeDefined>::matches(&portion));
}
