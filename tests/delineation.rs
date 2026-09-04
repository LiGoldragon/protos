use proptest::prelude::*;
use protos::{
    Boundary, Delineation, Enclosure, Extent, Fault, Head, Potential, Problem, Protoform,
    Protosizable, Separator, Situating, Textualizable,
};

fn delineate(source: &str) -> Result<Delineation, Fault> {
    source.to_owned().protosize()
}

#[derive(Clone, Debug)]
enum Spec {
    Bare(String),
    Headed(String, Separator, Box<Spec>),
    Enclosed(Enclosure, Vec<Spec>),
}

fn to_protoform(spec: &Spec) -> Protoform {
    match spec {
        Spec::Bare(s) => Protoform::Bare(Head::Bare(s.clone())),
        Spec::Headed(h, s, b) => {
            Protoform::Headed(Head::Bare(h.clone()), *s, Box::new(to_protoform(b)))
        }
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
        let printed = pf.textualize();
        let delineated = printed.protosize()
            .expect("the sole writer's output delineates");
        prop_assert_eq!(&delineated.protoforms, std::slice::from_ref(&pf));
        let reprinted = delineated.textualize();
        prop_assert_eq!(&reprinted, &printed);
    }
}

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
            Protoform::Headed(Head::Bare(h), s, b) => {
                assert_eq!(h, "alpha");
                assert_eq!(*s, sep);
                assert_eq!(**b, Protoform::Bare(Head::Bare("beta".to_owned())));
            }
            other => panic!("expected Headed, got {other:?}"),
        }
        assert_eq!(d.textualize(), source);
    }
}

#[test]
fn structural_enclosures_round_trip() {
    let cases = [
        ("{ alpha beta }", Enclosure::Braced),
        ("[ alpha beta ]", Enclosure::Bracketed),
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
        assert_eq!(d.textualize(), source, "print round-trip for {source}");
    }
}

#[test]
fn empty_enclosures_print_tight() {
    assert_eq!(
        Protoform::Enclosed(Enclosure::Braced, vec![]).textualize(),
        "{}"
    );
    assert_eq!(
        Protoform::Enclosed(Enclosure::Bracketed, vec![]).textualize(),
        "[]"
    );
    assert_eq!(
        Protoform::Enclosed(Enclosure::Angled, vec![]).textualize(),
        "<>"
    );
}

#[test]
fn nonempty_braces_brackets_have_inner_space() {
    let pf =
        Protoform::Enclosed(Enclosure::Braced, vec![Protoform::Bare(Head::Bare("a".to_owned()))]);
    assert_eq!(pf.textualize(), "{ a }");

    let pf = Protoform::Enclosed(
        Enclosure::Bracketed,
        vec![Protoform::Bare(Head::Bare("a".to_owned()))],
    );
    assert_eq!(pf.textualize(), "[ a ]");
}

#[test]
fn angled_is_always_tight() {
    let pf = Protoform::Enclosed(
        Enclosure::Angled,
        vec![
            Protoform::Bare(Head::Bare("a".to_owned())),
            Protoform::Bare(Head::Bare("b".to_owned())),
        ],
    );
    assert_eq!(pf.textualize(), "<a b>");
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
    assert_eq!(d.textualize(), source);
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
    assert_eq!(d.textualize(), source);
}

#[test]
fn parentheses_escaped_on_print() {
    let pf = Protoform::Opaque(Boundary::Parentheses, "a)b".to_owned());
    let printed = pf.textualize();
    assert_eq!(printed, "(a\\)b)");
    let d = delineate(&printed).unwrap();
    assert_eq!(d.protoforms.len(), 1);
    assert_eq!(d.protoforms[0], pf);
}

#[test]
fn single_semicolon_opens_comment_to_end_of_line() {
    let source = "alpha ; dropped\nbeta";
    let d = delineate(source).unwrap();
    assert_eq!(d.protoforms.len(), 2);
    assert_eq!(
        d.protoforms[0],
        Protoform::Bare(Head::Bare("alpha".to_owned()))
    );
    assert_eq!(
        d.protoforms[1],
        Protoform::Bare(Head::Bare("beta".to_owned()))
    );
}

#[test]
fn comment_at_end_of_input() {
    let source = "alpha ; comment";
    let d = delineate(source).unwrap();
    assert_eq!(d.protoforms.len(), 1);
    assert_eq!(
        d.protoforms[0],
        Protoform::Bare(Head::Bare("alpha".to_owned()))
    );
}

#[test]
fn headed_chain_parses_and_prints() {
    let source = "a.b.c";
    let d = delineate(source).unwrap();
    assert_eq!(d.protoforms.len(), 1);
    let expected = Protoform::Headed(
        Head::Bare("a".to_owned()),
        Separator::Period,
        Box::new(Protoform::Headed(
            Head::Bare("b".to_owned()),
            Separator::Period,
            Box::new(Protoform::Bare(Head::Bare("c".to_owned()))),
        )),
    );
    assert_eq!(d.protoforms[0], expected);
    assert_eq!(d.textualize(), source);
}

#[test]
fn headed_with_enclosed_body() {
    let source = "Head.{ a b }";
    let d = delineate(source).unwrap();
    assert_eq!(d.protoforms.len(), 1);
    match &d.protoforms[0] {
        Protoform::Headed(Head::Bare(h), s, b) => {
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
    assert_eq!(d.textualize(), source);
}

#[test]
fn headed_chain_with_enclosed_body() {
    let source = "Observed.Locks.[]";
    let d = delineate(source).unwrap();
    assert_eq!(d.protoforms.len(), 1);
    let expected = Protoform::Headed(
        Head::Bare("Observed".to_owned()),
        Separator::Period,
        Box::new(Protoform::Headed(
            Head::Bare("Locks".to_owned()),
            Separator::Period,
            Box::new(Protoform::Enclosed(Enclosure::Bracketed, vec![])),
        )),
    );
    assert_eq!(d.protoforms[0], expected);
    assert_eq!(d.textualize(), source);
}

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
    assert_eq!(err.extent.0, 6);
    assert_eq!(err.extent.1, 7);
}

#[test]
fn fault_missing_body() {
    let source = "alpha.";
    let err = delineate(source).unwrap_err();
    assert_eq!(err.problem, Problem::MissingBody);
    assert_eq!(err.extent, Extent(5, 6));
}

#[test]
fn fault_missing_head() {
    let source = ".alpha";
    let err = delineate(source).unwrap_err();
    assert_eq!(err.problem, Problem::MissingHead);
    assert_eq!(err.extent, Extent(0, 1));
}

#[test]
fn empty_input_gives_empty_delineation() {
    let d = delineate("").unwrap();
    assert!(d.protoforms.is_empty());
}

#[test]
fn whitespace_only_gives_empty_delineation() {
    let d = delineate("   \n\t  ").unwrap();
    assert!(d.protoforms.is_empty());
}

#[test]
fn situation_records_correct_extents() {
    let source = "alpha { beta }";
    let d = delineate(source).unwrap();
    assert_eq!(d.situate(&[0]), Some(Extent(0, 5)));
    assert_eq!(d.situate(&[1]), Some(Extent(6, 14)));
    assert_eq!(d.situate(&[1, 0]), Some(Extent(8, 12)));
}

#[test]
fn situation_for_headed() {
    let source = "Head.body";
    let d = delineate(source).unwrap();
    assert_eq!(d.situate(&[0]), Some(Extent(0, 9)));
    assert_eq!(d.situate(&[0, 0]), Some(Extent(5, 9)));
}

#[test]
fn situation_for_headed_chain() {
    let source = "a.b.c";
    let d = delineate(source).unwrap();
    assert_eq!(d.situate(&[0]), Some(Extent(0, 5)));
    assert_eq!(d.situate(&[0, 0]), Some(Extent(2, 5)));
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
    assert_eq!(err.extent, Extent(6, 9));
}

#[test]
fn potential_delineates_from_text() {
    let pot: Potential<()> = Potential::from("alpha beta");
    let d = pot.protosize().unwrap();
    assert_eq!(d.protoforms.len(), 2);
}

#[test]
fn complex_headed_with_struct_and_vector() {
    let source = "{ Ada 1990 { \u{201C}12 Rue de la Paix\u{201D} Paris 75002 } [ Author Reviewer.{ 2024 17 } ] }";
    let d = delineate(source).unwrap();
    assert_eq!(d.protoforms.len(), 1);
    assert_eq!(d.textualize(), source);
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
        assert_eq!(d.textualize(), source, "round-trip: {source}");
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
        assert_eq!(d.textualize(), source, "round-trip: {source}");
    }
}

#[test]
fn meaning_examples() {
    let source = "{ Ada (The build passed on the third try (after two timeouts)) }";
    let d = delineate(source).unwrap();
    assert_eq!(d.protoforms.len(), 1);
    assert_eq!(d.textualize(), source);
}

#[test]
fn qualified_standalone() {
    let source = "Vector<Text>";
    let d = delineate(source).unwrap();
    assert_eq!(d.protoforms.len(), 1);
    match &d.protoforms[0] {
        Protoform::Bare(Head::Qualified(symbol, children)) => {
            assert_eq!(symbol, "Vector");
            assert_eq!(children.len(), 1);
            assert_eq!(children[0], Protoform::Bare(Head::Bare("Text".to_owned())));
        }
        other => panic!("expected Bare(Qualified), got {other:?}"),
    }
    assert_eq!(d.textualize(), source);
}

#[test]
fn qualified_with_multiple_args() {
    let source = "Result<Integer SinkError>";
    let d = delineate(source).unwrap();
    assert_eq!(d.protoforms.len(), 1);
    match &d.protoforms[0] {
        Protoform::Bare(Head::Qualified(symbol, children)) => {
            assert_eq!(symbol, "Result");
            assert_eq!(children.len(), 2);
        }
        other => panic!("expected Bare(Qualified), got {other:?}"),
    }
    assert_eq!(d.textualize(), source);
}

#[test]
fn qualified_as_head() {
    let source = "Processable<[ Clonable Sendable ] Serializable>.[ cap ]";
    let d = delineate(source).unwrap();
    assert_eq!(d.protoforms.len(), 1);
    match &d.protoforms[0] {
        Protoform::Headed(Head::Qualified(symbol, quals), sep, _body) => {
            assert_eq!(symbol, "Processable");
            assert_eq!(quals.len(), 2);
            assert_eq!(*sep, Separator::Period);
        }
        other => panic!("expected Headed with Qualified head, got {other:?}"),
    }
    assert_eq!(d.textualize(), source);
}

#[test]
fn standalone_angle_stays_enclosed() {
    let source = "<a b>";
    let d = delineate(source).unwrap();
    assert_eq!(d.protoforms.len(), 1);
    match &d.protoforms[0] {
        Protoform::Enclosed(Enclosure::Angled, children) => {
            assert_eq!(children.len(), 2);
        }
        other => panic!("expected Enclosed Angled, got {other:?}"),
    }
}

#[test]
fn separator_before_qualified_body() {
    let source = "LockPaths.Vector<LockPath>";
    let d = delineate(source).unwrap();
    assert_eq!(d.protoforms.len(), 1);
    let expected = Protoform::Headed(
        Head::Bare("LockPaths".to_owned()),
        Separator::Period,
        Box::new(Protoform::Bare(Head::Qualified(
            "Vector".to_owned(),
            vec![Protoform::Bare(Head::Bare("LockPath".to_owned()))],
        ))),
    );
    assert_eq!(d.protoforms[0], expected);
    assert_eq!(d.textualize(), source);
}

#[test]
fn chain_with_qualified_body() {
    let source = "A.B<C>.D";
    let d = delineate(source).unwrap();
    assert_eq!(d.protoforms.len(), 1);
    match &d.protoforms[0] {
        Protoform::Headed(Head::Bare(a), Separator::Period, rest) => {
            assert_eq!(a, "A");
            match rest.as_ref() {
                Protoform::Headed(Head::Qualified(b, quals), Separator::Period, d_body) => {
                    assert_eq!(b, "B");
                    assert_eq!(quals.len(), 1);
                    assert_eq!(
                        **d_body,
                        Protoform::Bare(Head::Bare("D".to_owned()))
                    );
                }
                other => panic!("expected Headed with Qualified head, got {other:?}"),
            }
        }
        other => panic!("expected Headed, got {other:?}"),
    }
    assert_eq!(d.textualize(), source);
}
