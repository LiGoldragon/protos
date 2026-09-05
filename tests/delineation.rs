//! Reading: every rule of the reader, every fault, every extent.

use protos::{
    Boundary, Delineation, Enclosure, Extent, Fault, Head, Locating, Opaque, Problem, Protoform,
    Protosizable, Separator, Situated, Situation,
};

fn sym(s: &str) -> Head {
    Head::Symbol(s.to_owned())
}
fn bare(s: &str) -> Protoform {
    Protoform::Bare(sym(s))
}
fn headed(h: &str, sep: Separator, body: Protoform) -> Protoform {
    Protoform::Headed(sym(h), sep, Box::new(body))
}
fn dot(h: &str, body: Protoform) -> Protoform {
    headed(h, Separator::Period, body)
}
fn braced(children: Vec<Protoform>) -> Protoform {
    Protoform::Enclosed(Enclosure::Braced, children)
}
fn bracketed(children: Vec<Protoform>) -> Protoform {
    Protoform::Enclosed(Enclosure::Bracketed, children)
}
fn quoted(s: &str) -> Protoform {
    Protoform::Opaque(Boundary::CurlyQuotes, Opaque::from(s))
}
fn parens(s: &str) -> Protoform {
    Protoform::Opaque(Boundary::Parentheses, Opaque::from(s))
}
fn qualified(s: &str, constraints: Vec<Protoform>) -> Head {
    Head::Qualified(s.to_owned(), constraints)
}

fn read(text: &str) -> Delineation {
    text.protosize().unwrap()
}
fn forms(text: &str) -> Vec<Protoform> {
    read(text)
        .0
        .into_iter()
        .map(|Situated(_, form)| form)
        .collect()
}
fn one(text: &str) -> Situated<Protoform> {
    let mut all = read(text).0;
    assert_eq!(all.len(), 1, "{text:?} is one structure");
    all.pop().unwrap()
}
fn situation(text: &str) -> Situation {
    one(text).0
}
fn fault(text: &str) -> Fault {
    text.protosize().unwrap_err()
}

#[test]
fn head_separator_body() {
    assert_eq!(forms("Some.42"), vec![dot("Some", bare("42"))]);
    let s = situation("Some.42");
    assert_eq!(s.locate(&[]), Some(Extent(0, 7)));
    assert_eq!(s.locate(&[0]), Some(Extent(0, 4)));
    assert_eq!(s.locate(&[1]), Some(Extent(5, 7)));
}

#[test]
fn chain_is_right_associative() {
    assert_eq!(
        forms("a:b:c"),
        vec![headed(
            "a",
            Separator::Colon,
            headed("b", Separator::Colon, bare("c"))
        )]
    );
    let s = situation("a:b:c");
    assert_eq!(s.locate(&[]), Some(Extent(0, 5)));
    assert_eq!(s.locate(&[0]), Some(Extent(0, 1)));
    assert_eq!(s.locate(&[1]), Some(Extent(2, 5)));
    assert_eq!(s.locate(&[1, 0]), Some(Extent(2, 3)));
    assert_eq!(s.locate(&[1, 1]), Some(Extent(4, 5)));
    assert_eq!(s.locate(&[2]), None);
    assert_eq!(
        forms("a!b.c"),
        vec![headed("a", Separator::Exclamation, dot("b", bare("c")))]
    );
}

#[test]
fn head_with_enclosed_body() {
    let text = "Reviewer.{ 2024 17 }";
    assert_eq!(
        forms(text),
        vec![dot("Reviewer", braced(vec![bare("2024"), bare("17")]))]
    );
    let s = situation(text);
    assert_eq!(s.locate(&[]), Some(Extent(0, 20)));
    assert_eq!(s.locate(&[0]), Some(Extent(0, 8)));
    assert_eq!(s.locate(&[1]), Some(Extent(9, 20)));
    assert_eq!(s.locate(&[1, 0]), Some(Extent(11, 15)));
    assert_eq!(s.locate(&[1, 1]), Some(Extent(16, 18)));
}

#[test]
fn chain_with_enclosed_body() {
    let text = "Observed.Locks.[]";
    assert_eq!(
        forms(text),
        vec![dot("Observed", dot("Locks", bracketed(vec![])))]
    );
    let s = situation(text);
    assert_eq!(s.locate(&[]), Some(Extent(0, 17)));
    assert_eq!(s.locate(&[0]), Some(Extent(0, 8)));
    assert_eq!(s.locate(&[1]), Some(Extent(9, 17)));
    assert_eq!(s.locate(&[1, 0]), Some(Extent(9, 14)));
    assert_eq!(s.locate(&[1, 1]), Some(Extent(15, 17)));
}

#[test]
fn head_with_opaque_body() {
    assert_eq!(forms("Some.(x)"), vec![dot("Some", parens("x"))]);
    assert_eq!(situation("Some.(x)").locate(&[1]), Some(Extent(5, 8)));
    assert_eq!(forms("Some.“x y”"), vec![dot("Some", quoted("x y"))]);
}

#[test]
fn runs_that_are_not_chains_stay_whole() {
    for word in ["a.", ".a", "a..b", "-", "-42", "a.b.", "..", "2026-09-03"] {
        assert_eq!(forms(word), vec![bare(word)], "{word:?}");
    }
}

#[test]
fn a_timestamp_with_colons_is_a_colon_chain() {
    assert_eq!(
        forms("2026-09-03T17:46:20"),
        vec![headed(
            "2026-09-03T17",
            Separator::Colon,
            headed("46", Separator::Colon, bare("20"))
        )]
    );
}

#[test]
fn adjacency_without_one_trailing_separator_yields_siblings() {
    assert_eq!(
        forms("a..{ 1 }"),
        vec![bare("a.."), braced(vec![bare("1")])]
    );
    let d = read("a..{ 1 }");
    assert_eq!(d.0[0].0.locate(&[]), Some(Extent(0, 3)));
    assert_eq!(d.0[1].0.locate(&[]), Some(Extent(3, 8)));
    assert_eq!(
        forms("a.{ 1 }.b"),
        vec![dot("a", braced(vec![bare("1")])), bare(".b")]
    );
    assert_eq!(forms("a{ 1 }"), vec![bare("a"), braced(vec![bare("1")])]);
    assert_eq!(
        forms("a<b>c"),
        vec![Protoform::Bare(qualified("a", vec![bare("b")])), bare("c")]
    );
}

#[test]
fn qualified_head_alone() {
    let text = "Vector<Text>";
    assert_eq!(
        forms(text),
        vec![Protoform::Bare(qualified("Vector", vec![bare("Text")]))]
    );
    let s = situation(text);
    assert_eq!(s.locate(&[]), Some(Extent(0, 12)));
    assert_eq!(s.locate(&[0]), Some(Extent(7, 11)));
    assert_eq!(
        forms("Processable<[Clonable Sendable] Serializable>"),
        vec![Protoform::Bare(qualified(
            "Processable",
            vec![
                bracketed(vec![bare("Clonable"), bare("Sendable")]),
                bare("Serializable")
            ]
        ))]
    );
}

#[test]
fn qualified_head_with_body() {
    let text = "A<B>.{ 1 }";
    assert_eq!(
        forms(text),
        vec![Protoform::Headed(
            qualified("A", vec![bare("B")]),
            Separator::Period,
            Box::new(braced(vec![bare("1")]))
        )]
    );
    let s = situation(text);
    assert_eq!(s.locate(&[]), Some(Extent(0, 10)));
    assert_eq!(s.locate(&[0]), Some(Extent(0, 4)));
    assert_eq!(s.locate(&[0, 0]), Some(Extent(2, 3)));
    assert_eq!(s.locate(&[1]), Some(Extent(5, 10)));
    assert_eq!(s.locate(&[1, 0]), Some(Extent(7, 8)));
    assert_eq!(
        forms("A<B>.C"),
        vec![Protoform::Headed(
            qualified("A", vec![bare("B")]),
            Separator::Period,
            Box::new(bare("C"))
        )]
    );
    assert_eq!(
        forms("A.B<C>"),
        vec![dot("A", Protoform::Bare(qualified("B", vec![bare("C")])))]
    );
    assert_eq!(
        forms("A<B>. C"),
        vec![
            Protoform::Bare(qualified("A", vec![bare("B")])),
            bare("."),
            bare("C")
        ]
    );
}

#[test]
fn curly_quotes_are_opaque() {
    let text = "“a { b” c";
    assert_eq!(forms(text), vec![quoted("a { b"), bare("c")]);
    let d = read(text);
    assert_eq!(d.0[0].0.locate(&[]), Some(Extent(0, 11)));
    assert_eq!(d.0[1].0.locate(&[]), Some(Extent(12, 13)));
    assert_eq!(forms("“”"), vec![quoted("")]);
    assert_eq!(
        forms("“ ; not a comment ”"),
        vec![quoted(" ; not a comment ")]
    );
}

#[test]
fn parentheses_are_read_by_balance() {
    assert_eq!(forms("(a (b) c)"), vec![parens("a (b) c")]);
    assert_eq!(forms("(a \\) b)"), vec![parens("a ) b")]);
    assert_eq!(forms("(a \\( b)"), vec![parens("a ( b")]);
    assert_eq!(forms("(\\\\)"), vec![parens("\\")]);
    assert_eq!(forms("(a\\x)"), vec![parens("a\\x")]);
    assert_eq!(forms("()"), vec![parens("")]);
    assert_eq!(forms("(a ; b)"), vec![parens("a ; b")]);
    assert_eq!(forms("(a “ b)"), vec![parens("a “ b")]);
    assert_eq!(forms("(a ” b)"), vec![parens("a ” b")]);
}

#[test]
fn comments_run_to_end_of_line() {
    assert_eq!(forms("a ; comment\n b"), vec![bare("a"), bare("b")]);
    assert_eq!(
        forms("{ 1 ; c }\n 2 }"),
        vec![braced(vec![bare("1"), bare("2")])]
    );
    assert_eq!(forms("a;b"), vec![bare("a")]);
}

#[test]
fn whitespace_and_emptiness() {
    assert_eq!(forms(""), vec![]);
    assert_eq!(forms("  \n\t "), vec![]);
    assert_eq!(forms("{\n\t1\r\n}"), vec![braced(vec![bare("1")])]);
    assert_eq!(forms("{}"), vec![braced(vec![])]);
    assert_eq!(forms("[  ]"), vec![bracketed(vec![])]);
}

#[test]
fn nested_enclosures_situated() {
    let text = "{ Ada 1990 { “12 Rue de la Paix” Paris 75002 } [ Author Reviewer.{ 2024 17 } ] }";
    let Situated(s, form) = one(text);
    assert_eq!(
        form,
        braced(vec![
            bare("Ada"),
            bare("1990"),
            braced(vec![
                quoted("12 Rue de la Paix"),
                bare("Paris"),
                bare("75002")
            ]),
            bracketed(vec![
                bare("Author"),
                dot("Reviewer", braced(vec![bare("2024"), bare("17")]))
            ]),
        ])
    );
    assert_eq!(s.locate(&[]), Some(Extent(0, text.len() as i64)));
    let at = text.find("17").unwrap() as i64;
    assert_eq!(s.locate(&[3, 1, 1, 1]), Some(Extent(at, at + 2)));
    assert_eq!(s.locate(&[3, 1, 0]), Some(Extent(60, 68)));
    assert_eq!(s.locate(&[2, 0]), Some(Extent(13, 36)));
}

#[test]
fn faults_are_situated() {
    let f = fault("{ 1 ");
    assert_eq!(
        f,
        Fault {
            extent: Extent(0, 4),
            problem: Problem::Unclosed(Enclosure::Braced)
        }
    );
    assert_eq!(
        fault("[ { 1 ] }"),
        Fault {
            extent: Extent(6, 7),
            problem: Problem::Unopened(Enclosure::Bracketed)
        }
    );
    assert_eq!(
        fault("}"),
        Fault {
            extent: Extent(0, 1),
            problem: Problem::Unopened(Enclosure::Braced)
        }
    );
    assert_eq!(
        fault("{ [ 1 }"),
        Fault {
            extent: Extent(6, 7),
            problem: Problem::Unopened(Enclosure::Braced)
        }
    );
    assert_eq!(
        fault("“abc"),
        Fault {
            extent: Extent(0, 6),
            problem: Problem::Unterminated(Boundary::CurlyQuotes)
        }
    );
    assert_eq!(
        fault("(a"),
        Fault {
            extent: Extent(0, 2),
            problem: Problem::Unterminated(Boundary::Parentheses)
        }
    );
    assert_eq!(
        fault("(a\\"),
        Fault {
            extent: Extent(0, 3),
            problem: Problem::Unterminated(Boundary::Parentheses)
        }
    );
    assert_eq!(
        fault("((a)"),
        Fault {
            extent: Extent(0, 4),
            problem: Problem::Unterminated(Boundary::Parentheses)
        }
    );
    assert_eq!(
        fault("a ” b"),
        Fault {
            extent: Extent(2, 5),
            problem: Problem::Stray(Boundary::CurlyQuotes)
        }
    );
    assert_eq!(
        fault(")"),
        Fault {
            extent: Extent(0, 1),
            problem: Problem::Stray(Boundary::Parentheses)
        }
    );
    assert_eq!(
        fault("Some.{ 1 "),
        Fault {
            extent: Extent(5, 9),
            problem: Problem::Unclosed(Enclosure::Braced)
        }
    );
    assert_eq!(
        fault("A<B"),
        Fault {
            extent: Extent(1, 3),
            problem: Problem::Unclosed(Enclosure::Angled)
        }
    );
}

#[test]
fn several_top_level_structures() {
    let d = read("a { b } “c”");
    assert_eq!(d.0.len(), 3);
    assert_eq!(d.0[1].0.locate(&[0]), Some(Extent(4, 5)));
    assert_eq!(d.0[2].0.locate(&[]), Some(Extent(8, 15)));
}

#[test]
fn a_string_protosizes_like_a_str() {
    let owned = String::from("{ 1 }");
    assert_eq!(owned.protosize().unwrap(), "{ 1 }".protosize().unwrap());
}
