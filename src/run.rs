//! A bare run and its pieces: the run split at its separators.

use crate::anatomy::{Integer, Separator};
use crate::glyph::{Classifying, Glyph};

/// A maximal run of plain and separator glyphs, at its byte offset in the text.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Run<'a> {
    /// The run's text.
    pub text: &'a str,
    /// The byte offset of its first glyph.
    pub start: Integer,
}

/// One piece of a run: the text between two separators, and the separator after it.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Piece<'a> {
    /// The piece's text: a symbol when non-empty.
    pub text: &'a str,
    /// The byte offset of its first glyph.
    pub start: Integer,
    /// The separator that follows it, if one does.
    pub separator: Option<Separator>,
}

/// The kind whose capability splits a run into its pieces.
pub(crate) trait Splitting<'a> {
    /// The pieces, in order; one more than the separators.
    fn pieces(&self) -> Vec<Piece<'a>>;
}

impl<'a> Splitting<'a> for Run<'a> {
    fn pieces(&self) -> Vec<Piece<'a>> {
        let mut pieces = Vec::new();
        let mut piece_start = 0;
        for (offset, glyph) in self.text.char_indices() {
            if let Glyph::Separate(separator) = glyph.classify() {
                pieces.push(Piece {
                    text: &self.text[piece_start..offset],
                    start: self.start + piece_start as Integer,
                    separator: Some(separator),
                });
                piece_start = offset + glyph.len_utf8();
            }
        }
        pieces.push(Piece {
            text: &self.text[piece_start..],
            start: self.start + piece_start as Integer,
            separator: None,
        });
        pieces
    }
}

/// The kind whose capability says whether a piece is a symbol.
pub(crate) trait Symbolic {
    /// Non-empty, with no whitespace, delimiter or separator glyph: guaranteed by the run for all but emptiness.
    fn is_symbol(&self) -> bool;
}

impl Symbolic for Piece<'_> {
    fn is_symbol(&self) -> bool {
        !self.text.is_empty()
    }
}
