//! Universal structural vocabulary for Protos-family dialects.
//!
//! This crate owns no dialect meanings. A dialect supplies its own real types
//! and implements the capability traits below; this crate supplies the fixed
//! textual shapes, lexical blocks, string carriers, and one frame discipline.

mod block;
mod form;
mod shape;
mod walk;

pub use block::{Block, BlockScanner, Head, SourceText, StringCarrier};
pub use form::{Realize, Textualize};
pub use shape::{Shape, ShapeDefined};
pub use walk::{
    CursorObserving, FrameObserving, RealizeDriving, RealizeScope, RealizeScoping, RealizeWalk,
    StructuralWalk, TextualizeDriving, TextualizeScope, TextualizeScoping, TextualizeWalk, Walk,
    WalkFault, WalkFrame, WalkObservation, WalkObserving,
};

/// A value that can disclose the optional dotted prefix carried by a block.
pub trait Headed {
    fn head(&self) -> Option<&Head>;
}

/// A lexical source that can be separated into universal structural blocks.
pub trait BlockScanning {
    fn blocks(&self) -> Result<Vec<Block>, WalkFault>;
}

/// Access to a string carrier's lexical body.
pub trait StringCarrying {
    fn textual_body(&self) -> &str;
}

/// Safe access to a UTF-8 byte extent in source text.
pub trait SourceSlicing {
    fn source_slice(&self, span: std::ops::Range<usize>) -> Option<&str>;
}
