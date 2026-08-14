use protos::{
    BlockScanning, CursorObserving, FrameObserving, Headed, Realize, RealizeDriving, RealizeWalk,
    Shape, ShapeDefined, SourceSlicing, SourceText, StringCarrier, StringCarrying, StructuralWalk,
    Textualize, TextualizeDriving, TextualizeWalk, Walk, WalkFault, WalkObserving,
};

#[derive(Debug, Eq, PartialEq)]
enum ExampleSelection {
    Bare,
    Quoted,
}

struct Example {
    selection: ExampleSelection,
}

struct RecursiveFixture {
    source: SourceText,
}

trait RecursiveReading {
    fn read(&self, walk: &mut RealizeWalk, block: &protos::Block) -> Result<(), WalkFault>;
}

trait RecursiveWriting {
    fn write_document(&self, walk: &mut TextualizeWalk) -> Result<(), WalkFault>;
    fn write_report(&self, walk: &mut TextualizeWalk) -> Result<(), WalkFault>;
    fn write_map(&self, walk: &mut TextualizeWalk) -> Result<(), WalkFault>;
    fn write_vector(&self, walk: &mut TextualizeWalk) -> Result<(), WalkFault>;
}

impl RecursiveReading for RecursiveFixture {
    fn read(&self, walk: &mut RealizeWalk, block: &protos::Block) -> Result<(), WalkFault> {
        match block.shape {
            Shape::DottedBraced => {
                assert_eq!(walk.position(), 0, "Report begins at its own position");
                assert_eq!(
                    self.source.source_slice(block.span.clone()),
                    Some("Report.{ Map.[ [ Entry.(child sees } ] only as text) ] ] }")
                );
                walk.realize_body(&block.body, block.body_span.start, &mut |driver, child| {
                    self.read(driver, child)
                })?;
                assert_eq!(walk.position(), 1, "Map close resumes Report exactly once");
                assert_eq!(
                    walk.observation().last_closed.expect("Map closed").shape(),
                    Shape::DottedSquareBracketed
                );
            }
            Shape::DottedSquareBracketed => {
                assert_eq!(walk.position(), 0, "Map begins before its Vector child");
                assert_eq!(
                    self.source.source_slice(block.span.clone()),
                    Some("Map.[ [ Entry.(child sees } ] only as text) ] ]")
                );
                walk.realize_body(&block.body, block.body_span.start, &mut |driver, child| {
                    self.read(driver, child)
                })?;
                assert_eq!(walk.position(), 1, "Vector close resumes Map exactly once");
            }
            Shape::SquareBracketed => {
                assert_eq!(walk.position(), 0, "Vector begins before its Entry child");
                assert_eq!(
                    self.source.source_slice(block.span.clone()),
                    Some("[ Entry.(child sees } ] only as text) ]")
                );
                walk.realize_body(&block.body, block.body_span.start, &mut |driver, child| {
                    self.read(driver, child)
                })?;
                assert_eq!(
                    walk.position(),
                    1,
                    "Entry close resumes Vector exactly once"
                );
            }
            Shape::DottedParenthesized => {
                assert_eq!(
                    walk.position(),
                    0,
                    "Entry starts while Vector remains unchanged"
                );
                assert_eq!(
                    self.source.source_slice(block.span.clone()),
                    Some("Entry.(child sees } ] only as text)")
                );
                assert_eq!(
                    block
                        .string_carrier
                        .as_ref()
                        .expect("carrier")
                        .textual_body(),
                    "child sees } ] only as text"
                );
            }
            _ => return Err(WalkFault::InvalidHead),
        }
        Ok(())
    }
}

impl RecursiveWriting for RecursiveFixture {
    fn write_document(&self, walk: &mut TextualizeWalk) -> Result<(), WalkFault> {
        walk.textualize_block(
            Shape::DottedBraced,
            Some(&protos::Head("Report".into())),
            |driver| self.write_report(driver),
        )
    }

    fn write_report(&self, walk: &mut TextualizeWalk) -> Result<(), WalkFault> {
        assert_eq!(walk.position(), 0, "Report begins before Map");
        walk.textualize_block(
            Shape::DottedSquareBracketed,
            Some(&protos::Head("Map".into())),
            |driver| self.write_map(driver),
        )?;
        assert_eq!(walk.position(), 1, "Map close resumes Report exactly once");
        let closed = walk.observation().last_closed.expect("Map output span");
        assert_eq!(
            walk.textual_source().source_slice(closed.span()),
            Some("Map.[[Entry.(child sees } ] only as text)]]")
        );
        Ok(())
    }

    fn write_map(&self, walk: &mut TextualizeWalk) -> Result<(), WalkFault> {
        assert_eq!(walk.position(), 0, "Map begins before Vector");
        walk.textualize_block(Shape::SquareBracketed, None, |driver| {
            self.write_vector(driver)
        })?;
        assert_eq!(walk.position(), 1, "Vector close resumes Map exactly once");
        let closed = walk.observation().last_closed.expect("Vector output span");
        assert_eq!(
            walk.textual_source().source_slice(closed.span()),
            Some("[Entry.(child sees } ] only as text)]")
        );
        Ok(())
    }

    fn write_vector(&self, walk: &mut TextualizeWalk) -> Result<(), WalkFault> {
        assert_eq!(walk.position(), 0, "Vector begins before Entry");
        walk.textualize_block(
            Shape::DottedParenthesized,
            Some(&protos::Head("Entry".into())),
            |driver| {
                assert_eq!(
                    driver.position(),
                    0,
                    "Entry is live before its carrier body"
                );
                driver.emit_text("child sees } ] only as text");
                Ok(())
            },
        )?;
        assert_eq!(
            walk.position(),
            1,
            "Entry close resumes Vector exactly once"
        );
        let closed = walk.observation().last_closed.expect("Entry output span");
        assert_eq!(
            walk.textual_source().source_slice(closed.span()),
            Some("Entry.(child sees } ] only as text)")
        );
        Ok(())
    }
}

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
        blocks[0].string_carrier,
        Some(StringCarrier::Parenthesized(
            "inside } ] (nested) and “quote".into()
        ))
    );
    assert_eq!(
        blocks[1].string_carrier,
        Some(StringCarrier::Bare("tail".into()))
    );
    assert_eq!(
        blocks[0].textualize(),
        SourceText("Note.(inside } ] (nested) and “quote)".into())
    );
}

#[test]
fn first_pass_recognises_ruled_structural_blocks_and_heads() {
    let source = SourceText("Report.{Map.[x] Tags.[y] {bare}}".into());
    let report = source.blocks().expect("report block").remove(0);
    assert_eq!(report.shape, Shape::DottedBraced);
    assert_eq!(report.head().expect("Report head").0, "Report");
    let interior = report.body.blocks().expect("report contents");
    assert_eq!(interior[0].shape, Shape::DottedSquareBracketed);
    assert_eq!(interior[0].head().expect("Map head").0, "Map");
    assert_eq!(interior[1].shape, Shape::DottedSquareBracketed);
    assert_eq!(interior[1].head().expect("Tags head").0, "Tags");
    assert_eq!(interior[2].shape, Shape::Braced);
    assert_eq!(interior[2].head(), None);
    assert_eq!(report.textualize(), source);
}

#[test]
fn structural_blocks_keep_nested_ruled_parenthesis_strings_opaque() {
    let source = SourceText(
        "Group.{ (Deep } ] “quote) [ Note.tail ] Map.[ remark.(child sees } ] and (nested markup) only as text) ] }".into(),
    );
    let group = source.blocks().expect("whole Group block").remove(0);
    assert_eq!(group.shape, Shape::DottedBraced);
    assert_eq!(group.head().expect("Group head").0, "Group");
    assert_eq!(group.textualize(), source);

    let contents = group.body.blocks().expect("Group contents");
    assert_eq!(contents.len(), 3);
    assert_eq!(contents[0].shape, Shape::Parenthesized);
    assert_eq!(
        contents[0].string_carrier,
        Some(StringCarrier::Parenthesized("Deep } ] “quote".into()))
    );
    assert_eq!(contents[1].shape, Shape::SquareBracketed);
    assert_eq!(contents[2].shape, Shape::DottedSquareBracketed);
    assert_eq!(contents[2].head().expect("Map head").0, "Map");

    let map_contents = contents[2].body.blocks().expect("Map contents");
    assert_eq!(map_contents.len(), 1);
    assert_eq!(map_contents[0].shape, Shape::DottedParenthesized);
    assert_eq!(map_contents[0].head().expect("remark head").0, "remark");
    assert_eq!(
        map_contents[0].string_carrier,
        Some(StringCarrier::Parenthesized(
            "child sees } ] and (nested markup) only as text".into()
        ))
    );
}

#[test]
fn structural_blocks_balance_cross_nested_braces_and_squares() {
    let source =
        SourceText("Outer.{ [Inner.{(} stays string) {x}}] Tail.[ {y] stays string} ] }".into());
    let outer = source.blocks().expect("outer").remove(0);
    assert_eq!(outer.shape, Shape::DottedBraced);
    assert_eq!(outer.head().expect("Outer head").0, "Outer");
    assert_eq!(outer.textualize(), source);

    let body = outer.body.blocks().expect("Outer body");
    assert_eq!(body[0].shape, Shape::SquareBracketed);
    assert_eq!(body[0].head(), None);
    let square_contents = body[0].body.blocks().expect("square contents");
    assert_eq!(square_contents[0].shape, Shape::DottedBraced);
    assert_eq!(square_contents[0].head().expect("Inner head").0, "Inner");

    assert_eq!(body[1].shape, Shape::DottedSquareBracketed);
    assert_eq!(body[1].head().expect("Tail head").0, "Tail");
    let tail_contents = body[1].body.blocks().expect("tail contents");
    assert_eq!(tail_contents[0].shape, Shape::Braced);
    assert_eq!(tail_contents[0].head(), None);
}

#[test]
fn structural_blocks_keep_escaped_unbalanced_parentheses_inside_strings() {
    let source = SourceText("Group.{ remark.(a lone \\( remains text) }".into());
    let group = source.blocks().expect("Group").remove(0);
    assert_eq!(group.textualize(), source);
    let remark = group.body.blocks().expect("content").remove(0);
    assert_eq!(remark.shape, Shape::DottedParenthesized);
    assert_eq!(
        remark.string_carrier,
        Some(StringCarrier::Parenthesized(
            "a lone \\( remains text".into()
        ))
    );
}

#[test]
fn shape_definition_only_selects() {
    let selection = Example {
        selection: ExampleSelection::Bare,
    };
    assert_eq!(selection.selection, ExampleSelection::Bare);
    assert_eq!(Example::shapes(), &[Shape::Bare, Shape::CurlyQuoted]);
    assert_eq!(
        Example::select(Shape::Bare, None),
        Some(ExampleSelection::Bare)
    );
    assert_eq!(Example::select(Shape::Parenthesized, None), None);
    // The signature grants only shape and head; no block interior can be consumed.
    assert_eq!(
        Example::select(Shape::Bare, Some(&protos::Head("Tag".into()))),
        None
    );
}

#[test]
fn scanner_extents_are_utf8_bytes_and_safely_slice_unicode_source() {
    let source = SourceText("Note.(“é”) tail".into());
    let block = source.blocks().expect("blocks").remove(0);
    assert_eq!(source.source_slice(block.span.clone()), Some("Note.(“é”)"));
    assert_eq!(block.span.end, "Note.(“é”)".len());
}

#[test]
fn drivers_use_real_source_and_output_spans_then_finish_balanced_and_reusable() {
    let source = SourceText("Outer.(child } ] (nested)) tail".into());
    let mut real = RealizeWalk::default();
    let blocks = real.realize_blocks(&source).expect("real source");
    assert_eq!(real.cursor(), source.0.len());
    assert_eq!(real.observation().depth, 0);
    assert_eq!(real.observation().resumptions, 2);
    assert_eq!(
        source.source_slice(blocks[0].span.clone()),
        Some("Outer.(child } ] (nested))")
    );

    let mut textual = TextualizeWalk::default();
    let rendered = textual.textualize_blocks(&blocks);
    assert_eq!(rendered, source);
    assert_eq!(textual.cursor(), rendered.0.len());
    assert_eq!(textual.observation().depth, 0);
    assert_eq!(textual.observation().resumptions, 2);
    assert_eq!(
        textual.textualize_blocks(&blocks),
        source,
        "a finished driver is reusable"
    );
}

#[test]
fn textual_walk_is_canonical_block_projection_not_source_format_preservation() {
    let source = SourceText("one\n\n  two\tthree".into());
    let mut walk = RealizeWalk::default();
    let blocks = walk.realize_blocks(&source).expect("blocks");
    let canonical = TextualizeWalk::default().textualize_blocks(&blocks);
    assert_eq!(canonical, SourceText("one two three".into()));
}

#[test]
fn one_child_close_requires_exactly_one_parent_resume() {
    let source = SourceText("Outer.(child } ] (nested)) tail".into());
    let outer = source.blocks().expect("outer block").remove(0);
    let nested_start = source.0.find("(nested)").expect("nested child");
    let nested_span = nested_start..nested_start + "(nested)".len();

    let mut walk = StructuralWalk::default();
    walk.enter(outer.shape, outer.span.clone());
    walk.enter(Shape::Parenthesized, nested_span);
    let child = walk.close().expect("child frame");
    assert_eq!(child.shape(), Shape::Parenthesized);
    assert_eq!(source.source_slice(child.span()), Some("(nested)"));
    assert_eq!(walk.position(), 0, "close does not resume a parent");
    walk.resume();
    assert_eq!(walk.position(), 1);
    walk.resume();
    assert_eq!(
        walk.position(),
        1,
        "duplicate resume is refused by neutral state"
    );
    assert_eq!(walk.observation().resumptions, 1);
    walk.close();
    assert_eq!(walk.observation().depth, 0);
}

#[test]
fn scoped_drivers_keep_recursive_dialect_lifecycle_in_one_neutral_walk() {
    let fixture = RecursiveFixture {
        source: SourceText("Report.{ Map.[ [ Entry.(child sees } ] only as text) ] ] }".into()),
    };

    let mut realize = RealizeWalk::default();
    realize
        .realize_source::<(), WalkFault, _>(&fixture.source, |walk, block| {
            fixture.read(walk, block)
        })
        .expect("scoped source realization");
    assert_eq!(realize.cursor(), fixture.source.0.len());
    assert_eq!(realize.observation().depth, 0);
    assert_eq!(realize.observation().resumptions, 4);
    assert_eq!(
        realize
            .observation()
            .last_closed
            .expect("document root")
            .span(),
        0..fixture.source.0.len()
    );

    let mut textualize = TextualizeWalk::default();
    textualize
        .textualize_source::<(), WalkFault, _>(|walk| fixture.write_document(walk))
        .expect("scoped textualization");
    let output = textualize.textual_source();
    assert_eq!(
        output,
        SourceText("Report.{Map.[[Entry.(child sees } ] only as text)]]}".into()),
        "the driver projects canonical structural text rather than source trivia"
    );
    assert_eq!(textualize.cursor(), output.0.len());
    assert_eq!(textualize.observation().depth, 0);
    assert_eq!(textualize.observation().resumptions, 4);
    assert_eq!(
        textualize
            .observation()
            .last_closed
            .expect("document root")
            .span(),
        0..output.0.len()
    );
}

#[test]
fn scoped_driver_failure_closes_without_resuming_and_becomes_unusable() {
    let source = SourceText("Report.{ one }".into());
    let mut walk = RealizeWalk::default();
    let failure =
        walk.realize_source::<(), WalkFault, _>(&source, |_, _| Err(WalkFault::InvalidHead));
    assert_eq!(failure, Err(WalkFault::InvalidHead));
    assert_eq!(walk.observation().depth, 0);
    assert_eq!(
        walk.observation().resumptions,
        0,
        "failed child has no false resume"
    );
    assert!(walk.observation().faulted);
    assert_eq!(
        walk.realize_source::<(), WalkFault, _>(&source, |_, _| Ok(())),
        Err(WalkFault::FaultedWalk)
    );
}
