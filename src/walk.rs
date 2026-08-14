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

/// A direction layer which scopes universal source blocks through the neutral walk.
pub trait RealizeDriving {
    fn realize_blocks(&mut self, source: &SourceText) -> Result<Vec<Block>, WalkFault>;

    fn realize_source<T, E, F>(&mut self, source: &SourceText, dialect: F) -> Result<Vec<T>, E>
    where
        E: From<WalkFault>,
        F: FnMut(&mut Self, &Block) -> Result<T, E>;

    fn realize_body<T, E, F>(
        &mut self,
        source: &SourceText,
        origin: usize,
        dialect: &mut F,
    ) -> Result<Vec<T>, E>
    where
        E: From<WalkFault>,
        F: FnMut(&mut Self, &Block) -> Result<T, E>;
}

/// A direction layer which scopes universal output blocks through the neutral walk.
pub trait TextualizeDriving {
    fn textualize_blocks(&mut self, blocks: &[Block]) -> SourceText;

    fn textualize_source<T, E, F>(&mut self, dialect: F) -> Result<T, E>
    where
        E: From<WalkFault>,
        F: FnOnce(&mut Self) -> Result<T, E>;

    fn textualize_block<T, E, F>(
        &mut self,
        shape: Shape,
        head: Option<&Head>,
        dialect: F,
    ) -> Result<T, E>
    where
        E: From<WalkFault>,
        F: FnOnce(&mut Self) -> Result<T, E>;

    fn emit_text(&mut self, text: &str);
    fn textual_source(&self) -> SourceText;
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
        F: FnMut(&mut Self, &Block) -> Result<T, E>,
    {
        if self.is_faulted() {
            return Err(E::from(WalkFault::FaultedWalk));
        }
        self.enter(Shape::Bare, 0..source.0.len());
        let result = self.realize_body(source, 0, &mut dialect);
        match result {
            Ok(values) => {
                self.structural.finish(0..source.0.len());
                self.close();
                Ok(values)
            }
            Err(error) => {
                self.fail();
                Err(error)
            }
        }
    }

    fn realize_body<T, E, F>(
        &mut self,
        source: &SourceText,
        origin: usize,
        dialect: &mut F,
    ) -> Result<Vec<T>, E>
    where
        E: From<WalkFault>,
        F: FnMut(&mut Self, &Block) -> Result<T, E>,
    {
        if self.is_faulted() {
            return Err(E::from(WalkFault::FaultedWalk));
        }
        let mut values = Vec::new();
        for mut block in source.blocks().map_err(E::from)? {
            block.span = (origin + block.span.start)..(origin + block.span.end);
            block.body_span = (origin + block.body_span.start)..(origin + block.body_span.end);
            self.enter(block.shape, block.span.clone());
            self.source_cursor = block.span.end;
            match dialect(self, &block) {
                Ok(value) => {
                    self.close();
                    self.resume();
                    self.source_cursor = block.span.end;
                    values.push(value);
                }
                Err(error) => {
                    self.close();
                    self.fail();
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
                        body.emit_text(&block.body.0);
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
        F: FnOnce(&mut Self) -> Result<T, E>,
    {
        if self.is_faulted() {
            return Err(E::from(WalkFault::FaultedWalk));
        }
        self.output = SourceText(String::new());
        self.emission_cursor = 0;
        self.enter(Shape::Bare, 0..0);
        match dialect(self) {
            Ok(value) => {
                self.structural.finish(0..self.output.0.len());
                self.emission_cursor = self.output.0.len();
                self.close();
                Ok(value)
            }
            Err(error) => {
                self.fail();
                Err(error)
            }
        }
    }

    fn textualize_block<T, E, F>(
        &mut self,
        shape: Shape,
        head: Option<&Head>,
        dialect: F,
    ) -> Result<T, E>
    where
        E: From<WalkFault>,
        F: FnOnce(&mut Self) -> Result<T, E>,
    {
        if self.is_faulted() {
            return Err(E::from(WalkFault::FaultedWalk));
        }
        if self.position() != 0 {
            self.emit_text(" ");
        }
        let start = self.output.0.len();
        if let Some(head) = head {
            self.emit_text(&head.0);
            self.emit_text(".");
        }
        let delimiters = match shape {
            Shape::Bare => (None, None),
            Shape::CurlyQuoted | Shape::DottedCurlyQuoted => (Some('“'), Some('”')),
            Shape::Parenthesized | Shape::DottedParenthesized => (Some('('), Some(')')),
            Shape::SquareBracketed | Shape::DottedSquareBracketed => (Some('['), Some(']')),
            Shape::Braced | Shape::DottedBraced => (Some('{'), Some('}')),
        };
        if let Some(opening) = delimiters.0 {
            self.emit_text(&opening.to_string());
        }
        self.enter(shape, start..start);
        match dialect(self) {
            Ok(value) => {
                if let Some(closing) = delimiters.1 {
                    self.emit_text(&closing.to_string());
                }
                let end = self.output.0.len();
                self.structural.finish(start..end);
                self.emission_cursor = end;
                self.close();
                self.resume();
                Ok(value)
            }
            Err(error) => {
                self.close();
                self.fail();
                Err(error)
            }
        }
    }

    fn emit_text(&mut self, text: &str) {
        if !self.is_faulted() {
            self.output.0.push_str(text);
            self.emission_cursor = self.output.0.len();
        }
    }

    fn textual_source(&self) -> SourceText {
        self.output.clone()
    }
}
