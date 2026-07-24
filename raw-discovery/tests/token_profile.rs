//! Durable witnesses for sealed, language-supplied lexical data.

use content_identity::PortableArchive;
use raw_discovery::{
    BareTokenPolicy, Block, CarrierBody, CarrierCapture, CarrierIdentity, CarrierRule,
    Delimiter, DelimiterToken, GluedApplicationToken, GlyphClass, GlyphClassSet,
    ProfileRevision, PunctuationToken, Recognizer, TokenBoundary, TokenProfile,
    TokenProfileError, TokenProfileSpec, TriviaRule,
};

fn classes(classes: Vec<GlyphClass>) -> GlyphClassSet {
    GlyphClassSet::new(classes)
}

fn data_profile(revision: u32) -> TokenProfile {
    TokenProfile::seal(TokenProfileSpec {
        revision: ProfileRevision::new(revision),
        delimiters: vec![
            DelimiterToken {
                delimiter: Delimiter::Parenthesis,
                opening: "<(".to_owned(),
                closing: ")>".to_owned(),
            },
            DelimiterToken {
                delimiter: Delimiter::SquareBracket,
                opening: "<[".to_owned(),
                closing: "]>".to_owned(),
            },
            DelimiterToken {
                delimiter: Delimiter::Brace,
                opening: "<{".to_owned(),
                closing: "}>".to_owned(),
            },
        ],
        application: GluedApplicationToken {
            text: "::".to_owned(),
        },
        punctuation: vec![
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
        ],
        trivia: vec![
            TriviaRule::Whitespace,
            TriviaRule::LineComment {
                opening: "//".to_owned(),
            },
            TriviaRule::BlockComment {
                opening: "/*".to_owned(),
                closing: "*/".to_owned(),
                nested: true,
            },
        ],
        carriers: vec![
            CarrierRule {
                identity: CarrierIdentity::new(10),
                prefix: "\"".to_owned(),
                body: CarrierBody::Delimited {
                    closing: "\"".to_owned(),
                    escape: Some("\\".to_owned()),
                },
                capture: CarrierCapture::WholeToken,
            },
            CarrierRule {
                identity: CarrierIdentity::new(11),
                prefix: "b\"".to_owned(),
                body: CarrierBody::Delimited {
                    closing: "\"".to_owned(),
                    escape: Some("\\".to_owned()),
                },
                capture: CarrierCapture::WholeToken,
            },
            CarrierRule {
                identity: CarrierIdentity::new(12),
                prefix: "'".to_owned(),
                body: CarrierBody::DelimitedOrClassRun {
                    closing: "'".to_owned(),
                    escape: Some("\\".to_owned()),
                    first: classes(vec![GlyphClass::AsciiAlphabetic]),
                    continuation: classes(vec![
                        GlyphClass::AsciiAlphanumeric,
                        GlyphClass::Exact("_".to_owned()),
                    ]),
                },
                capture: CarrierCapture::WholeToken,
            },
            CarrierRule {
                identity: CarrierIdentity::new(13),
                prefix: "q".to_owned(),
                body: CarrierBody::Fenced {
                    fence: '#',
                    opening: "\"".to_owned(),
                    closing: "\"".to_owned(),
                    minimum_fences: 0,
                    maximum_fences: 8,
                },
                capture: CarrierCapture::WholeToken,
            },
            CarrierRule {
                identity: CarrierIdentity::new(14),
                prefix: "id#".to_owned(),
                body: CarrierBody::ClassRun {
                    first: classes(vec![
                        GlyphClass::AsciiAlphabetic,
                        GlyphClass::Exact("_".to_owned()),
                    ]),
                    continuation: classes(vec![
                        GlyphClass::AsciiAlphanumeric,
                        GlyphClass::Exact("_".to_owned()),
                    ]),
                },
                capture: CarrierCapture::WholeToken,
            },
            CarrierRule {
                identity: CarrierIdentity::new(15),
                prefix: String::new(),
                body: CarrierBody::ClassRun {
                    first: classes(vec![GlyphClass::AsciiDigit]),
                    continuation: classes(vec![
                        GlyphClass::AsciiAlphanumeric,
                        GlyphClass::Exact("._+-".to_owned()),
                    ]),
                },
                capture: CarrierCapture::WholeToken,
            },
        ],
        bare_tokens: BareTokenPolicy::Classed(vec![TokenBoundary {
            first: classes(vec![
                GlyphClass::AsciiAlphabetic,
                GlyphClass::Exact("_".to_owned()),
            ]),
            continuation: classes(vec![
                GlyphClass::AsciiAlphanumeric,
                GlyphClass::Exact("_-".to_owned()),
            ]),
        }]),
    })
    .expect("generic data profile seals")
}

#[test]
fn profile_data_drives_multichar_structure_punctuation_and_trivia() {
    let profile = data_profile(1);
    let document = Recognizer::with_token_profile(profile)
        .recognize("<(alpha::beta, /* nested /* ok */ done */ gamma)> // tail")
        .expect("profile-driven recognition");
    let root = document.root_object_at(0).expect("one root");
    let children = root
        .as_delimited(Delimiter::Parenthesis)
        .expect("configured parenthesis");
    assert!(children[0].is_application(), "configured :: is application");
    assert_eq!(children[1].demote_to_string(), Some(","));
    assert_eq!(children[2].demote_to_string(), Some("gamma"));
}

#[test]
fn generic_carrier_automata_cover_quoted_prefixed_fenced_dual_and_numeric_tokens() {
    let profile = data_profile(1);
    for source in [
        "\"string body\"",
        "b\"byte body\"",
        "'x'",
        "'lifetime",
        "q##\"raw body\"##",
        "id#raw_name",
        "12.5e-2",
    ] {
        let document = Recognizer::with_token_profile(profile.clone())
            .recognize(source)
            .unwrap_or_else(|error| panic!("{source}: {error}"));
        assert_eq!(
            document.root_object_at(0).and_then(Block::demote_to_string),
            Some(source),
            "{source}"
        );
    }
}

#[test]
fn profile_identity_records_versioned_disagreement_and_round_trips_its_payload() {
    let first = data_profile(1);
    let second = data_profile(2);
    assert_ne!(first.identity(), second.identity());

    let bytes = first.spec().to_archive_bytes().expect("archive profile data");
    let restored =
        TokenProfileSpec::from_archive_bytes(&bytes).expect("validate and restore profile data");
    let resealed = TokenProfile::seal(restored).expect("reseal restored profile");
    assert_eq!(first.identity(), resealed.identity());
}

#[test]
fn an_ambiguous_profile_cannot_be_constructed() {
    let mut spec = data_profile(1).spec().clone();
    spec.punctuation.push(PunctuationToken {
        text: "::".to_owned(),
        attach_left: true,
        attach_right: true,
    });
    let error = TokenProfile::seal(spec).expect_err("application/punctuation collision");
    assert!(matches!(
        error,
        TokenProfileError::AmbiguousTrigger { token, .. } if token == "::"
    ));
}

#[test]
fn existing_protos_syntax_remains_the_standard_profile_contract() {
    let source = "Public.Newtype.( CommitSequence [ rkyv.Archive Clone ] Integer )";
    let document = Recognizer::standard()
        .recognize(source)
        .expect("ruled Protos syntax remains accepted");
    assert!(document.root_object_at(0).is_some_and(Block::is_application));
}
