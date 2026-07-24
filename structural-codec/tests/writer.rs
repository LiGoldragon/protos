//! Canonical rendering witnesses for raw blocks emitted by the structural codec.

use raw_discovery::{
    Atom, Block, Delimiter, PipeText, PunctuationToken, Recognizer, TokenProfile,
};
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

#[test]
fn sealed_profile_drives_canonical_delimiters_application_and_attachment() {
    let mut spec = TokenProfile::standard().spec().clone();
    let parenthesis = spec
        .delimiters
        .iter_mut()
        .find(|tokens| tokens.delimiter == Delimiter::Parenthesis)
        .expect("parenthesis profile");
    parenthesis.opening = "<(".to_owned();
    parenthesis.closing = ")>".to_owned();
    spec.application.text = "::".to_owned();
    spec.punctuation = vec![
        PunctuationToken {
            text: "=>".to_owned(),
            attach_left: true,
            attach_right: true,
        },
        PunctuationToken {
            text: ",".to_owned(),
            attach_left: true,
            attach_right: false,
        },
    ];
    let profile = TokenProfile::seal(spec).expect("custom profile seals");
    let block = Block::Delimited {
        delimiter: Delimiter::Parenthesis,
        root_objects: vec![
            Block::Atom(Atom::new("alpha")),
            Block::Atom(Atom::new("=>")),
            Block::Application {
                head: Box::new(Block::Atom(Atom::new("beta"))),
                payload: Box::new(Block::Atom(Atom::new("value"))),
            },
            Block::Atom(Atom::new(",")),
            Block::Atom(Atom::new("gamma")),
        ],
    };

    let rendered = block
        .canonical_text_with(&profile)
        .expect("profile-driven emission");
    assert_eq!(rendered, "<(alpha=>beta::value, gamma)>");
    let reparsed = Recognizer::with_token_profile(profile)
        .recognize(&rendered)
        .expect("profile-driven recognition");
    assert_eq!(reparsed.root_object_at(0), Some(&block));
}
