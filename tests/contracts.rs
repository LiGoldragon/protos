use std::error::Error;
use std::fmt;
use std::str::FromStr;

use content_identity::{
    CapsuleNameTreeDomain, ContentHash, DomainSeparation, HashDomain, LayoutVersion, ShortCode,
};
use name_table::{IdentifierNamespace, Name, NameTable};
use protos::{
    Capsule, CapsuleKind, CapsuleVerificationError, ShortIdentifier, TextualCapsuleAssociation,
};
use structural_codec::EncodedForm;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct FixtureLanguage;

#[derive(Clone, Debug, Eq, PartialEq)]
struct FixtureEncoded(u32);

impl EncodedForm for FixtureEncoded {
    type Language = FixtureLanguage;
}

struct FixtureContentDomain;

impl HashDomain for FixtureContentDomain {
    fn separation() -> DomainSeparation {
        DomainSeparation::Contextual {
            context: "protos contract fixture content",
            layout: LayoutVersion::new(1),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct DerivationFailure(&'static str);

impl fmt::Display for DerivationFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.0)
    }
}

impl Error for DerivationFailure {}

#[derive(Debug, Eq, PartialEq)]
struct FixtureCapsule {
    encoded: FixtureEncoded,
    names: NameTable,
    short_identifier: ShortCode,
    content_pin: ContentHash<FixtureContentDomain>,
    nametree_pin: ContentHash<CapsuleNameTreeDomain>,
    fail_content_derivation: bool,
    fail_nametree_derivation: bool,
}

impl FixtureCapsule {
    fn sealed(value: u32, names: NameTable, short_identifier: ShortCode) -> Self {
        let encoded = FixtureEncoded(value);
        let content_pin = Self::derive_content(&encoded);
        let nametree_pin = Self::derive_nametree(&names).expect("fixture nametree identity");
        Self {
            encoded,
            names,
            short_identifier,
            content_pin,
            nametree_pin,
            fail_content_derivation: false,
            fail_nametree_derivation: false,
        }
    }

    fn derive_content(encoded: &FixtureEncoded) -> ContentHash<FixtureContentDomain> {
        ContentHash::derive(&encoded.0.to_le_bytes())
    }

    fn derive_nametree(
        names: &NameTable,
    ) -> Result<ContentHash<CapsuleNameTreeDomain>, DerivationFailure> {
        let slice_identity = names
            .identity()
            .map_err(|_| DerivationFailure("fixture nametree identity"))?;
        Ok(ContentHash::derive(slice_identity.bytes()))
    }
}

impl ShortIdentifier for FixtureCapsule {
    fn short_identifier(&self) -> ShortCode {
        self.short_identifier
    }
}

impl Capsule for FixtureCapsule {
    const KIND: CapsuleKind = CapsuleKind::Logos;

    type EncodedForm = FixtureEncoded;
    type ContentDomain = FixtureContentDomain;
    type NameTree = NameTable;
    type ContentIdentityError = DerivationFailure;
    type NameTreeIdentityError = DerivationFailure;

    fn encoded_form(&self) -> &Self::EncodedForm {
        &self.encoded
    }

    fn nametree(&self) -> &Self::NameTree {
        &self.names
    }

    fn content_identity_pin(&self) -> ContentHash<Self::ContentDomain> {
        self.content_pin
    }

    fn nametree_identity_pin(&self) -> ContentHash<CapsuleNameTreeDomain> {
        self.nametree_pin
    }

    fn rederive_content_identity(
        &self,
    ) -> Result<ContentHash<Self::ContentDomain>, Self::ContentIdentityError> {
        if self.fail_content_derivation {
            return Err(DerivationFailure("content derivation"));
        }
        Ok(Self::derive_content(&self.encoded))
    }

    fn rederive_nametree_identity(
        &self,
    ) -> Result<ContentHash<CapsuleNameTreeDomain>, Self::NameTreeIdentityError> {
        if self.fail_nametree_derivation {
            return Err(DerivationFailure("nametree derivation"));
        }
        Self::derive_nametree(&self.names)
    }
}

fn code() -> ShortCode {
    ShortCode::from_value(42).expect("canonical short code")
}

fn capsule() -> FixtureCapsule {
    FixtureCapsule::sealed(42, NameTable::new(IdentifierNamespace::Logos), code())
}

#[test]
fn capsule_kind_is_exactly_the_three_component_roles() {
    assert_eq!(
        CapsuleKind::ALL,
        [CapsuleKind::Schema, CapsuleKind::Logos, CapsuleKind::Nomos,]
    );
    for kind in CapsuleKind::ALL {
        match kind {
            CapsuleKind::Schema | CapsuleKind::Logos | CapsuleKind::Nomos => {}
        }
    }
}

#[test]
fn short_identifier_exposes_the_canonical_content_identity_code() {
    let capsule = capsule();
    let exposed: ShortCode = capsule.short_identifier();
    assert_eq!(exposed, code());
    assert_eq!(exposed.value(), 42);
}

#[test]
fn pins_are_required_typed_values_and_verification_succeeds() {
    fn require_pins<C: Capsule>(capsule: &C) {
        let _: ContentHash<C::ContentDomain> = capsule.content_identity_pin();
        let _: ContentHash<CapsuleNameTreeDomain> = capsule.nametree_identity_pin();
    }

    let capsule = capsule();
    require_pins(&capsule);
    assert_eq!(capsule.encoded_form(), &FixtureEncoded(42));
    assert_eq!(
        capsule.nametree(),
        &NameTable::new(IdentifierNamespace::Logos)
    );
    capsule.verify().expect("valid required pins");
}

#[test]
fn verification_reports_both_derivation_failures() {
    let mut content_failure = capsule();
    content_failure.fail_content_derivation = true;
    assert!(matches!(
        content_failure.verify(),
        Err(CapsuleVerificationError::ContentIdentityDerivation(
            DerivationFailure("content derivation")
        ))
    ));

    let mut nametree_failure = capsule();
    nametree_failure.fail_nametree_derivation = true;
    assert!(matches!(
        nametree_failure.verify(),
        Err(CapsuleVerificationError::NameTreeIdentityDerivation(
            DerivationFailure("nametree derivation")
        ))
    ));
}

#[test]
fn verification_reports_both_mismatches_with_values() {
    let mut content_mismatch = capsule();
    let pinned_content = ContentHash::derive(b"different content");
    content_mismatch.content_pin = pinned_content;
    match content_mismatch.verify() {
        Err(CapsuleVerificationError::ContentMismatch { pinned, actual }) => {
            assert_eq!(pinned, pinned_content);
            assert_eq!(actual, FixtureCapsule::derive_content(&FixtureEncoded(42)));
        }
        other => panic!("expected content mismatch, got {other:?}"),
    }

    let mut nametree_mismatch = capsule();
    let pinned_nametree = ContentHash::derive(b"different nametree");
    nametree_mismatch.nametree_pin = pinned_nametree;
    match nametree_mismatch.verify() {
        Err(CapsuleVerificationError::NameTreeMismatch { pinned, actual }) => {
            assert_eq!(pinned, pinned_nametree);
            assert_eq!(
                actual,
                FixtureCapsule::derive_nametree(&NameTable::new(IdentifierNamespace::Logos))
                    .expect("fixture nametree identity")
            );
        }
        other => panic!("expected nametree mismatch, got {other:?}"),
    }
}

struct SourceRepresentation(String);

struct DocumentRepresentation(Vec<u8>);

struct SourceAssociation;
struct DocumentAssociation;

const FIXTURE_NAME: &str = "fixture_capsule";

#[derive(Debug, Eq, PartialEq)]
enum ProjectionViewError {
    MissingCanonicalName,
}

impl fmt::Display for ProjectionViewError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingCanonicalName => formatter.write_str("fixture canonical name is missing"),
        }
    }
}

impl Error for ProjectionViewError {}

#[derive(Debug, Eq, PartialEq)]
enum ProjectionUnviewError {
    Malformed(&'static str),
    Incompatible {
        field: &'static str,
        expected: &'static str,
        actual: String,
    },
}

impl fmt::Display for ProjectionUnviewError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Malformed(details) => write!(formatter, "malformed projection text: {details}"),
            Self::Incompatible {
                field,
                expected,
                actual,
            } => write!(
                formatter,
                "incompatible projection {field}: expected {expected}, got {actual}"
            ),
        }
    }
}

impl Error for ProjectionUnviewError {}

fn projection_name(capsule: &FixtureCapsule) -> Result<&str, ProjectionViewError> {
    capsule
        .nametree()
        .resolve(IdentifierNamespace::Logos.identifier(0))
        .map(|name| name.as_str())
        .map_err(|_| ProjectionViewError::MissingCanonicalName)
}

fn projection_capsule(value: u32, short_identifier: ShortCode) -> FixtureCapsule {
    let mut names = NameTable::new(IdentifierNamespace::Logos);
    names
        .intern(Name::new(FIXTURE_NAME))
        .expect("fixture canonical name");
    FixtureCapsule::sealed(value, names, short_identifier)
}

fn projection_capsule_from_fields(
    kind: &str,
    value: &str,
    short_identifier: &str,
    name: &str,
) -> Result<FixtureCapsule, ProjectionUnviewError> {
    if kind != "logos" {
        return Err(ProjectionUnviewError::Incompatible {
            field: "kind",
            expected: "logos",
            actual: kind.to_owned(),
        });
    }
    if name != FIXTURE_NAME {
        return Err(ProjectionUnviewError::Incompatible {
            field: "name",
            expected: FIXTURE_NAME,
            actual: name.to_owned(),
        });
    }

    let value = value
        .parse()
        .map_err(|_| ProjectionUnviewError::Malformed("value is not a u32"))?;
    let short_identifier = ShortCode::from_str(short_identifier)
        .map_err(|_| ProjectionUnviewError::Malformed("short identifier is not canonical"))?;
    Ok(projection_capsule(value, short_identifier))
}

fn parse_source(text: &str) -> Result<FixtureCapsule, ProjectionUnviewError> {
    let mut fields = text.split('|');
    if fields.next() != Some("protos-source:v1") {
        return Err(ProjectionUnviewError::Malformed("source header"));
    }

    let mut kind = None;
    let mut value = None;
    let mut short_identifier = None;
    let mut name = None;
    for field in fields {
        let (key, value_part) = field
            .split_once('=')
            .ok_or(ProjectionUnviewError::Malformed("source field"))?;
        let destination = match key {
            "kind" => &mut kind,
            "value" => &mut value,
            "short" => &mut short_identifier,
            "name" => &mut name,
            _ => return Err(ProjectionUnviewError::Malformed("unknown source field")),
        };
        if destination.replace(value_part).is_some() {
            return Err(ProjectionUnviewError::Malformed("duplicate source field"));
        }
    }

    projection_capsule_from_fields(
        kind.ok_or(ProjectionUnviewError::Malformed("missing source kind"))?,
        value.ok_or(ProjectionUnviewError::Malformed("missing source value"))?,
        short_identifier.ok_or(ProjectionUnviewError::Malformed("missing source short"))?,
        name.ok_or(ProjectionUnviewError::Malformed("missing source name"))?,
    )
}

fn parse_document(bytes: &[u8]) -> Result<FixtureCapsule, ProjectionUnviewError> {
    let text = std::str::from_utf8(bytes)
        .map_err(|_| ProjectionUnviewError::Malformed("document is not UTF-8"))?;
    let mut lines = text.lines();
    if lines.next() != Some("protos-document/v1") {
        return Err(ProjectionUnviewError::Malformed("document header"));
    }

    let mut fields = [None; 4];
    for line in lines {
        let (key, value) = line
            .split_once(": ")
            .ok_or(ProjectionUnviewError::Malformed("document field"))?;
        let slot = match key {
            "kind" => &mut fields[0],
            "value" => &mut fields[1],
            "short" => &mut fields[2],
            "name" => &mut fields[3],
            _ => return Err(ProjectionUnviewError::Malformed("unknown document field")),
        };
        if slot.replace(value).is_some() {
            return Err(ProjectionUnviewError::Malformed("duplicate document field"));
        }
    }

    projection_capsule_from_fields(
        fields[0].ok_or(ProjectionUnviewError::Malformed("missing document kind"))?,
        fields[1].ok_or(ProjectionUnviewError::Malformed("missing document value"))?,
        fields[2].ok_or(ProjectionUnviewError::Malformed("missing document short"))?,
        fields[3].ok_or(ProjectionUnviewError::Malformed("missing document name"))?,
    )
}

impl TextualCapsuleAssociation for SourceAssociation {
    type TextualRepresentation = SourceRepresentation;
    type Capsule = FixtureCapsule;
    type ViewError = ProjectionViewError;
    type UnviewError = ProjectionUnviewError;

    fn view_capsule(
        capsule: &Self::Capsule,
    ) -> Result<Self::TextualRepresentation, Self::ViewError> {
        Ok(SourceRepresentation(format!(
            "protos-source:v1|kind=logos|value={}|short={}|name={}",
            capsule.encoded_form().0,
            capsule.short_identifier(),
            projection_name(capsule)?,
        )))
    }

    fn unview_capsule(
        textual: &Self::TextualRepresentation,
    ) -> Result<Self::Capsule, Self::UnviewError> {
        parse_source(&textual.0)
    }
}

impl TextualCapsuleAssociation for DocumentAssociation {
    type TextualRepresentation = DocumentRepresentation;
    type Capsule = FixtureCapsule;
    type ViewError = ProjectionViewError;
    type UnviewError = ProjectionUnviewError;

    fn view_capsule(
        capsule: &Self::Capsule,
    ) -> Result<Self::TextualRepresentation, Self::ViewError> {
        Ok(DocumentRepresentation(
            format!(
                "protos-document/v1\nkind: logos\nvalue: {}\nshort: {}\nname: {}",
                capsule.encoded_form().0,
                capsule.short_identifier(),
                projection_name(capsule)?,
            )
            .into_bytes(),
        ))
    }

    fn unview_capsule(
        textual: &Self::TextualRepresentation,
    ) -> Result<Self::Capsule, Self::UnviewError> {
        parse_document(&textual.0)
    }
}

#[test]
fn two_projection_associations_round_trip_one_capsule_without_textual() {
    fn requires_fixed_capsule<Association>()
    where
        Association: TextualCapsuleAssociation<Capsule = FixtureCapsule>,
    {
    }

    requires_fixed_capsule::<SourceAssociation>();
    requires_fixed_capsule::<DocumentAssociation>();

    let original = projection_capsule(42, code());

    let source = SourceAssociation::view_capsule(&original).expect("source view");
    assert_eq!(
        source.0,
        "protos-source:v1|kind=logos|value=42|short=0016|name=fixture_capsule"
    );
    let source_round_trip = SourceAssociation::unview_capsule(&source).expect("source unview");
    source_round_trip.verify().expect("source Capsule");
    assert_eq!(source_round_trip, original);

    let document = DocumentAssociation::view_capsule(&original).expect("document view");
    assert_eq!(
        document.0,
        b"protos-document/v1\nkind: logos\nvalue: 42\nshort: 0016\nname: fixture_capsule"
    );
    let document_round_trip =
        DocumentAssociation::unview_capsule(&document).expect("document unview");
    document_round_trip.verify().expect("document Capsule");
    assert_eq!(document_round_trip, original);
}

#[test]
fn malformed_projection_text_returns_typed_errors() {
    let source_error =
        SourceAssociation::unview_capsule(&SourceRepresentation("not-a-source".to_owned()))
            .expect_err("malformed source must fail");
    assert_eq!(
        source_error,
        ProjectionUnviewError::Malformed("source header")
    );

    let document_error =
        DocumentAssociation::unview_capsule(&DocumentRepresentation(vec![0xff, 0xfe]))
            .expect_err("malformed document must fail");
    assert_eq!(
        document_error,
        ProjectionUnviewError::Malformed("document is not UTF-8")
    );
}

#[test]
fn incompatible_projection_text_returns_typed_errors() {
    let source_error = SourceAssociation::unview_capsule(&SourceRepresentation(
        "protos-source:v1|kind=schema|value=42|short=0016|name=fixture_capsule".to_owned(),
    ))
    .expect_err("wrong Capsule kind must fail");
    assert_eq!(
        source_error,
        ProjectionUnviewError::Incompatible {
            field: "kind",
            expected: "logos",
            actual: "schema".to_owned(),
        }
    );

    let document_error = DocumentAssociation::unview_capsule(&DocumentRepresentation(
        b"protos-document/v1\nkind: logos\nvalue: 42\nshort: 0016\nname: other_capsule".to_vec(),
    ))
    .expect_err("wrong canonical name must fail");
    assert_eq!(
        document_error,
        ProjectionUnviewError::Incompatible {
            field: "name",
            expected: FIXTURE_NAME,
            actual: "other_capsule".to_owned(),
        }
    );
}
