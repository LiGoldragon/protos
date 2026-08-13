use protos::{
    BlockScanning, Headed, Realize, RealizeDriving, RealizeWalk, Shape, ShapeDefined, SourceText,
    StringCarrier, StructuralWalk, Textualize, TextualizeDriving, TextualizeWalk, Walk,
    WalkTracing,
};

#[derive(Debug, Eq, PartialEq)]
enum ExampleSelection {
    Bare,
    Quoted,
}

struct Example;

impl ShapeDefined for Example {
    type Selection = ExampleSelection;

    fn shapes() -> &'static [Shape] {
        &[Shape::Bare, Shape::CurlyQuoted]
    }
    fn select(shape: Shape, head: Option<&protos::Head>) -> Option<Self::Selection> {
        match shape {
            Shape::Bare if head.is_none() => Some(ExampleSelection::Bare),
            Shape::CurlyQuoted if head.is_none() => Some(ExampleSelection::Quoted),
            _ => None,
        }
    }
}

#[test]
fn first_pass_keeps_string_interior_opaque_and_carries_head() {
    let source = SourceText("Note.(inside } ] (nested) and “quote) tail".into());
    let blocks = source.realize().expect("a balanced string block");
    assert_eq!(blocks.len(), 2);
    assert_eq!(blocks[0].head().expect("a head").0, "Note");
    assert_eq!(blocks[0].shape, Shape::DottedParenthesized);
    assert_eq!(
        blocks[0].body,
        StringCarrier::Parenthesized("inside } ] (nested) and “quote".into())
    );
    assert_eq!(blocks[1].body, StringCarrier::Bare("tail".into()));
}

#[test]
fn textual_blocks_round_trip_without_dialect_meaning() {
    let source = SourceText("Remark.(a } ] string) plain".into());
    let rendered: String = source
        .blocks()
        .expect("blocks")
        .into_iter()
        .map(|block| block.textualize().0)
        .collect::<Vec<_>>()
        .join(" ");
    assert_eq!(rendered, source.0);
}

#[test]
fn shape_definition_only_selects() {
    assert_eq!(Example::shapes(), &[Shape::Bare, Shape::CurlyQuoted]);
    assert_eq!(
        Example::select(Shape::Bare, None),
        Some(ExampleSelection::Bare)
    );
    assert_eq!(Example::select(Shape::Parenthesized, None), None);
    // Only shape and head reach `select`; the string body cannot be consumed.
    assert_eq!(
        Example::select(Shape::Bare, Some(&protos::Head("Tag".into()))),
        None
    );
}

#[test]
fn walks_bind_source_spans_and_emission_to_one_parent_resume() {
    let source = SourceText("Outer.(child } ] (nested)) tail".into());
    let mut real = RealizeWalk::default();
    let blocks = real.realize_blocks(&source).expect("real source");
    assert_eq!(real.source_cursor, source.0.chars().count());
    assert_eq!(real.position(), 2);
    assert_eq!(real.structural.resumptions, 2);

    let mut textual = TextualizeWalk::default();
    let rendered = textual.textualize_blocks(&blocks);
    assert_eq!(rendered, source);
    assert_eq!(textual.emission_cursor, rendered.0.chars().count());
    assert_eq!(textual.position(), 2);
    assert_eq!(textual.structural.resumptions, 2);
}

#[test]
fn child_close_leaves_parent_position_until_the_one_resume() {
    let source = SourceText("Outer.(child } ] (nested)) tail".into());
    let outer = source.blocks().expect("outer block").remove(0);
    let child_start = outer
        .body
        .textualize()
        .0
        .find("(nested)")
        .expect("nested child")
        + outer.span.start;
    let child_span = child_start..child_start + "(nested)".chars().count();

    let mut walk = StructuralWalk::default();
    walk.enter(outer.shape, outer.span.clone());
    assert_eq!(walk.position(), 0);
    walk.enter(Shape::Parenthesized, child_span);
    assert_eq!(walk.position(), 0);
    assert_eq!(
        walk.close().expect("child frame").shape,
        Shape::Parenthesized
    );
    assert_eq!(walk.position(), 0, "close does not resume a parent");
    walk.resume();
    assert_eq!(
        walk.position(),
        1,
        "one explicit resume advances exactly once"
    );
    assert_eq!(walk.structural_resumptions(), 1);
}
