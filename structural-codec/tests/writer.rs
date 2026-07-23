//! Canonical rendering witnesses for raw blocks emitted by the structural codec.

use raw_discovery::{Block, PipeText, Recognizer};
use structural_codec::CanonicalText;

/// The canonical writer escapes both a pipe-close marker and a backslash, so the
/// recognizer reconstructs the literal carrier contents exactly.
#[test]
fn canonical_pipe_text_round_trips_escaped_close_and_backslash() {
    let original = Block::PipeText(PipeText::new(r"literal |) and \ remain text"));
    let rendered = original.canonical_text();
    assert_eq!(rendered, r"(|literal \|) and \\ remain text|)");

    let reparsed = Recognizer::standard()
        .recognize(&rendered)
        .expect("recognize canonical pipe text")
        .root_object_at(0)
        .expect("one root")
        .clone();
    assert_eq!(reparsed, original);
}
