use std::ops::Range;

use crate::{Block, Realize, Shape, SourceText, Textualize};

/// A saved structural position. A child frame never changes its parent frame.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WalkFrame {
    pub shape: Shape,
    pub position: usize,
    pub span: Range<usize>,
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

/// Access to neutral transition evidence kept by the structural walk.
pub trait WalkTracing {
    fn structural_resumptions(&self) -> usize;
}

/// The neutral owner of every frame transition.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct StructuralWalk {
    pub frames: Vec<WalkFrame>,
    pub resumptions: usize,
}

impl Walk for StructuralWalk {
    fn enter(&mut self, shape: Shape, span: Range<usize>) {
        self.frames.push(WalkFrame {
            shape,
            position: 0,
            span,
        });
    }

    fn close(&mut self) -> Option<WalkFrame> {
        self.frames.pop()
    }

    fn position(&self) -> usize {
        self.frames.last().map_or(0, |frame| frame.position)
    }

    fn resume(&mut self) {
        if let Some(frame) = self.frames.last_mut() {
            frame.position += 1;
            self.resumptions += 1;
        }
    }
}

impl WalkTracing for StructuralWalk {
    fn structural_resumptions(&self) -> usize {
        self.resumptions
    }
}

/// The text-to-real driver. It binds neutral transitions to source block spans.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct RealizeWalk {
    pub structural: StructuralWalk,
    pub source_cursor: usize,
}

/// The real-to-text driver. It binds neutral transitions to emitted text.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct TextualizeWalk {
    pub structural: StructuralWalk,
    pub emission_cursor: usize,
}

/// A direction layer that consumes textual source through the neutral walk.
pub trait RealizeDriving {
    fn realize_blocks(&mut self, source: &SourceText) -> Result<Vec<Block>, WalkFault>;
}

/// A direction layer that emits textual source through the neutral walk.
pub trait TextualizeDriving {
    fn textualize_blocks(&mut self, blocks: &[Block]) -> SourceText;
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

impl RealizeDriving for RealizeWalk {
    fn realize_blocks(&mut self, source: &SourceText) -> Result<Vec<Block>, WalkFault> {
        let blocks = source.realize()?;
        self.enter(Shape::Bare, 0..source.0.chars().count());
        for block in &blocks {
            self.enter(block.shape, block.span.clone());
            self.source_cursor = block.span.end;
            self.close();
            self.resume();
        }
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
            let start = output.chars().count();
            let rendered = block.textualize();
            output.push_str(&rendered.0);
            let end = output.chars().count();
            self.enter(block.shape, start..end);
            self.emission_cursor = end;
            self.close();
            self.resume();
        }
        if let Some(root) = self.structural.frames.last_mut() {
            root.span = 0..output.chars().count();
        }
        SourceText(output)
    }
}
