use std::ops::Range;

use crate::{Block, BlockScanning, Head, Shape, SourceText};

/// A read-only record of one completed structural frame.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WalkFrame {
    shape: Shape,
    position: usize,
    span: Range<usize>,
}

/// Read-only facts about a structural walk.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WalkObservation {
    pub depth: usize,
    pub resumptions: usize,
    pub last_closed: Option<WalkFrame>,
    pub faulted: bool,
}

/// A structural failure which is independent of any dialect's meanings.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum WalkFault {
    UnexpectedCloser(char),
    UnclosedBlock(Shape),
    InvalidHead,
    FaultedWalk,
}

/// The one structural frame discipline, shared by both directions.
pub trait Walk {
    fn enter(&mut self, shape: Shape, span: Range<usize>);
    fn close(&mut self) -> Option<WalkFrame>;
    fn position(&self) -> usize;
    fn resume(&mut self);
}

/// Read-only access to transition evidence; callers cannot mutate frames.
pub trait WalkObserving {
    fn observation(&self) -> WalkObservation;
}

/// Read-only access to a completed frame's structural facts.
pub trait FrameObserving {
    fn shape(&self) -> Shape;
    fn position(&self) -> usize;
    fn span(&self) -> Range<usize>;
}

/// The neutral owner of every frame transition.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct StructuralWalk {
    frames: Vec<WalkFrame>,
    resumptions: usize,
    awaiting_resume: bool,
    last_closed: Option<WalkFrame>,
}

trait FrameFinishing {
    fn finish(&mut self, span: Range<usize>);
}

trait WalkAborting {
    fn abort(&mut self);
}

impl Walk for StructuralWalk {
    fn enter(&mut self, shape: Shape, span: Range<usize>) {
        if self.awaiting_resume {
            return;
        }
        self.frames.push(WalkFrame {
            shape,
            position: 0,
            span,
        });
    }

    fn close(&mut self) -> Option<WalkFrame> {
        if self.awaiting_resume {
            return None;
        }
        let closed = self.frames.pop()?;
        self.awaiting_resume = !self.frames.is_empty();
        self.last_closed = Some(closed.clone());
        Some(closed)
    }

    fn position(&self) -> usize {
        self.frames.last().map_or(0, |frame| frame.position)
    }

    fn resume(&mut self) {
        if !self.awaiting_resume {
            return;
        }
        if let Some(frame) = self.frames.last_mut() {
            frame.position += 1;
            self.resumptions += 1;
        }
        self.awaiting_resume = false;
    }
}

impl FrameFinishing for StructuralWalk {
    fn finish(&mut self, span: Range<usize>) {
        if let Some(frame) = self.frames.last_mut() {
            frame.span = span;
        }
    }
}

impl WalkAborting for StructuralWalk {
    fn abort(&mut self) {
        self.frames.clear();
        self.awaiting_resume = false;
    }
}

impl WalkObserving for StructuralWalk {
    fn observation(&self) -> WalkObservation {
        WalkObservation {
            depth: self.frames.len(),
            resumptions: self.resumptions,
            last_closed: self.last_closed.clone(),
            faulted: false,
        }
    }
}

impl FrameObserving for WalkFrame {
    fn shape(&self) -> Shape {
        self.shape
    }

    fn position(&self) -> usize {
        self.position
    }

    fn span(&self) -> Range<usize> {
        self.span.clone()
    }
}

/// The text-to-real driver. It binds neutral transitions to source block spans.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct RealizeWalk {
    structural: StructuralWalk,
    source_cursor: usize,
    faulted: bool,
}

/// The real-to-text driver. It binds neutral transitions to emitted text.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct TextualizeWalk {
    structural: StructuralWalk,
    output: SourceText,
    emission_cursor: usize,
    faulted: bool,
}

/// The restricted, data-bearing realization handle a dialect receives while a
/// lexical block frame is live.
///
/// ```compile_fail
/// use protos::{RealizeScope, Walk};
/// fn corrupt(scope: &mut RealizeScope<'_>) { Walk::close(scope); }
/// ```
pub struct RealizeScope<'a> {
    driver: &'a mut RealizeWalk,
    body: SourceText,
    body_span: Range<usize>,
}

/// The restricted, data-bearing textualization handle a dialect receives
/// while a structural output frame is live.
///
/// ```compile_fail
/// use protos::{TextualizeScope, Walk};
/// fn corrupt(scope: &mut TextualizeScope<'_>) { Walk::resume(scope); }
/// ```
pub struct TextualizeScope<'a> {
    driver: &'a mut TextualizeWalk,
}

/// A direction layer which scopes universal source blocks through the neutral walk.
pub trait RealizeDriving {
    fn realize_blocks(&mut self, source: &SourceText) -> Result<Vec<Block>, WalkFault>;

    fn realize_source<T, E, F>(&mut self, source: &SourceText, dialect: F) -> Result<Vec<T>, E>
    where
        E: From<WalkFault>,
        F: FnMut(&mut RealizeScope<'_>, &Block) -> Result<T, E>;
}

/// A direction layer which scopes universal output blocks through the neutral walk.
pub trait TextualizeDriving {
    fn textualize_blocks(&mut self, blocks: &[Block]) -> SourceText;

    fn textualize_source<T, E, F>(&mut self, dialect: F) -> Result<T, E>
    where
        E: From<WalkFault>,
        F: FnOnce(&mut TextualizeScope<'_>) -> Result<T, E>;

    fn textual_source(&self) -> SourceText;
}

/// Recursive realization available only inside a driver-owned live scope.
///
/// ```compile_fail
/// use protos::{RealizeScope, RealizeScoping, WalkFault};
/// fn lie(scope: &mut RealizeScope<'_>) -> Result<(), WalkFault> {
///     scope.realize_body(12, &mut |_, _| Ok(()))?;
///     Ok(())
/// }
/// ```
pub trait RealizeScoping {
    fn realize_body<T, E, F>(&mut self, dialect: &mut F) -> Result<Vec<T>, E>
    where
        E: From<WalkFault>,
        F: FnMut(&mut RealizeScope<'_>, &Block) -> Result<T, E>;
}

/// Safe dialect emission within a driver-owned live output scope.
pub trait TextualizeScoping {
    fn textualize_block<T, E, F>(
        &mut self,
        shape: Shape,
        head: Option<&Head>,
        dialect: F,
    ) -> Result<T, E>
    where
        E: From<WalkFault>,
        F: FnOnce(&mut TextualizeScope<'_>) -> Result<T, E>;

    fn emit_scalar(&mut self, text: &str);
}

trait ShapeHeading {
    fn accepts_head(&self, head: Option<&Head>) -> bool;
}

impl ShapeHeading for Shape {
    fn accepts_head(&self, head: Option<&Head>) -> bool {
        match (self, head) {
            (
                Shape::DottedCurlyQuoted
                | Shape::DottedParenthesized
                | Shape::DottedSquareBracketed
                | Shape::DottedBraced,
                Some(Head(value)),
            ) => !value.is_empty(),
            (
                Shape::Bare
                | Shape::CurlyQuoted
                | Shape::Parenthesized
                | Shape::SquareBracketed
                | Shape::Braced,
                None,
            ) => true,
            _ => false,
        }
    }
}

/// Read-only cursor evidence recorded by either direction driver.
pub trait CursorObserving {
    fn cursor(&self) -> usize;
}

trait DriverFailing {
    fn fail(&mut self);
    fn is_faulted(&self) -> bool;
}

impl DriverFailing for RealizeWalk {
    fn fail(&mut self) {
        self.structural.abort();
        self.faulted = true;
    }

    fn is_faulted(&self) -> bool {
        self.faulted
    }
}

impl DriverFailing for TextualizeWalk {
    fn fail(&mut self) {
        self.structural.abort();
        self.faulted = true;
    }

    fn is_faulted(&self) -> bool {
        self.faulted
    }
}

impl Walk for RealizeWalk {
    fn enter(&mut self, shape: Shape, span: Range<usize>) {
        self.structural.enter(shape, span);
    }

    fn close(&mut self) -> Option<WalkFrame> {
        self.structural.close()
    }

    fn position(&self) -> usize {
        self.structural.position()
    }

    fn resume(&mut self) {
        self.structural.resume();
    }
}

impl Walk for TextualizeWalk {
    fn enter(&mut self, shape: Shape, span: Range<usize>) {
        self.structural.enter(shape, span);
    }

    fn close(&mut self) -> Option<WalkFrame> {
        self.structural.close()
    }

    fn position(&self) -> usize {
        self.structural.position()
    }

    fn resume(&mut self) {
        self.structural.resume();
    }
}

impl WalkObserving for RealizeWalk {
    fn observation(&self) -> WalkObservation {
        let mut observation = self.structural.observation();
        observation.faulted = self.faulted;
        observation
    }
}

impl WalkObserving for TextualizeWalk {
    fn observation(&self) -> WalkObservation {
        let mut observation = self.structural.observation();
        observation.faulted = self.faulted;
        observation
    }
}

impl CursorObserving for RealizeWalk {
    fn cursor(&self) -> usize {
        self.source_cursor
    }
}

impl CursorObserving for TextualizeWalk {
    fn cursor(&self) -> usize {
        self.emission_cursor
    }
}

impl RealizeDriving for RealizeWalk {
    fn realize_blocks(&mut self, source: &SourceText) -> Result<Vec<Block>, WalkFault> {
        self.realize_source::<Block, WalkFault, _>(source, |_, block| Ok(block.clone()))
    }

    fn realize_source<T, E, F>(&mut self, source: &SourceText, mut dialect: F) -> Result<Vec<T>, E>
    where
        E: From<WalkFault>,
        F: FnMut(&mut RealizeScope<'_>, &Block) -> Result<T, E>,
    {
        if self.is_faulted() {
            return Err(E::from(WalkFault::FaultedWalk));
        }
        self.enter(Shape::Bare, 0..source.0.len());
        let mut scope = RealizeScope {
            driver: self,
            body: source.clone(),
            body_span: 0..source.0.len(),
        };
        let result = scope.realize_body(&mut dialect);
        match result {
            Ok(values) => {
                scope.driver.structural.finish(0..source.0.len());
                scope.driver.close();
                Ok(values)
            }
            Err(error) => {
                scope.driver.fail();
                Err(error)
            }
        }
    }
}

impl RealizeScoping for RealizeScope<'_> {
    fn realize_body<T, E, F>(&mut self, dialect: &mut F) -> Result<Vec<T>, E>
    where
        E: From<WalkFault>,
        F: FnMut(&mut RealizeScope<'_>, &Block) -> Result<T, E>,
    {
        if self.driver.is_faulted() {
            return Err(E::from(WalkFault::FaultedWalk));
        }
        let mut values = Vec::new();
        for mut block in self.body.blocks().map_err(E::from)? {
            block.span =
                (self.body_span.start + block.span.start)..(self.body_span.start + block.span.end);
            block.body_span = (self.body_span.start + block.body_span.start)
                ..(self.body_span.start + block.body_span.end);
            self.driver.enter(block.shape, block.span.clone());
            self.driver.source_cursor = block.span.end;
            let mut child_scope = RealizeScope {
                driver: &mut *self.driver,
                body: block.body.clone(),
                body_span: block.body_span.clone(),
            };
            match dialect(&mut child_scope, &block) {
                Ok(value) => {
                    child_scope.driver.close();
                    child_scope.driver.resume();
                    child_scope.driver.source_cursor = block.span.end;
                    values.push(value);
                }
                Err(error) => {
                    child_scope.driver.close();
                    child_scope.driver.fail();
                    return Err(error);
                }
            }
        }
        Ok(values)
    }
}

impl TextualizeDriving for TextualizeWalk {
    fn textualize_blocks(&mut self, blocks: &[Block]) -> SourceText {
        let result = self.textualize_source::<(), WalkFault, _>(|walk| {
            for block in blocks {
                walk.textualize_block::<(), WalkFault, _>(
                    block.shape,
                    block.head.as_ref(),
                    |body| {
                        body.emit_scalar(&block.body.0);
                        Ok(())
                    },
                )?;
            }
            Ok(())
        });
        if result.is_err() {
            return SourceText(String::new());
        }
        self.textual_source()
    }

    fn textualize_source<T, E, F>(&mut self, dialect: F) -> Result<T, E>
    where
        E: From<WalkFault>,
        F: FnOnce(&mut TextualizeScope<'_>) -> Result<T, E>,
    {
        if self.is_faulted() {
            return Err(E::from(WalkFault::FaultedWalk));
        }
        self.output = SourceText(String::new());
        self.emission_cursor = 0;
        self.enter(Shape::Bare, 0..0);
        let mut scope = TextualizeScope { driver: self };
        match dialect(&mut scope) {
            Ok(value) => {
                scope
                    .driver
                    .structural
                    .finish(0..scope.driver.output.0.len());
                scope.driver.emission_cursor = scope.driver.output.0.len();
                scope.driver.close();
                Ok(value)
            }
            Err(error) => {
                scope.driver.close();
                scope.driver.fail();
                Err(error)
            }
        }
    }

    fn textual_source(&self) -> SourceText {
        self.output.clone()
    }
}

impl TextualizeScoping for TextualizeScope<'_> {
    fn textualize_block<T, E, F>(
        &mut self,
        shape: Shape,
        head: Option<&Head>,
        dialect: F,
    ) -> Result<T, E>
    where
        E: From<WalkFault>,
        F: FnOnce(&mut TextualizeScope<'_>) -> Result<T, E>,
    {
        if self.driver.is_faulted() {
            return Err(E::from(WalkFault::FaultedWalk));
        }
        if !shape.accepts_head(head) {
            return Err(E::from(WalkFault::InvalidHead));
        }
        if self.driver.position() != 0 {
            self.emit_scalar(" ");
        }
        let start = self.driver.output.0.len();
        if let Some(head) = head {
            self.emit_scalar(&head.0);
            self.emit_scalar(".");
        }
        let delimiters = match shape {
            Shape::Bare => (None, None),
            Shape::CurlyQuoted | Shape::DottedCurlyQuoted => (Some('“'), Some('”')),
            Shape::Parenthesized | Shape::DottedParenthesized => (Some('('), Some(')')),
            Shape::SquareBracketed | Shape::DottedSquareBracketed => (Some('['), Some(']')),
            Shape::Braced | Shape::DottedBraced => (Some('{'), Some('}')),
        };
        if let Some(opening) = delimiters.0 {
            self.emit_scalar(&opening.to_string());
        }
        self.driver.enter(shape, start..start);
        match dialect(self) {
            Ok(value) => {
                if let Some(closing) = delimiters.1 {
                    self.emit_scalar(&closing.to_string());
                }
                let end = self.driver.output.0.len();
                self.driver.structural.finish(start..end);
                self.driver.emission_cursor = end;
                self.driver.close();
                self.driver.resume();
                Ok(value)
            }
            Err(error) => {
                self.driver.close();
                self.driver.fail();
                Err(error)
            }
        }
    }

    fn emit_scalar(&mut self, text: &str) {
        if !self.driver.is_faulted() {
            self.driver.output.0.push_str(text);
            self.driver.emission_cursor = self.driver.output.0.len();
        }
    }
}
