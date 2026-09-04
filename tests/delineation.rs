use proptest::prelude::*;
use protos::{
    Boundary, Delineation, Enclosure, Extent, Fault, Potential, Printing, Problem, Protoform,
    Separator, Situating, Structural,
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn delineate(source: &str) -> Result<Delineation, Fault> {
    source.to_owned().delineate()
}

// ---------------------------------------------------------------------------
// Proptest: print then delineate round-trips for Protoform
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
enum Spec {
    Bare(String),
    Headed(String, Separator, Box<Spec>),
    Enclosed(Enclosure, Vec<Spec>),
}

fn to_protoform(spec: &Spec) -> Protoform {
    match spec {
        Spec::Bare(s) => Protoform::Bare(s.clone()),
        Spec::Headed(h, s, b) => Protoform::Headed(h.clone(), *s, Box::new(to_protoform(b))),
        Spec::Enclosed(e, children) => {
            Protoform::Enclosed(*e, children.iter().map(to_protoform).collect())
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
                .prop_map(|(h, s, b)| Spec::Headed(h, s, Box::new(b))),
            (
                prop_oneof![
                    Just(Enclosure::Braced),
                    Just(Enclosure::Bracketed),
                    Just(Enclosure::Guillemets),
                    Just(Enclosure::Angled),
                ],
                prop::collection::vec(inner, 0..4),
            )
                .prop_map(|(e, c)| Spec::Enclosed(e, c)),
        ]
    })
}

proptest! {
    #[test]
    fn protoform_print_then_delineate_round_trips(spec in spec_strategy()) {
        let pf = to_protoform(&spec);
        let printed = pf.print();
        let delineated = printed.delineate()
            .expect("the sole writer's output delineates");
        prop_assert_eq!(&delineated.protoforms, std::slice::from_ref(&pf));
        let reprinted = delineated.print();
        prop_assert_eq!(&reprinted, &printed);
    }
}

// ---------------------------------------------------------------------------
// Delimiter round-trip tests
// ---------------------------------------------------------------------------

#[test]
fn all_separators_round_trip() {
    for (source, sep) in [
        ("alpha.beta", Separator::Period),
        ("alpha!beta", Separator::Exclamation),
        ("alpha:beta", Separator::Colon),
    ] {
        let d = delineate(source).unwrap();
        assert_eq!(d.protoforms.len(), 1);
        match &d.protoforms[0] {
            Protoform::Headed(h, s, b) => {
                assert_eq!(h, "alpha");
                assert_eq!(*s, sep);
                assert_eq!(**b, Protoform::Bare("beta".to_owned()));
            }
            other => panic!("expected Headed, got {other:?}"),
        }
        assert_eq!(d.print(), source);
    }
}

#[test]
fn structural_enclosures_round_trip() {
    let cases = [
        ("{ alpha beta }", Enclosure::Braced),
        ("[ alpha beta ]", Enclosure::Bracketed),
        ("\u{00AB} alpha beta \u{00BB}", Enclosure::Guillemets),
        ("<alpha beta>", Enclosure::Angled),
    ];
    for (source, enclosure) in cases {
        let d = delineate(source).unwrap();
        assert_eq!(d.protoforms.len(), 1, "source: {source}");
        match &d.protoforms[0] {
            Protoform::Enclosed(e, children) => {
                assert_eq!(*e, enclosure, "source: {source}");
                assert_eq!(children.len(), 2, "source: {source}");
            }
            other => panic!("expected Enclosed for {source}, got {other:?}"),
        }
        assert_eq!(d.print(), source, "print round-trip for {source}");
    }
}

#[test]
fn empty_enclosures_print_tight() {
    assert_eq!(Protoform::Enclosed(Enclosure::Braced, vec![]).print(), "{}");
    assert_eq!(
        Protoform::Enclosed(Enclosure::Bracketed, vec![]).print(),
        "[]"
    );
    assert_eq!(
        Protoform::Enclosed(Enclosure::Guillemets, vec![]).print(),
        "\u{00AB}\u{00BB}"
    );
    assert_eq!(Protoform::Enclosed(Enclosure::Angled, vec![]).print(), "<>");
}

#[test]
fn nonempty_braces_brackets_guillemets_have_inner_space() {
    let pf = Protoform::Enclosed(Enclosure::Braced, vec![Protoform::Bare("a".to_owned())]);
    assert_eq!(pf.print(), "{ a }");

    let pf = Protoform::Enclosed(Enclosure::Bracketed, vec![Protoform::Bare("a".to_owned())]);
    assert_eq!(pf.print(), "[ a ]");

    let pf = Protoform::Enclosed(Enclosure::Guillemets, vec![Protoform::Bare("a".to_owned())]);
    assert_eq!(pf.print(), "\u{00AB} a \u{00BB}");
}

#[test]
fn angled_is_always_tight() {
    let pf = Protoform::Enclosed(
        Enclosure::Angled,
        vec![
            Protoform::Bare("a".to_owned()),
            Protoform::Bare("b".to_owned()),
        ],
    );
    assert_eq!(pf.print(), "<a b>");
}

#[test]
fn curly_quotes_round_trip() {
    let source = "\u{201C}hello world\u{201D}";
    let d = delineate(source).unwrap();
    assert_eq!(d.protoforms.len(), 1);
    match &d.protoforms[0] {
        Protoform::Opaque(Boundary::CurlyQuotes, content) => {
            assert_eq!(content, "hello world");
        }
        other => panic!("expected CurlyQuotes opaque, got {other:?}"),
    }
    assert_eq!(d.print(), source);
}

#[test]
fn parentheses_read_by_balance() {
    let source = "(alpha(beta)gamma)";
    let d = delineate(source).unwrap();
    assert_eq!(d.protoforms.len(), 1);
    match &d.protoforms[0] {
        Protoform::Opaque(Boundary::Parentheses, content) => {
            assert_eq!(content, "alpha(beta)gamma");
        }
        other => panic!("expected Parentheses opaque, got {other:?}"),
    }
    assert_eq!(d.print(), source);
}

#[test]
fn parentheses_escaped_on_print() {
    let pf = Protoform::Opaque(Boundary::Parentheses, "a)b".to_owned());
    let printed = pf.print();
    assert_eq!(printed, "(a\\)b)");
    let d = delineate(&printed).unwrap();
    assert_eq!(d.protoforms.len(), 1);
    assert_eq!(d.protoforms[0], pf);
}

// ---------------------------------------------------------------------------
// Comment tests
// ---------------------------------------------------------------------------

#[test]
fn single_semicolon_opens_comment_to_end_of_line() {
    let source = "alpha ; dropped\nbeta";
    let d = delineate(source).unwrap();
    assert_eq!(d.protoforms.len(), 2);
    assert_eq!(d.protoforms[0], Protoform::Bare("alpha".to_owned()));
    assert_eq!(d.protoforms[1], Protoform::Bare("beta".to_owned()));
}

#[test]
fn comment_at_end_of_input() {
    let source = "alpha ; comment";
    let d = delineate(source).unwrap();
    assert_eq!(d.protoforms.len(), 1);
    assert_eq!(d.protoforms[0], Protoform::Bare("alpha".to_owned()));
}

// ---------------------------------------------------------------------------
// Headed chain tests
// ---------------------------------------------------------------------------

#[test]
fn headed_chain_parses_and_prints() {
    let source = "a.b.c";
    let d = delineate(source).unwrap();
    assert_eq!(d.protoforms.len(), 1);
    let expected = Protoform::Headed(
        "a".to_owned(),
        Separator::Period,
        Box::new(Protoform::Headed(
            "b".to_owned(),
            Separator::Period,
            Box::new(Protoform::Bare("c".to_owned())),
        )),
    );
    assert_eq!(d.protoforms[0], expected);
    assert_eq!(d.print(), source);
}

#[test]
fn headed_with_enclosed_body() {
    let source = "Head.{ a b }";
    let d = delineate(source).unwrap();
    assert_eq!(d.protoforms.len(), 1);
    match &d.protoforms[0] {
        Protoform::Headed(h, s, b) => {
            assert_eq!(h, "Head");
            assert_eq!(*s, Separator::Period);
            match b.as_ref() {
                Protoform::Enclosed(Enclosure::Braced, children) => {
                    assert_eq!(children.len(), 2);
                }
                other => panic!("expected Enclosed body, got {other:?}"),
            }
        }
        other => panic!("expected Headed, got {other:?}"),
    }
    assert_eq!(d.print(), source);
}

#[test]
fn headed_chain_with_enclosed_body() {
    let source = "Observed.Locks.[]";
    let d = delineate(source).unwrap();
    assert_eq!(d.protoforms.len(), 1);
    let expected = Protoform::Headed(
        "Observed".to_owned(),
        Separator::Period,
        Box::new(Protoform::Headed(
            "Locks".to_owned(),
            Separator::Period,
            Box::new(Protoform::Enclosed(Enclosure::Bracketed, vec![])),
        )),
    );
    assert_eq!(d.protoforms[0], expected);
    assert_eq!(d.print(), source);
}

// ---------------------------------------------------------------------------
// Fault tests
// ---------------------------------------------------------------------------

#[test]
fn fault_unclosed_brace() {
    let err = delineate("{ alpha").unwrap_err();
    assert_eq!(err.problem, Problem::Unclosed(Enclosure::Braced));
    assert_eq!(err.extent.0, 0);
}

#[test]
fn fault_unclosed_curly_quote() {
    let source = "\u{201C}alpha";
    let err = delineate(source).unwrap_err();
    assert_eq!(
        err.problem,
        Problem::UnclosedBoundary(Boundary::CurlyQuotes)
    );
    assert_eq!(err.extent.0, 0);
}

#[test]
fn fault_unclosed_parenthesis() {
    let source = "(alpha";
    let err = delineate(source).unwrap_err();
    assert_eq!(
        err.problem,
        Problem::UnclosedBoundary(Boundary::Parentheses)
    );
    assert_eq!(err.extent.0, 0);
}

#[test]
fn fault_unopened() {
    let source = "alpha }";
    let err = delineate(source).unwrap_err();
    assert_eq!(err.problem, Problem::Unopened);
    // The `}` is at byte 6
    assert_eq!(err.extent.0, 6);
    assert_eq!(err.extent.1, 7);
}

#[test]
fn fault_missing_body() {
    let source = "alpha.";
    let err = delineate(source).unwrap_err();
    assert_eq!(err.problem, Problem::MissingBody);
    // The `.` is at byte 5
    assert_eq!(err.extent, Extent(5, 6));
}

#[test]
fn fault_missing_head() {
    let source = ".alpha";
    let err = delineate(source).unwrap_err();
    assert_eq!(err.problem, Problem::MissingHead);
    // The `.` is at byte 0
    assert_eq!(err.extent, Extent(0, 1));
}

// ---------------------------------------------------------------------------
// Situation tests
// ---------------------------------------------------------------------------

#[test]
fn situation_records_correct_extents() {
    let source = "alpha { beta }";
    let d = delineate(source).unwrap();
    // "alpha" at [0]: bytes 0..5
    assert_eq!(d.situate(&[0]), Some(Extent(0, 5)));
    // "{ beta }" at [1]: bytes 6..14
    assert_eq!(d.situate(&[1]), Some(Extent(6, 14)));
    // "beta" at [1, 0]: bytes 8..12
    assert_eq!(d.situate(&[1, 0]), Some(Extent(8, 12)));
}

#[test]
fn situation_for_headed() {
    // "Head.body" = H(0) e(1) a(2) d(3) .(4) b(5) o(6) d(7) y(8) = 9 bytes
    let source = "Head.body";
    let d = delineate(source).unwrap();
    // The whole headed structure at [0]: bytes 0..9
    assert_eq!(d.situate(&[0]), Some(Extent(0, 9)));
    // The body "body" at [0, 0]: bytes 5..9
    assert_eq!(d.situate(&[0, 0]), Some(Extent(5, 9)));
}

#[test]
fn situation_for_headed_chain() {
    // "a.b.c" = a(0) .(1) b(2) .(3) c(4) = 5 bytes
    let source = "a.b.c";
    let d = delineate(source).unwrap();
    // The whole chain at [0]: bytes 0..5
    assert_eq!(d.situate(&[0]), Some(Extent(0, 5)));
    // "b.c" at [0, 0]: bytes 2..5
    assert_eq!(d.situate(&[0, 0]), Some(Extent(2, 5)));
    // "c" at [0, 0, 0]: bytes 4..5
    assert_eq!(d.situate(&[0, 0, 0]), Some(Extent(4, 5)));
}

#[test]
fn fault_unopened_close_paren() {
    let err = delineate("alpha )").unwrap_err();
    assert_eq!(err.problem, Problem::Unopened);
    assert_eq!(err.extent, Extent(6, 7));
}

#[test]
fn fault_unopened_close_curly_quote() {
    let err = delineate("alpha \u{201D}").unwrap_err();
    assert_eq!(err.problem, Problem::Unopened);
    // \u{201D} is 3 bytes; starts at byte 6
    assert_eq!(err.extent, Extent(6, 9));
}

// ---------------------------------------------------------------------------
// Potential tests
// ---------------------------------------------------------------------------

#[test]
fn potential_delineates_from_text() {
    let pot: Potential<()> = Potential::from("alpha beta");
    let d = pot.delineate().unwrap();
    assert_eq!(d.protoforms.len(), 2);
}

// ---------------------------------------------------------------------------
// Complex round-trip tests (matching spec examples)
// ---------------------------------------------------------------------------

#[test]
fn complex_headed_with_struct_and_vector() {
    // Person-like structure from Vision/datom.md
    let source = "{ Ada 1990 { \u{201C}12 Rue de la Paix\u{201D} Paris 75002 } [ Author Reviewer.{ 2024 17 } ] }";
    let d = delineate(source).unwrap();
    assert_eq!(d.protoforms.len(), 1);
    assert_eq!(d.print(), source);
}

#[test]
fn map_with_bare_string_keys() {
    let source = "\u{00AB} name:first Ada born 1990 \u{00BB}";
    let d = delineate(source).unwrap();
    assert_eq!(d.protoforms.len(), 1);
    assert_eq!(d.print(), source);
}

#[test]
fn reply_variants() {
    for source in [
        "Accepted.{ 42 2026-09-03T17:46:20 }",
        "Refused.{ \u{201C}no such file: { } is content\u{201D} 2 }",
        "Pending",
    ] {
        let d = delineate(source).unwrap();
        assert_eq!(d.protoforms.len(), 1, "source: {source}");
        assert_eq!(d.print(), source, "round-trip: {source}");
    }
}

#[test]
fn orchestrate_lock_examples() {
    for source in [
        "Observed.Locks.[]",
        "Observed.Locks.[ { 440 WisprAuthWitness run_wispr_live_witness [ /home/li/wt/x ] \u{201C}create isolated workspace for one authorized witness\u{201D} } ]",
        "Locked.{ 442 MyLock 6329f1 [ /abs/path ] \u{201C}why I hold it\u{201D} }",
        "ReleaseRejected.UnknownLockId",
    ] {
        let d = delineate(source).unwrap();
        assert_eq!(d.protoforms.len(), 1, "source: {source}");
        assert_eq!(d.print(), source, "round-trip: {source}");
    }
}

#[test]
fn meaning_examples() {
    let source = "{ Ada (The build passed on the third try (after two timeouts)) }";
    let d = delineate(source).unwrap();
    assert_eq!(d.protoforms.len(), 1);
    assert_eq!(d.print(), source);
}
