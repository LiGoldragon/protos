//! The TextualForm ↔ EncodedForm boundary executes one sealed lexical profile
//! through the shared recognizer, evaluator, and canonical emitter.

use name_table::{IdentifierNamespace, Name, NameTable};
use raw_discovery::{RecognizeError, TokenProfile};
use structural_codec::fixture::{COMMIT_SEQUENCE, FIELD, FixtureBuilder};
use structural_codec::{
    AddressedStructuralTable, DecodeError, EncodeError, ScopedEncodedTypeId,
    SingleChunkRequired, StructuralValue, Textual, TextualForm, TextualProfileError,
};

struct FixtureLanguage;

struct FixtureTextual {
    table: AddressedStructuralTable,
    profile: TokenProfile,
}

#[derive(Debug, thiserror::Error)]
enum FixtureError {
    #[error("the textual view had no root object")]
    MissingRoot,
    #[error(transparent)]
    Recognize(#[from] RecognizeError),
    #[error(transparent)]
    Decode(#[from] DecodeError),
    #[error(transparent)]
    Encode(#[from] EncodeError),
    #[error(transparent)]
    Chunk(#[from] SingleChunkRequired),
    #[error(transparent)]
    Profile(#[from] TextualProfileError),
}

impl Textual for FixtureTextual {
    type Encoded = StructuralValue;
    type Language = FixtureLanguage;
    type Error = FixtureError;

    fn structuretree(&self) -> &AddressedStructuralTable {
        &self.table
    }

    fn token_profile(&self) -> TokenProfile {
        self.profile.clone()
    }

    fn missing_root_object(&self) -> Self::Error {
        FixtureError::MissingRoot
    }

    fn reify(
        &self,
        _expected: ScopedEncodedTypeId,
        mirror: &StructuralValue,
        _names: &mut NameTable,
    ) -> Result<Self::Encoded, Self::Error> {
        Ok(mirror.clone())
    }

    fn reflect(
        &self,
        _expected: ScopedEncodedTypeId,
        encoded: &Self::Encoded,
        _names: &mut NameTable,
    ) -> Result<StructuralValue, Self::Error> {
        Ok(encoded.clone())
    }
}

fn custom_profile(revision: u32) -> TokenProfile {
    let mut spec = TokenProfile::standard().spec().clone();
    spec.revision = raw_discovery::ProfileRevision::new(revision);
    spec.application.text = "::".to_owned();
    let brace = spec
        .delimiters
        .iter_mut()
        .find(|tokens| tokens.delimiter == raw_discovery::Delimiter::Brace)
        .expect("brace tokens");
    brace.opening = "<{".to_owned();
    brace.closing = "}>".to_owned();
    TokenProfile::seal(spec).expect("custom profile")
}

#[test]
fn one_profile_drives_recognition_evaluation_and_canonical_emission() {
    let profile = custom_profile(7);
    let textual = FixtureTextual {
        table: FixtureBuilder::new()
            .with_token_profile(&profile)
            .build()
            .expect("table"),
        profile,
    };
    let source = "CommitSequence::<{ Integer }>";
    let mut names = NameTable::new(IdentifierNamespace::Fixture);
    let encoded = textual
        .unview(
            COMMIT_SEQUENCE,
            &TextualForm::single(source.to_owned()),
            &mut names,
        )
        .expect("profile-driven unview");
    let rendered = textual
        .view(COMMIT_SEQUENCE, &encoded, &mut names)
        .expect("profile-driven view");
    assert_eq!(rendered.sole_text().unwrap(), "CommitSequence::<{Integer}>");
}

#[test]
fn recognition_failure_leaves_the_nametree_byte_identical() {
    let profile = TokenProfile::standard();
    let textual = FixtureTextual {
        table: FixtureBuilder::new().build().expect("table"),
        profile,
    };
    let mut names = NameTable::new(IdentifierNamespace::Fixture);
    names
        .intern(Name::new("PriorName"))
        .expect("prior name allocation");
    let before = names.to_archive_bytes().expect("before").to_vec();
    let outcome = textual.unview(
        COMMIT_SEQUENCE,
        &TextualForm::single("$forbidden".to_owned()),
        &mut names,
    );
    assert!(matches!(outcome, Err(FixtureError::Recognize(_))));
    assert_eq!(before, names.to_archive_bytes().expect("after").as_ref());
}

#[test]
fn a_table_and_textual_profile_identity_disagreement_is_typed() {
    let textual = FixtureTextual {
        table: FixtureBuilder::new().build().expect("standard table"),
        profile: custom_profile(2),
    };
    let mut names = NameTable::new(IdentifierNamespace::Fixture);
    let outcome = textual.unview(
        FIELD,
        &TextualForm::single("Integer".to_owned()),
        &mut names,
    );
    assert!(matches!(
        outcome,
        Err(FixtureError::Profile(
            TextualProfileError::IdentityMismatch { .. }
        ))
    ));
}
