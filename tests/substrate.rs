use protos::{
    BlockScanning, CursorObserving, FrameObserving, Headed, IdentityObserving, ObservationViewing,
    ParentObserving, Realize, RealizeDriving, RealizeScope, RealizeScoping, RealizeWalk, Shape,
    ShapeDefined, SourceSlicing, SourceText, StringCarrier, StringCarrying, StructuralWalk,
    Textualize, TextualizeDriving, TextualizeScope, TextualizeScoping, TextualizeWalk,
    TransitionObserving, Walk, WalkFault, WalkObserving, WalkTransitionKind,
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

struct SiblingFixture {
    source: SourceText,
}

struct SiblingValue {
    selection: SiblingSelection,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum SiblingSelection {
    First,
    Second,
}

trait SiblingReading {
    fn read(
        &self,
        scope: &mut RealizeScope<'_>,
        block: &protos::Block,
    ) -> Result<SiblingValue, WalkFault>;
}

trait SiblingWriting {
    fn write(&self, scope: &mut TextualizeScope<'_>, value: &SiblingValue)
    -> Result<(), WalkFault>;
}

impl ShapeDefined for SiblingValue {
    type Selection = SiblingSelection;

    fn shapes() -> &'static [Shape] {
        &[Shape::DottedParenthesized]
    }

    fn select(shape: Shape, head: Option<&protos::Head>) -> Option<Self::Selection> {
        let first = protos::Head("First".into());
        let second = protos::Head("Second".into());
        match (shape, head) {
            (Shape::DottedParenthesized, Some(value)) if value == &first => {
                Some(SiblingSelection::First)
            }
            (Shape::DottedParenthesized, Some(value)) if value == &second => {
                Some(SiblingSelection::Second)
            }
            _ => None,
        }
    }
}

impl SiblingReading for SiblingFixture {
    fn read(
        &self,
        _scope: &mut RealizeScope<'_>,
        block: &protos::Block,
    ) -> Result<SiblingValue, WalkFault> {
        let selection =
            SiblingValue::select(block.shape, block.head()).ok_or(WalkFault::InvalidHead)?;
        Ok(SiblingValue { selection })
    }
}

impl SiblingWriting for SiblingFixture {
    fn write(
        &self,
        scope: &mut TextualizeScope<'_>,
        value: &SiblingValue,
    ) -> Result<(), WalkFault> {
        let head = match value.selection {
            SiblingSelection::First => protos::Head("First".into()),
            SiblingSelection::Second => protos::Head("Second".into()),
        };
        scope.textualize_block(Shape::DottedParenthesized, Some(&head), |body| {
            body.emit_scalar(match value.selection {
                SiblingSelection::First => "é",
                SiblingSelection::Second => "two",
            });
            Ok(())
        })
    }
}

trait RecursiveReading {
    fn read(&self, scope: &mut RealizeScope<'_>, block: &protos::Block) -> Result<(), WalkFault>;
}

trait RecursiveWriting {
    fn write_document(&self, scope: &mut TextualizeScope<'_>) -> Result<(), WalkFault>;
    fn write_report(&self, scope: &mut TextualizeScope<'_>) -> Result<(), WalkFault>;
    fn write_map(&self, scope: &mut TextualizeScope<'_>) -> Result<(), WalkFault>;
    fn write_vector(&self, scope: &mut TextualizeScope<'_>) -> Result<(), WalkFault>;
}

impl RecursiveReading for RecursiveFixture {
    fn read(&self, scope: &mut RealizeScope<'_>, block: &protos::Block) -> Result<(), WalkFault> {
        match block.shape {
            Shape::DottedBraced => {
                assert_eq!(
                    self.source.source_slice(block.span.clone()),
                    Some("Report.{ Map.[ [ Entry.(child sees } ] only as text) ] ] }")
                );
                scope.realize_body(&mut |child_scope, child| self.read(child_scope, child))?;
            }
            Shape::DottedSquareBracketed => {
                assert_eq!(
                    self.source.source_slice(block.span.clone()),
                    Some("Map.[ [ Entry.(child sees } ] only as text) ] ]")
                );
                scope.realize_body(&mut |child_scope, child| self.read(child_scope, child))?;
            }
            Shape::SquareBracketed => {
                assert_eq!(
                    self.source.source_slice(block.span.clone()),
                    Some("[ Entry.(child sees } ] only as text) ]")
                );
                scope.realize_body(&mut |child_scope, child| self.read(child_scope, child))?;
            }
            Shape::DottedParenthesized => {
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
    fn write_document(&self, scope: &mut TextualizeScope<'_>) -> Result<(), WalkFault> {
        scope.textualize_block(
            Shape::DottedBraced,
            Some(&protos::Head("Report".into())),
            |driver| self.write_report(driver),
        )
    }

    fn write_report(&self, scope: &mut TextualizeScope<'_>) -> Result<(), WalkFault> {
        scope.textualize_block(
            Shape::DottedSquareBracketed,
            Some(&protos::Head("Map".into())),
            |driver| self.write_map(driver),
        )?;
        Ok(())
    }

    fn write_map(&self, scope: &mut TextualizeScope<'_>) -> Result<(), WalkFault> {
        scope.textualize_block(Shape::SquareBracketed, None, |driver| {
            self.write_vector(driver)
        })?;
        Ok(())
    }

    fn write_vector(&self, scope: &mut TextualizeScope<'_>) -> Result<(), WalkFault> {
        scope.textualize_block(
            Shape::DottedParenthesized,
            Some(&protos::Head("Entry".into())),
            |driver| {
                driver.emit_scalar("child sees } ] only as text");
                Ok(())
            },
        )?;
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
    assert_eq!(real.observation().depth(), 0);
    assert_eq!(real.observation().resumptions(), 2);
    assert_eq!(
        source.source_slice(blocks[0].span.clone()),
        Some("Outer.(child } ] (nested))")
    );

    let mut textual = TextualizeWalk::default();
    let rendered = textual.textualize_blocks(&blocks);
    assert_eq!(rendered, source);
    assert_eq!(textual.cursor(), rendered.0.len());
    assert_eq!(textual.observation().depth(), 0);
    assert_eq!(textual.observation().resumptions(), 2);
    assert_eq!(
        textual.textualize_blocks(&blocks),
        source,
        "a finished driver is reusable"
    );
    let output_observation = textual.observation();
    assert_eq!(output_observation.history()[0].ordinal(), 0);
    assert_eq!(
        output_observation.history()[0].frame().identity().ordinal(),
        0
    );
}

#[test]
fn realization_cursor_covers_trailing_utf8_trivia_and_resets_on_reuse() {
    let mut walk = RealizeWalk::default();
    let trailing = SourceText("é  \n".into());
    walk.realize_source::<(), WalkFault, _>(&trailing, |_, _| Ok(()))
        .expect("trailing trivia source");
    assert_eq!(walk.cursor(), trailing.0.len());

    let empty = SourceText(String::new());
    walk.realize_source::<(), WalkFault, _>(&empty, |_, _| Ok(()))
        .expect("empty source reuse");
    assert_eq!(walk.cursor(), 0);

    let following = SourceText("two ".into());
    walk.realize_source::<(), WalkFault, _>(&following, |_, _| Ok(()))
        .expect("second nonempty source reuse");
    assert_eq!(walk.cursor(), following.0.len());
    let observation = walk.observation();
    assert_eq!(observation.history()[0].ordinal(), 0);
    assert_eq!(observation.history()[0].frame().identity().ordinal(), 0);
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
    assert_eq!(walk.observation().resumptions(), 1);
    walk.close();
    assert_eq!(walk.observation().depth(), 0);
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
    assert_eq!(realize.observation().depth(), 0);
    assert_eq!(realize.observation().resumptions(), 4);
    assert_eq!(
        realize
            .observation()
            .last_closed()
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
    let report = output.blocks().expect("rescanned report").remove(0);
    assert_eq!(report.shape, Shape::DottedBraced);
    assert_eq!(report.head().expect("Report head").0, "Report");
    let map = report.body.blocks().expect("rescanned Map").remove(0);
    assert_eq!(map.shape, Shape::DottedSquareBracketed);
    assert_eq!(map.head().expect("Map head").0, "Map");
    let vector = map.body.blocks().expect("rescanned Vector").remove(0);
    assert_eq!(vector.shape, Shape::SquareBracketed);
    let entry = vector.body.blocks().expect("rescanned Entry").remove(0);
    assert_eq!(entry.shape, Shape::DottedParenthesized);
    assert_eq!(entry.head().expect("Entry head").0, "Entry");
    assert_eq!(textualize.cursor(), output.0.len());
    assert_eq!(textualize.observation().depth(), 0);
    assert_eq!(textualize.observation().resumptions(), 4);
    assert_eq!(
        textualize
            .observation()
            .last_closed()
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
    assert_eq!(walk.observation().depth(), 0);
    assert_eq!(
        walk.observation().resumptions(),
        0,
        "failed child has no false resume"
    );
    assert!(walk.observation().faulted());
    assert_eq!(
        walk.realize_source::<(), WalkFault, _>(&source, |_, _| Ok(())),
        Err(WalkFault::FaultedWalk)
    );
}

#[test]
fn textual_scope_failure_closes_without_resuming_and_becomes_unusable() {
    let mut walk = TextualizeWalk::default();
    let failure = walk.textualize_source::<(), WalkFault, _>(|scope| {
        scope.emit_scalar("é ");
        Err(WalkFault::InvalidHead)
    });
    assert_eq!(failure, Err(WalkFault::InvalidHead));
    assert_eq!(walk.observation().depth(), 0);
    assert_eq!(
        walk.observation().resumptions(),
        0,
        "failed root has no resume"
    );
    assert!(walk.observation().faulted());
    assert_eq!(walk.textual_source(), SourceText("é ".into()));
    assert_eq!(walk.cursor(), "é ".len());
    let observation = walk.observation();
    let root_close = observation.history().last().expect("partial root close");
    assert_eq!(root_close.kind(), WalkTransitionKind::Close);
    assert_eq!(root_close.frame().shape(), Shape::Bare);
    assert_eq!(root_close.frame().span(), 0.."é ".len());
    assert_eq!(root_close.parent_before(), None);
    assert_eq!(root_close.parent_after(), None);
    assert!(
        observation
            .history()
            .iter()
            .all(|transition| transition.kind() != WalkTransitionKind::Resume)
    );
    assert_eq!(
        walk.textualize_source::<(), WalkFault, _>(|_| Ok(())),
        Err(WalkFault::FaultedWalk)
    );
}

#[test]
fn textual_scope_rejects_shape_head_mismatches_before_emission() {
    let parenthesized = TextualizeWalk::default();
    let mut parenthesized = parenthesized;
    let parenthesized_result = parenthesized.textualize_source::<(), WalkFault, _>(|scope| {
        scope.textualize_block(
            Shape::Parenthesized,
            Some(&protos::Head("Wrong".into())),
            |_| Ok(()),
        )
    });
    assert_eq!(parenthesized_result, Err(WalkFault::InvalidHead));
    assert_eq!(parenthesized.textual_source(), SourceText(String::new()));
    assert_eq!(parenthesized.observation().depth(), 0);
    assert_eq!(parenthesized.observation().resumptions(), 0);

    let mut dotted = TextualizeWalk::default();
    let dotted_result = dotted.textualize_source::<(), WalkFault, _>(|scope| {
        scope.textualize_block(Shape::DottedBraced, None, |_| Ok(()))
    });
    assert_eq!(dotted_result, Err(WalkFault::InvalidHead));
    assert_eq!(dotted.textual_source(), SourceText(String::new()));
    assert_eq!(dotted.observation().depth(), 0);
    assert_eq!(dotted.observation().resumptions(), 0);

    let mut empty = TextualizeWalk::default();
    let empty_result = empty.textualize_source::<(), WalkFault, _>(|scope| {
        scope.textualize_block(
            Shape::DottedParenthesized,
            Some(&protos::Head(String::new())),
            |_| Ok(()),
        )
    });
    assert_eq!(empty_result, Err(WalkFault::InvalidHead));
    assert_eq!(empty.textual_source(), SourceText(String::new()));
    assert_eq!(empty.observation().depth(), 0);
    assert_eq!(empty.observation().resumptions(), 0);
}

#[test]
fn branded_realization_scope_ignores_a_forged_callback_block_and_keeps_utf8_spans() {
    let source = SourceText("Outer.{ é child }".into());
    let mut walk = RealizeWalk::default();
    let mut actual = Vec::new();
    walk.realize_source::<(), WalkFault, _>(&source, |scope, block| {
        assert_eq!(
            source.source_slice(block.span.clone()),
            Some("Outer.{ é child }")
        );
        let mut forged = block.clone();
        forged.body = SourceText("forged".into());
        forged.body_span = 0..6;
        forged.span = 0..6;
        assert_eq!(forged.body.0, "forged");

        scope.realize_body(&mut |_, child| {
            actual.push((child.span.clone(), source.source_slice(child.span.clone())));
            Ok(())
        })?;
        Ok(())
    })
    .expect("the live scope uses its driver-branded source rather than forged Block data");
    assert_eq!(
        actual,
        vec![
            ("Outer.{ ".len().."Outer.{ é".len(), Some("é")),
            ("Outer.{ é ".len().."Outer.{ é child".len(), Some("child")),
        ]
    );
    assert_eq!(walk.cursor(), source.0.len());
    assert_eq!(walk.observation().depth(), 0);
}

#[test]
fn typed_shape_defined_siblings_use_scoped_lifecycle_and_actual_extents() {
    let fixture = SiblingFixture {
        source: SourceText("Parent.{ First.(é) Second.(two) }".into()),
    };
    let parent = fixture.source.blocks().expect("parent block").remove(0);
    let mut realized = Vec::new();
    let mut source_spans = Vec::new();
    let mut realize = RealizeWalk::default();

    realize
        .realize_source::<(), WalkFault, _>(&fixture.source, |root, block| {
            assert_eq!(block.shape, Shape::DottedBraced);
            root.realize_body(&mut |scope, child| {
                source_spans.push((
                    child.span.clone(),
                    fixture.source.source_slice(child.span.clone()),
                ));
                realized.push(fixture.read(scope, child)?);
                Ok(())
            })?;
            Ok(())
        })
        .expect("two typed sibling values");

    assert_eq!(parent.shape, Shape::DottedBraced);
    assert_eq!(
        realized
            .into_iter()
            .map(|value| value.selection)
            .collect::<Vec<_>>(),
        vec![SiblingSelection::First, SiblingSelection::Second]
    );
    assert_eq!(
        source_spans,
        vec![
            (
                "Parent.{ ".len().."Parent.{ First.(é)".len(),
                Some("First.(é)")
            ),
            (
                "Parent.{ First.(é) ".len().."Parent.{ First.(é) Second.(two)".len(),
                Some("Second.(two)")
            ),
        ]
    );
    assert_eq!(realize.observation().resumptions(), 3);
    assert_eq!(realize.cursor(), fixture.source.0.len());

    let values = vec![
        SiblingValue {
            selection: SiblingSelection::First,
        },
        SiblingValue {
            selection: SiblingSelection::Second,
        },
    ];
    let mut textualize = TextualizeWalk::default();
    textualize
        .textualize_source::<(), WalkFault, _>(|root| {
            root.textualize_block(
                Shape::DottedBraced,
                Some(&protos::Head("Parent".into())),
                |parent_scope| {
                    for value in &values {
                        fixture.write(parent_scope, value)?;
                    }
                    Ok(())
                },
            )
        })
        .expect("typed sibling projection");
    let output = textualize.textual_source();
    assert_eq!(output, SourceText("Parent.{First.(é) Second.(two)}".into()));
    let output_parent = output.blocks().expect("output parent").remove(0);
    let output_children = output_parent.body.blocks().expect("output siblings");
    assert_eq!(output_children.len(), 2);
    assert_eq!(
        output_children[0].head().expect("First head"),
        &protos::Head("First".into())
    );
    assert_eq!(
        output_children[1].head().expect("Second head"),
        &protos::Head("Second".into())
    );
    let first_output_span = (output_parent.body_span.start + output_children[0].span.start)
        ..(output_parent.body_span.start + output_children[0].span.end);
    let second_output_span = (output_parent.body_span.start + output_children[1].span.start)
        ..(output_parent.body_span.start + output_children[1].span.end);
    assert_eq!(output.source_slice(first_output_span), Some("First.(é)"));
    assert_eq!(
        output.source_slice(second_output_span),
        Some("Second.(two)")
    );
    assert_eq!(textualize.observation().resumptions(), 3);
    assert_eq!(textualize.cursor(), output.0.len());
}

#[test]
fn driver_transition_history_binds_typed_siblings_to_actual_source_and_output() {
    let fixture = SiblingFixture {
        source: SourceText("Parent.{ First.(é) Second.(two) }".into()),
    };
    let mut source_walk = RealizeWalk::default();
    source_walk
        .realize_source::<(), WalkFault, _>(&fixture.source, |root, block| {
            assert_eq!(block.shape, Shape::DottedBraced);
            root.realize_body(&mut |scope, child| fixture.read(scope, child).map(|_| ()))?;
            Ok(())
        })
        .expect("typed source siblings");
    let source_observation = source_walk.observation();
    let source_history = source_observation.history();
    assert_eq!(source_history[0].ordinal(), 0);
    assert_eq!(source_history[0].kind(), WalkTransitionKind::Enter);
    assert_eq!(source_history[0].frame().identity().ordinal(), 0);

    let source_sibling_closes = source_history
        .iter()
        .filter(|transition| {
            transition.kind() == WalkTransitionKind::Close
                && transition.frame().shape() == Shape::DottedParenthesized
        })
        .collect::<Vec<_>>();
    assert_eq!(source_sibling_closes.len(), 2);
    for (index, close) in source_sibling_closes.into_iter().enumerate() {
        let parent_before = close.parent_before().expect("live Parent frame");
        let parent_after = close.parent_after().expect("unchanged Parent frame");
        assert_eq!(
            parent_before, parent_after,
            "child close does not advance Parent"
        );
        let resume = &source_history[close.ordinal() + 1];
        assert_eq!(resume.kind(), WalkTransitionKind::Resume);
        assert_eq!(resume.frame().identity(), parent_before.frame());
        assert_eq!(resume.parent_before(), Some(parent_before));
        assert_eq!(
            resume.parent_after().expect("advanced parent").position(),
            parent_before.position() + 1
        );
        assert_eq!(
            fixture.source.source_slice(close.frame().span()),
            if index == 0 {
                Some("First.(é)")
            } else {
                Some("Second.(two)")
            },
        );
    }
    let source_root_close = source_history.last().expect("root close");
    assert_eq!(source_root_close.kind(), WalkTransitionKind::Close);
    assert_eq!(source_root_close.frame().shape(), Shape::Bare);
    assert_eq!(source_root_close.parent_before(), None);
    assert_eq!(source_root_close.parent_after(), None);
    assert!(!source_observation.faulted());

    let values = vec![
        SiblingValue {
            selection: SiblingSelection::First,
        },
        SiblingValue {
            selection: SiblingSelection::Second,
        },
    ];
    let mut output_walk = TextualizeWalk::default();
    output_walk
        .textualize_source::<(), WalkFault, _>(|root| {
            root.textualize_block(
                Shape::DottedBraced,
                Some(&protos::Head("Parent".into())),
                |parent| {
                    for value in &values {
                        fixture.write(parent, value)?;
                    }
                    Ok(())
                },
            )
        })
        .expect("typed sibling output");
    let output = output_walk.textual_source();
    let output_observation = output_walk.observation();
    let output_history = output_observation.history();
    assert_eq!(
        source_history
            .iter()
            .map(|transition| (transition.kind(), transition.frame().shape()))
            .collect::<Vec<_>>(),
        output_history
            .iter()
            .map(|transition| (transition.kind(), transition.frame().shape()))
            .collect::<Vec<_>>(),
        "the same neutral machine recorded the source and output structural pattern",
    );
    let output_sibling_closes = output_history
        .iter()
        .filter(|transition| {
            transition.kind() == WalkTransitionKind::Close
                && transition.frame().shape() == Shape::DottedParenthesized
        })
        .collect::<Vec<_>>();
    assert_eq!(output_sibling_closes.len(), 2);
    assert_eq!(
        output.source_slice(output_sibling_closes[0].frame().span()),
        Some("First.(é)")
    );
    assert_eq!(
        output.source_slice(output_sibling_closes[1].frame().span()),
        Some("Second.(two)")
    );
    for close in output_sibling_closes {
        let parent_before = close.parent_before().expect("live output Parent frame");
        assert_eq!(close.parent_after(), Some(parent_before));
        let resume = &output_history[close.ordinal() + 1];
        assert_eq!(resume.kind(), WalkTransitionKind::Resume);
        assert_eq!(resume.frame().identity(), parent_before.frame());
        assert_eq!(resume.parent_before(), Some(parent_before));
        assert_eq!(
            resume
                .parent_after()
                .expect("advanced output parent")
                .position(),
            parent_before.position() + 1
        );
    }
    assert!(!output_observation.faulted());
}

#[test]
fn transition_history_keeps_nested_parent_positions_and_fault_closes_without_resume() {
    let source = SourceText("Root.{ Branch.[ Leaf.(child sees } ] (nested)) ] Tail.(two) }".into());
    let mut walk = RealizeWalk::default();
    walk.realize_source::<(), WalkFault, _>(&source, |root, block| {
        assert_eq!(block.shape, Shape::DottedBraced);
        root.realize_body(&mut |branch_scope, child| {
            if child.shape == Shape::DottedSquareBracketed {
                branch_scope.realize_body(&mut |_, leaf| {
                    assert_eq!(leaf.shape, Shape::DottedParenthesized);
                    Ok(())
                })?;
            }
            Ok(())
        })?;
        Ok(())
    })
    .expect("nested source");
    let nested_observation = walk.observation();
    let nested_history = nested_observation.history();
    let leaf_close = nested_history
        .iter()
        .find(|transition| {
            transition.kind() == WalkTransitionKind::Close
                && transition.frame().shape() == Shape::DottedParenthesized
        })
        .expect("Leaf close");
    let leaf_parent = leaf_close.parent_before().expect("Branch parent");
    assert_eq!(leaf_close.parent_after(), Some(leaf_parent));
    let leaf_resume = &nested_history[leaf_close.ordinal() + 1];
    assert_eq!(leaf_resume.kind(), WalkTransitionKind::Resume);
    assert_eq!(leaf_resume.frame().identity(), leaf_parent.frame());
    assert_eq!(leaf_resume.parent_before(), Some(leaf_parent));
    assert_eq!(
        leaf_resume
            .parent_after()
            .expect("Branch advanced")
            .position(),
        1
    );
    assert_eq!(
        source.source_slice(leaf_close.frame().span()),
        Some("Leaf.(child sees } ] (nested))")
    );

    let failed_source = SourceText("Root.{ child }".into());
    let mut failed = RealizeWalk::default();
    assert_eq!(
        failed
            .realize_source::<(), WalkFault, _>(&failed_source, |_, _| Err(WalkFault::InvalidHead)),
        Err(WalkFault::InvalidHead)
    );
    let failure_observation = failed.observation();
    let failure_history = failure_observation.history();
    assert!(failure_observation.faulted());
    assert!(
        failure_history
            .iter()
            .all(|transition| transition.kind() != WalkTransitionKind::Resume)
    );
    let failure_root_close = failure_history.last().expect("fault-closed root");
    assert_eq!(failure_root_close.kind(), WalkTransitionKind::Close);
    assert_eq!(failure_root_close.frame().shape(), Shape::Bare);
    assert_eq!(failure_root_close.parent_before(), None);
    assert_eq!(failure_root_close.parent_after(), None);
}

#[test]
fn nested_textual_failure_records_the_actual_partial_child_extent() {
    let mut walk = TextualizeWalk::default();
    let failure = walk.textualize_source::<(), WalkFault, _>(|root| {
        root.textualize_block(
            Shape::DottedBraced,
            Some(&protos::Head("Parent".into())),
            |parent| {
                parent.textualize_block(
                    Shape::DottedParenthesized,
                    Some(&protos::Head("Child".into())),
                    |child| {
                        child.emit_scalar("é");
                        Err(WalkFault::InvalidHead)
                    },
                )
            },
        )
    });
    assert_eq!(failure, Err(WalkFault::InvalidHead));
    let partial = SourceText("Parent.{Child.(é".into());
    assert_eq!(walk.textual_source(), partial);
    assert_eq!(walk.cursor(), partial.0.len());
    let observation = walk.observation();
    assert!(observation.faulted());
    assert!(
        observation
            .history()
            .iter()
            .all(|transition| transition.kind() != WalkTransitionKind::Resume)
    );
    let closes = observation
        .history()
        .iter()
        .filter(|transition| transition.kind() == WalkTransitionKind::Close)
        .collect::<Vec<_>>();
    assert_eq!(
        closes.len(),
        3,
        "Child, Parent, then document root close once"
    );
    assert_eq!(closes[0].frame().shape(), Shape::DottedParenthesized);
    assert_eq!(closes[1].frame().shape(), Shape::DottedBraced);
    assert_eq!(closes[2].frame().shape(), Shape::Bare);
    assert_eq!(
        closes
            .iter()
            .map(|transition| transition.ordinal())
            .collect::<Vec<_>>(),
        vec![3, 4, 5],
        "the leaf close is followed only by fault-closes of its live parents",
    );
    assert_eq!(closes[0].frame().span(), "Parent.{".len()..partial.0.len());
    assert_eq!(closes[1].frame().span(), 0..partial.0.len());
    assert_eq!(closes[2].frame().span(), 0..partial.0.len());
    assert_eq!(
        partial.source_slice(closes[0].frame().span()),
        Some("Child.(é")
    );
}
