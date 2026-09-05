//! Writing: canonical spacing, escapes, and extents computed as the writer writes.

use proptest::prelude::*;
use protos::{
    Bare, Boundary, Delineation, Enclosure, Extent, Head, Locating, Opaque, Protoform,
    Protosizable, Refusal, Separator, Situated, Situating, Symbol, Text, Textualizable, Word,
};

fn sym(s: &str) -> Head {
    Head::Symbol(Symbol::try_from(s).unwrap())
}
fn bare(s: &str) -> Protoform {
    Protoform::Bare(Bare::try_from(s).unwrap())
}
fn dot(h: &str, body: Protoform) -> Protoform {
    Protoform::Headed(sym(h), Separator::Period, Box::new(body))
}
fn enclosed(e: Enclosure, children: Vec<Protoform>) -> Protoform {
    Protoform::Enclosed(e, children)
}
fn opaque(b: Boundary, s: &str) -> Protoform {
    match b {
        Boundary::CurlyQuotes => Protoform::Quoted(Text::try_from(s).unwrap()),
        Boundary::Parentheses => Protoform::Parenthesized(Opaque::from(s)),
    }
}

/// The writer's situation must be the reader's situation of the written text.
fn agrees(form: Protoform) -> String {
    let Situated(written, text) = form.situate();
    assert_eq!(form.textualize(), text);
    let mut read = text.protosize().unwrap().0;
    assert_eq!(read.len(), 1);
    let Situated(found, form_again) = read.pop().unwrap();
    assert_eq!(form_again, form);
    assert_eq!(found, written, "situation of {text:?}");
    text
}

#[test]
fn canonical_spacing() {
    assert_eq!(
        agrees(enclosed(Enclosure::Braced, vec![bare("a"), bare("b")])),
        "{ a b }"
    );
    assert_eq!(agrees(enclosed(Enclosure::Braced, vec![])), "{}");
    assert_eq!(
        agrees(enclosed(
            Enclosure::Bracketed,
            vec![bare("0"), bare("42"), bare("-42")]
        )),
        "[ 0 42 -42 ]"
    );
    assert_eq!(agrees(enclosed(Enclosure::Bracketed, vec![])), "[]");
    assert_eq!(
        agrees(enclosed(Enclosure::Angled, vec![bare("a"), bare("b")])),
        "<a b>"
    );
    assert_eq!(agrees(enclosed(Enclosure::Angled, vec![])), "<>");
    assert_eq!(agrees(opaque(Boundary::CurlyQuotes, "a b")), "“a b”");
    assert_eq!(agrees(opaque(Boundary::CurlyQuotes, "")), "“”");
    assert_eq!(agrees(opaque(Boundary::Parentheses, "x")), "(x)");
}

#[test]
fn heads_and_chains() {
    assert_eq!(agrees(dot("Some", bare("42"))), "Some.42");
    assert_eq!(
        agrees(dot(
            "Reviewer",
            enclosed(Enclosure::Braced, vec![bare("2024"), bare("17")])
        )),
        "Reviewer.{ 2024 17 }"
    );
    assert_eq!(
        agrees(dot(
            "Observed",
            dot("Locks", enclosed(Enclosure::Bracketed, vec![]))
        )),
        "Observed.Locks.[]"
    );
    assert_eq!(
        agrees(Protoform::Headed(
            sym("a"),
            Separator::Colon,
            Box::new(Protoform::Headed(
                sym("b"),
                Separator::Exclamation,
                Box::new(bare("c"))
            ))
        )),
        "a:b!c"
    );
    assert_eq!(
        agrees(dot("Some", opaque(Boundary::Parentheses, "x y"))),
        "Some.(x y)"
    );
}

#[test]
fn qualified_heads() {
    assert_eq!(
        agrees(Protoform::Qualified(
            Symbol::try_from("Vector").unwrap(),
            vec![bare("Text")]
        )),
        "Vector<Text>"
    );
    assert_eq!(
        agrees(Protoform::Headed(
            Head::Qualified(
                Symbol::try_from("A").unwrap(),
                vec![bare("B"), enclosed(Enclosure::Bracketed, vec![bare("C")])]
            ),
            Separator::Period,
            Box::new(enclosed(Enclosure::Braced, vec![bare("1")]))
        )),
        "A<B [ C ]>.{ 1 }"
    );
}

#[test]
fn siblings_one_space_apart() {
    let d = "a  { b }\n“c”".protosize().unwrap();
    assert_eq!(d.textualize(), "a { b } “c”");
    let Situated(_, text) = d.0[1].situate();
    assert_eq!(text, "{ b }");
    assert_eq!(Delineation(vec![]).textualize(), "");
}

#[test]
fn only_unbalanced_parentheses_and_backslashes_are_escaped() {
    assert_eq!(
        agrees(opaque(Boundary::Parentheses, "a (b) c")),
        "(a (b) c)"
    );
    assert_eq!(agrees(opaque(Boundary::Parentheses, "a ) b")), "(a \\) b)");
    assert_eq!(agrees(opaque(Boundary::Parentheses, "a ( b")), "(a \\( b)");
    assert_eq!(agrees(opaque(Boundary::Parentheses, "\\")), "(\\\\)");
    assert_eq!(agrees(opaque(Boundary::Parentheses, "((a)")), "(\\((a))");
    assert_eq!(agrees(opaque(Boundary::Parentheses, "(a))")), "((a)\\))");
    assert_eq!(agrees(opaque(Boundary::Parentheses, ")(")), "(\\)\\()");
    assert_eq!(
        agrees(opaque(
            Boundary::Parentheses,
            "The build passed on the third try (after two timeouts)"
        )),
        "(The build passed on the third try (after two timeouts))"
    );
}

#[test]
fn deep_situation_matches_the_reader() {
    let text = "{ Ada 1990 { “12 Rue de la Paix” Paris 75002 } [ Author Reviewer.{ 2024 17 } ] }";
    let Situated(_, form) =
        "{ Ada 1990 { “12 Rue de la Paix” Paris 75002 } [ Author Reviewer.{ 2024 17 } ] }"
            .protosize()
            .unwrap()
            .0
            .pop()
            .unwrap();
    assert_eq!(agrees(form), text);
}

#[test]
fn writer_extents_index_the_written_text() {
    let form = dot(
        "Reviewer",
        enclosed(Enclosure::Braced, vec![bare("2024"), bare("17")]),
    );
    let Situated(situation, text) = form.situate();
    let Extent(start, end) = situation.locate(&[1, 1]).unwrap();
    assert_eq!(&text[start as usize..end as usize], "17");
    let Extent(start, end) = situation.locate(&[0]).unwrap();
    assert_eq!(&text[start as usize..end as usize], "Reviewer");
}

#[test]
fn text_refuses_the_closing_curly_quote() {
    assert_eq!(
        Text::try_from("a”b"),
        Err(Refusal {
            glyph: '”',
            offset: 1
        })
    );
    assert_eq!(
        Text::try_from(String::from("xy”")),
        Err(Refusal {
            glyph: '”',
            offset: 2
        })
    );
    let text = Text::try_from("a “ b ( ) { } ; \\").unwrap();
    assert_eq!(text.as_ref(), "a “ b ( ) { } ; \\");
    assert_eq!(String::from(text), "a “ b ( ) { } ; \\");
}

#[test]
fn bare_admits_only_the_reader_bare_anatomy() {
    assert!(Bare::try_from("a:b").is_err());
    assert!(Bare::try_from("a..b").is_ok());
    assert!(Word::try_from("a:b").is_ok());
}

proptest! {
    #[test]
    fn any_meaning_text_round_trips(content in ".*") {
        let form = opaque(Boundary::Parentheses, &content);
        let text = form.textualize();
        let mut read = text.protosize().unwrap().0;
        prop_assert_eq!(read.len(), 1);
        let Situated(_, back) = read.pop().unwrap();
        prop_assert_eq!(back, form);
    }

    #[test]
    fn any_quoted_text_round_trips(content in "[^”]*") {
        let form = opaque(Boundary::CurlyQuotes, &content);
        let text = form.textualize();
        let mut read = text.protosize().unwrap().0;
        prop_assert_eq!(read.len(), 1);
        let Situated(_, back) = read.pop().unwrap();
        prop_assert_eq!(back, form);
    }
}
