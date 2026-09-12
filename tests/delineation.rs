//! Reading: every rule of the reader, every error, every extent.

use protos::{
    Boundary, BoundedProtosizable, Enclosure, Error, Extent, Problem, Protos, Protosizable,
    ReaderBudget, Separator, Symbol,
};

fn read(text: &str) -> Protos {
    text.protosize()
        .unwrap_or_else(|error| panic!("{text:?} is one structure: {error}"))
}
fn failure(text: &str) -> Error {
    text.protosize().expect_err(&format!("{text:?} is refused"))
}
fn problem(text: &str) -> Problem {
    failure(text).problem
}
fn extent(form: &Protos) -> Extent {
    match form {
        Protos::Headed { extent, .. }
        | Protos::Enclosed { extent, .. }
        | Protos::Opaque { extent, .. }
        | Protos::Bare { extent, .. } => *extent,
    }
}
/// The slice of `text` a node claims as its own.
fn claimed<'a>(text: &'a str, form: &Protos) -> &'a str {
    let extent = extent(form);
    &text[extent.start..extent.end]
}
fn children(form: &Protos) -> &[Protos] {
    let Protos::Enclosed { children, .. } = form else {
        panic!("an enclosed structure, not {form:?}")
    };
    children
}
fn body(form: &Protos) -> &Protos {
    let Protos::Headed { body, .. } = form else {
        panic!("a headed structure, not {form:?}")
    };
    body
}
fn bare(text: &str) -> Protos {
    Protos::Bare {
        extent: Extent {
            start: 0,
            end: text.len(),
        },
        text: text.to_owned(),
    }
}

#[test]
fn a_head_a_separator_and_a_body() {
    let text = "Some.42";
    let form = read(text);
    let Protos::Headed {
        head,
        constraints,
        separator,
        ..
    } = &form
    else {
        panic!("headed, not {form:?}")
    };
    assert_eq!(head, &Symbol(String::from("Some")));
    assert_eq!(*separator, Separator::Period);
    assert!(constraints.is_none());
    assert_eq!(claimed(text, &form), text);
    assert_eq!(claimed(text, body(&form)), "42");
}

#[test]
fn a_chain_is_right_associative() {
    let text = "a:b:c";
    let form = read(text);
    assert_eq!(claimed(text, &form), "a:b:c");
    assert_eq!(claimed(text, body(&form)), "b:c");
    assert_eq!(claimed(text, body(body(&form))), "c");
    for (form, expected) in [(&form, Separator::Colon), (body(&form), Separator::Colon)] {
        let Protos::Headed { separator, .. } = form else {
            panic!("headed, not {form:?}")
        };
        assert_eq!(*separator, expected);
    }
    let mixed = read("a!b.c");
    let Protos::Headed { separator, .. } = &mixed else {
        panic!("headed")
    };
    assert_eq!(*separator, Separator::Exclamation);
    let Protos::Headed { separator, .. } = body(&mixed) else {
        panic!("headed")
    };
    assert_eq!(*separator, Separator::Period);
}

#[test]
fn a_separator_opens_an_enclosed_body() {
    let text = "Reviewer.{ 2024 17 }";
    let form = read(text);
    let enclosed = body(&form);
    assert_eq!(claimed(text, enclosed), "{ 2024 17 }");
    let years = children(enclosed);
    assert_eq!(years.len(), 2);
    assert_eq!(claimed(text, &years[0]), "2024");
    assert_eq!(claimed(text, &years[1]), "17");

    let chained = "Observed.Locks.[]";
    let form = read(chained);
    assert_eq!(claimed(chained, body(&form)), "Locks.[]");
    assert_eq!(claimed(chained, body(body(&form))), "[]");
    assert!(children(body(body(&form))).is_empty());
}

#[test]
fn a_run_that_is_not_a_chain_stays_whole() {
    for word in ["-", "-42", "2026-09-03", "a..b", "a.", ".a", "..", "a.b."] {
        assert_eq!(read(word), bare(word), "{word:?}");
    }
}

#[test]
fn a_timestamp_is_a_colon_chain() {
    let text = "2026-09-03T17:46:20";
    let form = read(text);
    let Protos::Headed { head, .. } = &form else {
        panic!("headed, not {form:?}")
    };
    assert_eq!(head, &Symbol(String::from("2026-09-03T17")));
    assert_eq!(claimed(text, body(&form)), "46:20");
}

#[test]
fn adjacency_without_a_separator_is_two_structures() {
    // One text, one structure: a second structure at the top is refused, and
    // the same adjacency inside an enclosure is two children.
    for text in ["a{ 1 }", "a..{ 1 }", "a.{ 1 }.b", "a<b>c", "Vector<Text>"] {
        assert_eq!(problem(text), Problem::Multiple, "{text:?}");
    }
    let text = "[ a{ 1 } ]";
    assert_eq!(children(&read(text)).len(), 2);
    let qualified = "[ Vector<Text> ]";
    let form = read(qualified);
    let parts = children(&form);
    assert_eq!(parts.len(), 2);
    assert_eq!(claimed(qualified, &parts[0]), "Vector");
    assert_eq!(claimed(qualified, &parts[1]), "<Text>");
}

#[test]
fn a_constrained_head_needs_a_separator_and_a_body() {
    let text = "A<B>.{ 1 }";
    let form = read(text);
    let Protos::Headed {
        head,
        constraints: Some(constraints),
        ..
    } = &form
    else {
        panic!("a constrained head, not {form:?}")
    };
    assert_eq!(head, &Symbol(String::from("A")));
    assert_eq!(claimed(text, constraints), "<B>");
    assert_eq!(claimed(text, &children(constraints)[0]), "B");
    assert_eq!(claimed(text, body(&form)), "{ 1 }");
    assert_eq!(problem("A<B>. C"), Problem::MissingBody);
    assert_eq!(problem("A<B>."), Problem::MissingBody);
    assert_eq!(problem("[ A<B>. ]"), Problem::MissingBody);
    // Angles with no head before them are an ordinary enclosure, so `C` is
    // then a second structure rather than a body.
    assert_eq!(problem("<B>.C"), Problem::Multiple);
}

#[test]
fn guillemets_hold_every_glyph_as_content() {
    for (text, content) in [
        ("«a { b»", "a { b"),
        ("«»", ""),
        ("« ; not a comment »", " ; not a comment "),
        ("«a\\»b»", "a»b"),
        ("«a\\\\b»", "a\\b"),
        ("«a\\b»", "a\\b"),
        ("«a(b»", "a(b"),
    ] {
        let form = read(text);
        assert_eq!(
            form,
            Protos::Opaque {
                extent: Extent {
                    start: 0,
                    end: text.len()
                },
                boundary: Boundary::Guillemets,
                content: String::from(content),
            },
            "{text:?}"
        );
    }
}

#[test]
fn parentheses_are_read_by_balance() {
    for (text, content) in [
        ("(a (b) c)", "a (b) c"),
        ("(a \\) b)", "a ) b"),
        ("(a \\( b)", "a ( b"),
        ("(\\\\)", "\\"),
        ("(a\\x)", "a\\x"),
        ("()", ""),
        ("(a ; b)", "a ; b"),
        ("(a « b)", "a « b"),
        ("(a » b)", "a » b"),
    ] {
        let form = read(text);
        assert_eq!(
            form,
            Protos::Opaque {
                extent: Extent {
                    start: 0,
                    end: text.len()
                },
                boundary: Boundary::Parentheses,
                content: String::from(content),
            },
            "{text:?}"
        );
    }
}

#[test]
fn a_comment_runs_to_the_end_of_its_line() {
    assert_eq!(read("a ; comment\n"), bare("a"));
    assert_eq!(read("a;b"), bare("a"));
    let text = "{ 1 ; c }\n 2 }";
    let form = read(text);
    let parts = children(&form);
    assert_eq!(parts.len(), 2);
    assert_eq!(claimed(text, &parts[0]), "1");
    assert_eq!(claimed(text, &parts[1]), "2");
}

#[test]
fn whitespace_separates_and_emptiness_is_refused() {
    assert_eq!(problem(""), Problem::Empty);
    assert_eq!(problem("  \n\t "), Problem::Empty);
    assert_eq!(problem(" ; only a comment\n"), Problem::Empty);
    assert_eq!(children(&read("{\n\t1\r\n}")).len(), 1);
    assert!(children(&read("{}")).is_empty());
    assert!(children(&read("[  ]")).is_empty());
    let Protos::Enclosed { enclosure, .. } = read("[  ]") else {
        panic!("enclosed")
    };
    assert_eq!(enclosure, Enclosure::Bracketed);
}

#[test]
fn every_node_of_a_nest_claims_its_own_slice() {
    let text = "{ Ada 1990 { «12 Rue de la Paix» Paris 75002 } [ Author Reviewer.{ 2024 17 } ] }";
    let form = read(text);
    assert_eq!(claimed(text, &form), text);
    let top = children(&form);
    assert_eq!(claimed(text, &top[0]), "Ada");
    assert_eq!(claimed(text, &top[1]), "1990");
    assert_eq!(
        claimed(text, &top[2]),
        "{ «12 Rue de la Paix» Paris 75002 }"
    );
    assert_eq!(claimed(text, &children(&top[2])[0]), "«12 Rue de la Paix»");
    assert_eq!(claimed(text, &top[3]), "[ Author Reviewer.{ 2024 17 } ]");
    let reviewer = &children(&top[3])[1];
    assert_eq!(claimed(text, reviewer), "Reviewer.{ 2024 17 }");
    let year = &children(body(reviewer))[1];
    assert_eq!(claimed(text, year), "17");
    assert_eq!(
        extent(year).start,
        text.find("17 }").expect("the inner year")
    );
}

#[test]
fn every_refusal_names_its_problem_and_where_it_arose() {
    for (text, expected) in [
        ("{ 1 ", Problem::Unclosed('{')),
        ("Some.{ 1 ", Problem::Unclosed('{')),
        ("A<B", Problem::Unclosed('<')),
        ("«abc", Problem::Unclosed('«')),
        ("(a", Problem::Unclosed('(')),
        ("(a\\", Problem::Unclosed('(')),
        ("((a)", Problem::Unclosed('(')),
        ("}", Problem::Unexpected('}')),
        (")", Problem::Unexpected(')')),
        ("»", Problem::Unexpected('»')),
        ("[ { 1 ] }", Problem::Unexpected(']')),
        ("a { b } c", Problem::Multiple),
    ] {
        assert_eq!(problem(text), expected, "{text:?}");
    }
    let error = failure("{ 1 ");
    assert_eq!(
        error.extent,
        Extent {
            start: 0,
            end: "{ 1 ".len()
        }
    );
    assert_eq!(
        format!("{error}"),
        "structural error at 0..4: Unclosed('{')"
    );
}

#[test]
fn descent_is_bounded_by_depth_before_it_is_bounded_by_the_stack() {
    let text = format!("{}1{}", "[".repeat(300), "]".repeat(300));
    let mut budget = ReaderBudget {
        remaining: usize::MAX,
    };
    assert_eq!(
        text.protosize_with(&mut budget)
            .expect_err("too deep")
            .problem,
        Problem::Depth
    );
}

#[test]
fn a_string_protosizes_like_a_str() {
    let owned = String::from("{ 1 }");
    assert_eq!(owned.protosize().expect("structure"), read("{ 1 }"));
}
