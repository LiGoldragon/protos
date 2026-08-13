use std::ops::Range;

use crate::{Block, Realize, Shape, SourceText, Textualize};

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
}

/// A structural failure which is independent of any dialect's meanings.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum WalkFault {
    UnexpectedCloser(char),
    UnclosedBlock(Shape),
    InvalidHead,
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

impl WalkObserving for StructuralWalk {
    fn observation(&self) -> WalkObservation {
        WalkObservation {
            depth: self.frames.len(),
            resumptions: self.resumptions,
            last_closed: self.last_closed.clone(),
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
}

/// The real-to-text driver. It binds neutral transitions to emitted text.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct TextualizeWalk {
    structural: StructuralWalk,
    emission_cursor: usize,
}

/// A direction layer that consumes textual source through the neutral walk.
pub trait RealizeDriving {
    fn realize_blocks(&mut self, source: &SourceText) -> Result<Vec<Block>, WalkFault>;
}

/// A direction layer that emits textual source through the neutral walk.
pub trait TextualizeDriving {
    fn textualize_blocks(&mut self, blocks: &[Block]) -> SourceText;
}

/// Read-only cursor evidence recorded by either direction driver.
pub trait CursorObserving {
    fn cursor(&self) -> usize;
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
        self.structural.observation()
    }
}
impl WalkObserving for TextualizeWalk {
    fn observation(&self) -> WalkObservation {
        self.structural.observation()
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
        let blocks = source.realize()?;
        self.enter(Shape::Bare, 0..source.0.len());
        for block in &blocks {
            self.enter(block.shape, block.span.clone());
            self.source_cursor = block.span.end;
            self.close();
            self.resume();
        }
        self.close();
        Ok(blocks)
    }
}

impl TextualizeDriving for TextualizeWalk {
    fn textualize_blocks(&mut self, blocks: &[Block]) -> SourceText {
        let mut output = String::new();
        self.enter(Shape::Bare, 0..0);
        for (index, block) in blocks.iter().enumerate() {
            if index != 0 {
                output.push(' ');
            }
            let start = output.len();
            output.push_str(&block.textualize().0);
            let end = output.len();
            self.enter(block.shape, start..end);
            self.emission_cursor = end;
            self.close();
            self.resume();
        }
        self.close();
        SourceText(output)
    }
}
