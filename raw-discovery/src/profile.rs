//! Sealed, versioned lexical profiles executed by the shared recognizer.
//!
//! A profile is textual-language data. It names delimiter spellings, the glued
//! application token, punctuation attachment, trivia, token boundaries, and
//! opaque carrier shapes. The recognizer owns the one execution mechanism; a
//! language supplies only a sealed [`TokenProfileSpec`].

use content_identity::{ContentHash, DomainSeparation, HashDomain, LayoutVersion};

use crate::Delimiter;

/// A monotonic revision carried in a lexical profile's addressed payload.
#[derive(
    rkyv::Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
    Clone,
    Copy,
    Debug,
    Eq,
    PartialEq,
    Ord,
    PartialOrd,
)]
pub struct ProfileRevision(u32);

impl ProfileRevision {
    pub const fn new(revision: u32) -> Self {
        Self(revision)
    }

    pub const fn value(self) -> u32 {
        self.0
    }
}

/// Compatibility names for the two Protos-family profiles that existed before
/// lexical profiles became fully data-driven.
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Copy, Debug, Eq, PartialEq)]
pub enum GlyphSet {
    Standard,
    NomosExtended,
}

impl GlyphSet {
    pub const fn admits_dollar_sigil(self) -> bool {
        matches!(self, Self::NomosExtended)
    }
}

/// The compatibility profile selector. New languages should construct and seal
/// a [`TokenProfileSpec`] directly.
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Copy, Debug, Eq, PartialEq)]
pub struct RawProfile {
    revision: ProfileRevision,
    glyphs: GlyphSet,
}

impl RawProfile {
    pub const fn new(revision: ProfileRevision, glyphs: GlyphSet) -> Self {
        Self { revision, glyphs }
    }

    pub const fn standard() -> Self {
        Self::new(ProfileRevision::new(1), GlyphSet::Standard)
    }

    pub const fn nomos_extended() -> Self {
        Self::new(ProfileRevision::new(1), GlyphSet::NomosExtended)
    }

    pub const fn revision(self) -> ProfileRevision {
        self.revision
    }

    pub const fn glyphs(self) -> GlyphSet {
        self.glyphs
    }

    /// Materialize and seal the compatibility profile as ordinary lexical data.
    pub fn token_profile(self) -> TokenProfile {
        let forbidden = match self.glyphs {
            GlyphSet::Standard => "$",
            GlyphSet::NomosExtended => "",
        };
        TokenProfile::seal(TokenProfileSpec {
            revision: self.revision,
            delimiters: Delimiter::ALL
                .into_iter()
                .map(|delimiter| DelimiterToken {
                    delimiter,
                    opening: delimiter.opening_text().to_owned(),
                    closing: delimiter.closing_text().to_owned(),
                })
                .collect(),
            application: GluedApplicationToken {
                text: ".".to_owned(),
            },
            punctuation: Vec::new(),
            trivia: vec![
                TriviaRule::Whitespace,
                TriviaRule::LineComment {
                    opening: ";;".to_owned(),
                },
            ],
            carriers: vec![CarrierRule {
                identity: CarrierIdentity::new(0),
                prefix: "(|".to_owned(),
                body: CarrierBody::Delimited {
                    closing: "|)".to_owned(),
                    escape: Some("\\".to_owned()),
                },
                capture: CarrierCapture::Content,
            }],
            bare_tokens: BareTokenPolicy::Unreserved {
                forbidden_glyphs: forbidden.to_owned(),
            },
        })
        .expect("the built-in Protos lexical profile is statically valid")
    }
}

/// One delimiter kind's textual tokens.
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, Eq, PartialEq)]
pub struct DelimiterToken {
    pub delimiter: Delimiter,
    pub opening: String,
    pub closing: String,
}

/// The punctuation token that constructs a right-associative raw application.
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, Eq, PartialEq)]
pub struct GluedApplicationToken {
    pub text: String,
}

/// Canonical attachment policy for a punctuation token.
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, Eq, PartialEq)]
pub struct PunctuationToken {
    pub text: String,
    pub attach_left: bool,
    pub attach_right: bool,
}

/// Trivia is discarded structurally according to sealed language data.
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, Eq, PartialEq)]
pub enum TriviaRule {
    Whitespace,
    LineComment {
        opening: String,
    },
    BlockComment {
        opening: String,
        closing: String,
        nested: bool,
    },
}

/// A generic character class used only to delimit opaque lexical tokens.
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, Eq, PartialEq)]
pub enum GlyphClass {
    AsciiAlphabetic,
    AsciiDigit,
    AsciiAlphanumeric,
    UnicodeAlphabetic,
    UnicodeNumeric,
    UnicodeAlphanumeric,
    Exact(String),
}

impl GlyphClass {
    pub(crate) fn contains(&self, character: char) -> bool {
        match self {
            Self::AsciiAlphabetic => character.is_ascii_alphabetic(),
            Self::AsciiDigit => character.is_ascii_digit(),
            Self::AsciiAlphanumeric => character.is_ascii_alphanumeric(),
            Self::UnicodeAlphabetic => character.is_alphabetic(),
            Self::UnicodeNumeric => character.is_numeric(),
            Self::UnicodeAlphanumeric => character.is_alphanumeric(),
            Self::Exact(characters) => characters.contains(character),
        }
    }

    fn overlaps(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Exact(left), Self::Exact(right)) => left.chars().any(|c| right.contains(c)),
            (Self::Exact(exact), class) | (class, Self::Exact(exact)) => {
                exact.chars().any(|character| class.contains(character))
            }
            (Self::AsciiAlphabetic, Self::AsciiDigit)
            | (Self::AsciiDigit, Self::AsciiAlphabetic)
            | (Self::UnicodeAlphabetic, Self::UnicodeNumeric)
            | (Self::UnicodeNumeric, Self::UnicodeAlphabetic) => false,
            _ => true,
        }
    }
}

/// A union of character classes.
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, Eq, PartialEq)]
pub struct GlyphClassSet(pub Vec<GlyphClass>);

impl GlyphClassSet {
    pub fn new(classes: Vec<GlyphClass>) -> Self {
        Self(classes)
    }

    pub(crate) fn contains(&self, character: char) -> bool {
        self.0.iter().any(|class| class.contains(character))
    }

    fn overlaps(&self, other: &Self) -> bool {
        self.0
            .iter()
            .any(|left| other.0.iter().any(|right| left.overlaps(right)))
    }
}

/// One bare-token boundary alternative. Several alternatives can distinguish
/// name-shaped and numeric-shaped tokens without assigning either semantic
/// meaning at the raw layer.
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, Eq, PartialEq)]
pub struct TokenBoundary {
    pub first: GlyphClassSet,
    pub continuation: GlyphClassSet,
}

/// How non-structural, non-carrier tokens are delimited.
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, Eq, PartialEq)]
pub enum BareTokenPolicy {
    /// Compatibility mode: accept any non-structural run except named glyphs.
    Unreserved { forbidden_glyphs: String },
    /// Accept a run only when one boundary alternative covers it completely.
    Classed(Vec<TokenBoundary>),
}

/// Stable, profile-local identity of one opaque carrier rule.
#[derive(
    rkyv::Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
    Clone,
    Copy,
    Debug,
    Eq,
    Hash,
    Ord,
    PartialEq,
    PartialOrd,
)]
pub struct CarrierIdentity(u32);

impl CarrierIdentity {
    pub const fn new(value: u32) -> Self {
        Self(value)
    }

    pub const fn value(self) -> u32 {
        self.0
    }
}

/// Whether raw recognition preserves the whole token or exposes decoded content
/// through the legacy literal-text carrier.
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Copy, Debug, Eq, PartialEq)]
pub enum CarrierCapture {
    WholeToken,
    Content,
}

/// Generic carrier automata. These mechanisms cover quoted tokens, prefixed
/// name-like tokens, fence-counted raw tokens, numeric/class runs, and the
/// quote-or-prefix ambiguity used by language families with both character and
/// lifetime-like tokens. They contain no target-language vocabulary.
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, Eq, PartialEq)]
pub enum CarrierBody {
    Delimited {
        closing: String,
        escape: Option<String>,
    },
    Fenced {
        fence: char,
        opening: String,
        closing: String,
        minimum_fences: u8,
        maximum_fences: u8,
    },
    ClassRun {
        first: GlyphClassSet,
        continuation: GlyphClassSet,
    },
    DelimitedOrClassRun {
        closing: String,
        escape: Option<String>,
        first: GlyphClassSet,
        continuation: GlyphClassSet,
    },
}

/// One opaque lexical carrier rule.
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, Eq, PartialEq)]
pub struct CarrierRule {
    pub identity: CarrierIdentity,
    /// Fixed text before the carrier body. It may be empty only for `ClassRun`.
    pub prefix: String,
    pub body: CarrierBody,
    pub capture: CarrierCapture,
}

/// The complete addressed pre-image of a token profile.
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, Eq, PartialEq)]
pub struct TokenProfileSpec {
    pub revision: ProfileRevision,
    pub delimiters: Vec<DelimiterToken>,
    pub application: GluedApplicationToken,
    pub punctuation: Vec<PunctuationToken>,
    pub trivia: Vec<TriviaRule>,
    pub carriers: Vec<CarrierRule>,
    pub bare_tokens: BareTokenPolicy,
}

/// Domain-separated identity of complete lexical-profile data.
pub struct TokenProfileDomain;

impl HashDomain for TokenProfileDomain {
    fn separation() -> DomainSeparation {
        DomainSeparation::Contextual {
            context: "raw-discovery 2026 sealed token profile",
            layout: LayoutVersion::new(1),
        }
    }
}

/// Portable profile identity used by structural tables.
#[derive(
    rkyv::Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
    Clone,
    Copy,
    Debug,
    Eq,
    Hash,
    Ord,
    PartialEq,
    PartialOrd,
)]
pub struct TokenProfileIdentity(pub [u8; 32]);

/// A seal refusal. No recognizer can be constructed from an ambiguous profile.
#[derive(
    rkyv::Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
    Clone,
    Debug,
    Eq,
    PartialEq,
    thiserror::Error,
)]
pub enum TokenProfileError {
    #[error("lexical token {field} must not be empty")]
    EmptyToken { field: String },
    #[error("delimiter kind {delimiter:?} was missing or repeated")]
    DelimiterCardinality { delimiter: Delimiter },
    #[error("carrier identity {identity:?} was repeated")]
    RepeatedCarrierIdentity { identity: CarrierIdentity },
    #[error("lexical trigger {token:?} is assigned to both {first} and {second}")]
    AmbiguousTrigger {
        token: String,
        first: String,
        second: String,
    },
    #[error("class-run carriers {first:?} and {second:?} overlap at their first glyph")]
    AmbiguousClassRun {
        first: CarrierIdentity,
        second: CarrierIdentity,
    },
    #[error("content capture requires a delimited carrier and may occur only once")]
    InvalidContentCarrier,
    #[error("fenced carrier {identity:?} has an invalid fence range")]
    InvalidFenceRange { identity: CarrierIdentity },
    #[error("classed bare-token policy requires at least one nonempty boundary")]
    InvalidBareTokenBoundary,
    #[error("portable token-profile identity failed: {message}")]
    Identity { message: String },
}

/// A validated lexical profile with its identity stored outside the pre-image.
#[derive(Clone, Debug)]
pub struct TokenProfile {
    spec: TokenProfileSpec,
    identity: TokenProfileIdentity,
}

impl TokenProfile {
    pub fn seal(spec: TokenProfileSpec) -> Result<Self, TokenProfileError> {
        TokenProfileValidator::new(&spec).validate()?;
        let hash = ContentHash::<TokenProfileDomain>::of_core(&spec)
            .map_err(|error| TokenProfileError::Identity {
                message: error.to_string(),
            })?;
        Ok(Self {
            spec,
            identity: TokenProfileIdentity(*hash.bytes()),
        })
    }

    pub fn standard() -> Self {
        RawProfile::standard().token_profile()
    }

    pub fn nomos_extended() -> Self {
        RawProfile::nomos_extended().token_profile()
    }

    pub const fn identity(&self) -> TokenProfileIdentity {
        self.identity
    }

    pub const fn revision(&self) -> ProfileRevision {
        self.spec.revision
    }

    pub fn spec(&self) -> &TokenProfileSpec {
        &self.spec
    }

    pub fn delimiter(&self, delimiter: Delimiter) -> &DelimiterToken {
        self.spec
            .delimiters
            .iter()
            .find(|tokens| tokens.delimiter == delimiter)
            .expect("seal proves one token pair per delimiter")
    }

    pub fn punctuation(&self, text: &str) -> Option<&PunctuationToken> {
        self.spec
            .punctuation
            .iter()
            .find(|punctuation| punctuation.text == text)
    }

    pub fn content_carrier(&self) -> Option<&CarrierRule> {
        self.spec
            .carriers
            .iter()
            .find(|carrier| carrier.capture == CarrierCapture::Content)
    }
}

struct TokenProfileValidator<'spec> {
    spec: &'spec TokenProfileSpec,
}

impl<'spec> TokenProfileValidator<'spec> {
    fn new(spec: &'spec TokenProfileSpec) -> Self {
        Self { spec }
    }

    fn validate(&self) -> Result<(), TokenProfileError> {
        self.require_nonempty("application", &self.spec.application.text)?;
        for delimiter in Delimiter::ALL {
            if self
                .spec
                .delimiters
                .iter()
                .filter(|tokens| tokens.delimiter == delimiter)
                .count()
                != 1
            {
                return Err(TokenProfileError::DelimiterCardinality { delimiter });
            }
        }

        let mut triggers: Vec<(String, String)> = Vec::new();
        for delimiter in &self.spec.delimiters {
            self.add_trigger(
                &mut triggers,
                format!("{:?} opening", delimiter.delimiter),
                &delimiter.opening,
            )?;
            self.add_trigger(
                &mut triggers,
                format!("{:?} closing", delimiter.delimiter),
                &delimiter.closing,
            )?;
        }
        self.add_trigger(
            &mut triggers,
            "application".to_owned(),
            &self.spec.application.text,
        )?;
        for punctuation in &self.spec.punctuation {
            self.add_trigger(
                &mut triggers,
                "punctuation".to_owned(),
                &punctuation.text,
            )?;
        }
        for trivia in &self.spec.trivia {
            match trivia {
                TriviaRule::Whitespace => {}
                TriviaRule::LineComment { opening } => {
                    self.add_trigger(&mut triggers, "line comment".to_owned(), opening)?;
                }
                TriviaRule::BlockComment {
                    opening, closing, ..
                } => {
                    self.add_trigger(&mut triggers, "block comment".to_owned(), opening)?;
                    self.require_nonempty("block-comment closing", closing)?;
                }
            }
        }

        let mut identities = Vec::new();
        let mut empty_class_runs: Vec<&CarrierRule> = Vec::new();
        let mut content_carriers = 0usize;
        for carrier in &self.spec.carriers {
            if identities.contains(&carrier.identity) {
                return Err(TokenProfileError::RepeatedCarrierIdentity {
                    identity: carrier.identity,
                });
            }
            identities.push(carrier.identity);
            if carrier.capture == CarrierCapture::Content {
                content_carriers += 1;
                if !matches!(carrier.body, CarrierBody::Delimited { .. }) {
                    return Err(TokenProfileError::InvalidContentCarrier);
                }
            }
            match &carrier.body {
                CarrierBody::Delimited { closing, escape } => {
                    self.require_nonempty("carrier prefix", &carrier.prefix)?;
                    self.require_nonempty("carrier closing", closing)?;
                    if escape.as_ref().is_some_and(String::is_empty) {
                        return Err(TokenProfileError::EmptyToken {
                            field: "carrier escape".to_owned(),
                        });
                    }
                }
                CarrierBody::Fenced {
                    opening,
                    closing,
                    minimum_fences,
                    maximum_fences,
                    ..
                } => {
                    self.require_nonempty("fenced carrier prefix", &carrier.prefix)?;
                    self.require_nonempty("fenced carrier opening", opening)?;
                    self.require_nonempty("fenced carrier closing", closing)?;
                    if minimum_fences > maximum_fences {
                        return Err(TokenProfileError::InvalidFenceRange {
                            identity: carrier.identity,
                        });
                    }
                }
                CarrierBody::ClassRun {
                    first,
                    continuation,
                } => {
                    self.validate_classes(first, continuation)?;
                    if carrier.prefix.is_empty() {
                        empty_class_runs.push(carrier);
                    }
                }
                CarrierBody::DelimitedOrClassRun {
                    closing,
                    escape,
                    first,
                    continuation,
                } => {
                    self.require_nonempty("dual carrier prefix", &carrier.prefix)?;
                    self.require_nonempty("dual carrier closing", closing)?;
                    if escape.as_ref().is_some_and(String::is_empty) {
                        return Err(TokenProfileError::EmptyToken {
                            field: "dual carrier escape".to_owned(),
                        });
                    }
                    self.validate_classes(first, continuation)?;
                }
            }
            if !carrier.prefix.is_empty() {
                self.add_trigger(
                    &mut triggers,
                    format!("carrier {:?}", carrier.identity),
                    &carrier.prefix,
                )?;
            }
        }
        if content_carriers > 1 {
            return Err(TokenProfileError::InvalidContentCarrier);
        }
        for (index, first) in empty_class_runs.iter().enumerate() {
            let CarrierBody::ClassRun {
                first: first_set, ..
            } = &first.body
            else {
                unreachable!()
            };
            for second in empty_class_runs.iter().skip(index + 1) {
                let CarrierBody::ClassRun {
                    first: second_set, ..
                } = &second.body
                else {
                    unreachable!()
                };
                if first_set.overlaps(second_set) {
                    return Err(TokenProfileError::AmbiguousClassRun {
                        first: first.identity,
                        second: second.identity,
                    });
                }
            }
        }
        if let BareTokenPolicy::Classed(boundaries) = &self.spec.bare_tokens {
            if boundaries.is_empty()
                || boundaries
                    .iter()
                    .any(|boundary| boundary.first.0.is_empty() || boundary.continuation.0.is_empty())
            {
                return Err(TokenProfileError::InvalidBareTokenBoundary);
            }
        }
        Ok(())
    }

    fn validate_classes(
        &self,
        first: &GlyphClassSet,
        continuation: &GlyphClassSet,
    ) -> Result<(), TokenProfileError> {
        if first.0.is_empty() || continuation.0.is_empty() {
            return Err(TokenProfileError::InvalidBareTokenBoundary);
        }
        Ok(())
    }

    fn require_nonempty(&self, field: &str, token: &str) -> Result<(), TokenProfileError> {
        if token.is_empty() {
            return Err(TokenProfileError::EmptyToken {
                field: field.to_owned(),
            });
        }
        Ok(())
    }

    fn add_trigger(
        &self,
        triggers: &mut Vec<(String, String)>,
        owner: String,
        token: &str,
    ) -> Result<(), TokenProfileError> {
        self.require_nonempty(&owner, token)?;
        if let Some((_, first)) = triggers.iter().find(|(existing, _)| existing == token) {
            return Err(TokenProfileError::AmbiguousTrigger {
                token: token.to_owned(),
                first: first.clone(),
                second: owner,
            });
        }
        triggers.push((token.to_owned(), owner));
        Ok(())
    }
}
