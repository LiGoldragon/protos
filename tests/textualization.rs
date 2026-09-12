//! Writing: canonical spacing, the escape rule of each opaque boundary, and
//! the round trip a built structure owes — print it, read it back, get it back.

use proptest::prelude::*;
use protos::{
    Boundary, Canonicalizable, Enclosure, Extent, Protos, Protosizable, Separator, Symbol,
    Textualizable,
};

/// Structures are built here, not read, so they start with no extent of their
/// own; `agrees` canonicalizes before printing and every extent is then a fact
/// about the text that comes out.
const NOWHERE: Extent = Extent { start: 0, end: 0 };

fn bare(text: &str) -> Protos {
    Protos::Bare {
        extent: NOWHERE,
        text: text.to_owned(),
    }
}
fn enclosed(enclosure: Enclosure, children: Vec<Protos>) -> Protos {
    Protos::Enclosed {
        extent: NOWHERE,
        enclosure,
        children,
    }
}
fn opaque(boundary: Boundary, content: &str) -> Protos {
    Protos::Opaque {
        extent: NOWHERE,
        boundary,
        content: content.to_owned(),
    }
}
fn headed(head: &str, separator: Separator, body: Protos) -> Protos {
    Protos::Headed {
        extent: NOWHERE,
        head: Symbol(head.to_owned()),
        constraints: None,
        separator,
        body: Box::new(body),
    }
}
fn dot(head: &str, body: Protos) -> Protos {
    headed(head, Separator::Period, body)
}
fn constrained(head: &str, constraints: Vec<Protos>, body: Protos) -> Protos {
    Protos::Headed {
        extent: NOWHERE,
        head: Symbol(head.to_owned()),
        constraints: Some(Box::new(enclosed(Enclosure::Angled, constraints))),
        separator: Separator::Period,
        body: Box::new(body),
    }
}

/// A built structure, canonicalized, printed, and read back whole — extents
/// included. This is the direction the ascent of a dialect actually travels.
fn agrees(mut form: Protos) -> String {
    form.canonicalize();
    let text = form.textualize();
    let read = text
        .protosize()
        .unwrap_or_else(|error| panic!("{text:?} reads back: {error}"));
    assert_eq!(read, form, "{text:?} is not the structure that wrote it");
    text
}

#[test]
fn canonical_spacing_puts_one_space_inside_a_non_empty_enclosure() {
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
    assert_eq!(agrees(opaque(Boundary::Guillemets, "a b")), "«a b»");
    assert_eq!(agrees(opaque(Boundary::Guillemets, "")), "«»");
    assert_eq!(agrees(opaque(Boundary::Parentheses, "x")), "(x)");
}

#[test]
fn a_head_meets_its_body_with_nothing_between_them() {
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
        agrees(headed(
            "a",
            Separator::Colon,
            headed("b", Separator::Exclamation, bare("c"))
        )),
        "a:b!c"
    );
    assert_eq!(
        agrees(dot("Some", opaque(Boundary::Parentheses, "x y"))),
        "Some.(x y)"
    );
    assert_eq!(
        agrees(dot("Some", opaque(Boundary::Guillemets, "x y"))),
        "Some.«x y»"
    );
}

#[test]
fn a_constrained_head_writes_its_angles_tight_against_the_name() {
    assert_eq!(
        agrees(constrained(
            "Vector",
            vec![bare("Text")],
            enclosed(Enclosure::Bracketed, vec![bare("1")])
        )),
        "Vector<Text>.[ 1 ]"
    );
    assert_eq!(
        agrees(constrained(
            "A",
            vec![bare("B"), enclosed(Enclosure::Bracketed, vec![bare("C")])],
            enclosed(Enclosure::Braced, vec![bare("1")])
        )),
        "A<B [ C ]>.{ 1 }"
    );
}

#[test]
fn only_an_unbalanced_parenthesis_or_an_ambiguous_backslash_is_escaped() {
    for (content, expected) in [
        ("a (b) c", "(a (b) c)"),
        ("a ) b", "(a \\) b)"),
        ("a ( b", "(a \\( b)"),
        ("\\", "(\\\\)"),
        ("a\\x", "(a\\x)"),
        ("((a)", "(\\((a))"),
        ("(a))", "((a)\\))"),
        (")(", "(\\)\\()"),
        ("a«b»c", "(a«b»c)"),
        (
            "The build passed on the third try (after two timeouts)",
            "(The build passed on the third try (after two timeouts))",
        ),
    ] {
        assert_eq!(
            agrees(opaque(Boundary::Parentheses, content)),
            expected,
            "{content:?}"
        );
    }
}

#[test]
fn only_a_closing_guillemet_or_an_ambiguous_backslash_is_escaped() {
    for (content, expected) in [
        ("a b", "«a b»"),
        ("a»b", "«a\\»b»"),
        ("she said »no» and left", "«she said \\»no\\» and left»"),
        // A backslash before a glyph it could escape must itself be escaped.
        ("\\", "«\\\\»"),
        ("\\»x", "«\\\\\\»x»"),
        ("a\\\\b", "«a\\\\\\b»"),
        // Before anything else a backslash is content and stays bare.
        ("a\\b", "«a\\b»"),
        ("C:\\Users\\ada", "«C:\\Users\\ada»"),
        // Guillemets do not nest, so an opener inside is plain content.
        ("a«b", "«a«b»"),
        ("a\\«b", "«a\\«b»"),
        // Every other delimiter is content too.
        ("{ [ ( ; ” “", "«{ [ ( ; ” “»"),
    ] {
        assert_eq!(
            agrees(opaque(Boundary::Guillemets, content)),
            expected,
            "{content:?}"
        );
    }
}

#[test]
fn an_opaque_leaf_keeps_its_siblings_apart() {
    // A mis-escaped closer would swallow what follows it rather than refuse.
    assert_eq!(
        agrees(enclosed(
            Enclosure::Braced,
            vec![
                opaque(Boundary::Guillemets, "a\\"),
                bare("b"),
                opaque(Boundary::Parentheses, "c\\"),
                bare("d"),
            ]
        )),
        "{ «a\\\\» b (c\\\\) d }"
    );
}

#[test]
fn a_read_tree_reprints_the_text_that_made_it() {
    for text in [
        "{ Ada 1990 { «12 Rue de la Paix» Paris 75002 } [ Author Reviewer.{ 2024 17 } ] }",
        "Processable<[ Clonable Sendable ] Serializable>.[ Vector <String> ]",
        "Some.(x (y) z)",
        "«a\\»b»",
    ] {
        let form = text.protosize().expect("structure");
        assert_eq!(form.textualize(), text, "{text:?}");
    }
}

/// The glyphs that decide an opaque region: both boundaries, the escape, the
/// structural delimiters, the comment opener, and ordinary content.
const ALPHABET: [char; 16] = [
    '\\', '«', '»', '(', ')', '{', '}', '[', ']', '<', '>', ';', ' ', '\n', 'a', '猫',
];

fn tricky_content() -> impl Strategy<Value = String> {
    prop::collection::vec(prop::sample::select(ALPHABET.as_slice()), 0..14)
        .prop_map(|glyphs| glyphs.into_iter().collect())
}

proptest! {
    #[test]
    fn any_guillemet_content_round_trips(content in tricky_content()) {
        let mut form = opaque(Boundary::Guillemets, &content);
        form.canonicalize();
        let text = form.textualize();
        prop_assert_eq!(text.protosize(), Ok(form), "{:?} wrote {:?}", content, text);
    }

    #[test]
    fn any_parentheses_content_round_trips(content in tricky_content()) {
        let mut form = opaque(Boundary::Parentheses, &content);
        form.canonicalize();
        let text = form.textualize();
        prop_assert_eq!(text.protosize(), Ok(form), "{:?} wrote {:?}", content, text);
    }

    #[test]
    fn arbitrary_guillemet_content_round_trips(content in ".*") {
        let mut form = opaque(Boundary::Guillemets, &content);
        form.canonicalize();
        prop_assert_eq!(form.textualize().protosize(), Ok(form));
    }

    #[test]
    fn arbitrary_parentheses_content_round_trips(content in ".*") {
        let mut form = opaque(Boundary::Parentheses, &content);
        form.canonicalize();
        prop_assert_eq!(form.textualize().protosize(), Ok(form));
    }

    /// An opaque leaf between siblings: a closer that escaped wrongly would
    /// end the region early and take the siblings with it.
    #[test]
    fn an_opaque_leaf_among_siblings_round_trips(
        first in tricky_content(),
        second in tricky_content(),
    ) {
        let mut form = enclosed(
            Enclosure::Braced,
            vec![
                bare("before"),
                opaque(Boundary::Guillemets, &first),
                bare("between"),
                opaque(Boundary::Parentheses, &second),
                bare("after"),
            ],
        );
        form.canonicalize();
        let text = form.textualize();
        prop_assert_eq!(text.protosize(), Ok(form), "wrote {:?}", text);
    }
}
