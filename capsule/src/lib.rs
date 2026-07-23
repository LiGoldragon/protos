//! The standard self-contained exchange container for an encoded form and its
//! complete composed nametree.
//!
//! A Capsule carries one encoded value, every separately archived name slice it
//! needs, the two identities that pin those values, and an agent-addressable
//! short identifier. Borrowed slices remain distinct archives: Capsule never
//! flattens, renumbers, restamps, or copies them into an owned slice.

use content_identity::{ArchiveError, ContentHash, DomainSeparation, HashDomain, LayoutVersion};
use name_table::{
    IdentifierNamespace, NameTable, NameTableDomain, NameTableError, NameTableSliceSnapshot,
};
use rkyv::Deserialize as RkyvDeserialize;
use rkyv::api::high::HighDeserializer;
use rkyv::bytecheck::CheckBytes;
use rkyv::rancor::{self, Strategy};
use rkyv::ser::Serializer;
use rkyv::ser::allocator::ArenaHandle;
use rkyv::ser::sharing::Share;
use rkyv::validation::Validator;
use rkyv::validation::archive::ArchiveValidator;
use rkyv::validation::shared::SharedValidator;
use short_identifier::{ShortCode, ShortIdentifier};
use thiserror::Error;

/// The typed identity domain for a complete, ordered Capsule nametree
/// composition. It folds namespace-tagged per-slice identities and topology,
/// never a flattened name list.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CapsuleNameTreeDomain;

impl HashDomain for CapsuleNameTreeDomain {
    fn separation() -> DomainSeparation {
        DomainSeparation::Contextual {
            context: "capsule 2026 complete composed nametree",
            layout: LayoutVersion::new(1),
        }
    }
}

/// The versioned rkyv representation carried by an exchange Capsule.
///
/// Adding a durable representation changes this closed enum rather than
/// reinterpreting Version1 bytes.
#[derive(Clone, Copy, Debug, Eq, PartialEq, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize)]
pub enum CapsuleArchiveLayout {
    /// The initial complete-composition Capsule archive.
    Version1,
}

/// The runtime label stored beside a typed identity pin at the archive boundary.
///
/// `ContentHash`'s generic domain prevents a normal in-memory mix-up. This
/// explicit tag makes a malformed archive's claimed identity domain observable
/// and therefore rejectable as a typed `WrongIdentityDomain` failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize)]
pub enum CapsuleIdentityDomain {
    /// The stringless encoded-form content identity.
    Content,
    /// The complete composed nametree identity.
    Nametree,
}

/// The member to which an identity pin belongs.
#[derive(Clone, Copy, Debug, Eq, PartialEq, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize)]
pub enum CapsuleMember {
    /// The encoded program value.
    Content,
    /// The complete composed nametree.
    Nametree,
}

/// A typed content hash together with its archive-level domain declaration.
///
/// This is not a digest type: all digest data remains `ContentHash<Domain>`.
#[derive(Clone, Debug, Eq, PartialEq, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize)]
pub struct CapsuleIdentityPin<Domain: HashDomain> {
    domain: CapsuleIdentityDomain,
    hash: ContentHash<Domain>,
}

impl<Domain: HashDomain> CapsuleIdentityPin<Domain> {
    /// Construct a pin with its archive-level domain declaration.
    pub fn new(domain: CapsuleIdentityDomain, hash: ContentHash<Domain>) -> Self {
        Self { domain, hash }
    }

    /// The domain declared at this capsule archive boundary.
    pub fn domain(&self) -> CapsuleIdentityDomain {
        self.domain
    }

    /// The typed identity itself.
    pub fn hash(&self) -> ContentHash<Domain> {
        self.hash
    }
}

/// The durable identity projection of one Capsule for provenance.
///
/// Short identifiers locate loaded daemon slots and intentionally never enter
/// this projection.
#[derive(Clone, Debug, Eq, PartialEq, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize)]
pub struct CapsuleIdentityProjection<ContentDomain: HashDomain> {
    content: ContentHash<ContentDomain>,
    nametree: ContentHash<CapsuleNameTreeDomain>,
}

impl<ContentDomain: HashDomain> CapsuleIdentityProjection<ContentDomain> {
    /// The encoded-form content identity.
    pub fn content(&self) -> ContentHash<ContentDomain> {
        self.content
    }

    /// The complete composed nametree identity.
    pub fn nametree(&self) -> ContentHash<CapsuleNameTreeDomain> {
        self.nametree
    }
}

/// A complete nametree carried as separately pinned slices.
#[derive(Clone, Debug, Eq, PartialEq, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize)]
pub struct CapsuleNameTree {
    home: NameTableSliceSnapshot,
    borrowed: Vec<NameTableSliceSnapshot>,
}

#[derive(Clone, Debug, Eq, PartialEq, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize)]
struct CapsuleNameTreeIdentitySlice {
    namespace: IdentifierNamespace,
    identity: ContentHash<NameTableDomain>,
}

#[derive(Clone, Debug, Eq, PartialEq, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize)]
struct CapsuleNameTreeIdentityPreimage {
    home: CapsuleNameTreeIdentitySlice,
    borrowed: Vec<CapsuleNameTreeIdentitySlice>,
}

impl CapsuleNameTree {
    /// Capture every slice of a composed NameTable without changing the existing
    /// per-slice archive or identity contracts.
    pub fn from_name_table(names: &NameTable) -> Result<Self, CapsuleNameTreeError> {
        let (home, borrowed) = names.slice_snapshots()?;
        let tree = Self { home, borrowed };
        tree.rederive_identity()?;
        Ok(tree)
    }

    /// The component-owned home slice.
    pub fn home(&self) -> &NameTableSliceSnapshot {
        &self.home
    }

    /// Borrowed source slices in canonical namespace order.
    pub fn borrowed(&self) -> &[NameTableSliceSnapshot] {
        &self.borrowed
    }

    /// Rebuild the composed NameTable by composing independently verified slices.
    pub fn restore(&self) -> Result<NameTable, CapsuleNameTreeError> {
        self.validate_topology()?;
        Ok(NameTable::from_slice_snapshots(&self.home, &self.borrowed)?)
    }

    /// Re-derive the complete composition identity from namespace-tagged slice
    /// pins and topology. Slice contents stay in their independently pinned
    /// archives; they are not flattened into this pre-image.
    pub fn rederive_identity(
        &self,
    ) -> Result<ContentHash<CapsuleNameTreeDomain>, CapsuleNameTreeError> {
        self.validate_topology()?;
        let _ = self.restore()?;
        let preimage = CapsuleNameTreeIdentityPreimage {
            home: CapsuleNameTreeIdentitySlice {
                namespace: self.home.namespace(),
                identity: self.home.identity(),
            },
            borrowed: self
                .borrowed
                .iter()
                .map(|slice| CapsuleNameTreeIdentitySlice {
                    namespace: slice.namespace(),
                    identity: slice.identity(),
                })
                .collect(),
        };
        Ok(ContentHash::<CapsuleNameTreeDomain>::of_core(&preimage)?)
    }

    fn validate_topology(&self) -> Result<(), CapsuleNameTreeError> {
        let mut previous = None;
        for slice in &self.borrowed {
            let namespace = slice.namespace();
            if namespace == self.home.namespace()
                || previous.is_some_and(|previous_namespace| previous_namespace >= namespace)
            {
                return Err(CapsuleNameTreeError::InvalidTopology { namespace });
            }
            previous = Some(namespace);
        }
        Ok(())
    }
}

/// A Capsule nametree construction or restore failure.
#[derive(Clone, Debug, Error)]
pub enum CapsuleNameTreeError {
    /// One independently archived slice was not a valid NameTable slice.
    #[error(transparent)]
    NameTable(#[from] NameTableError),

    /// The declared borrowed-slice topology repeats, includes the home, or is
    /// not in deterministic namespace order.
    #[error("the Capsule nametree topology is not canonical at {namespace:?}")]
    InvalidTopology { namespace: IdentifierNamespace },

    /// Deriving the composition identity could not archive its canonical
    /// structural pre-image.
    #[error(transparent)]
    Archive(#[from] ArchiveError),
}

/// The four structural verification failures approved for Capsules.
#[derive(Clone, Debug, Error, Eq, PartialEq, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize)]
pub enum CapsuleVerificationFailure {
    /// The encoded payload no longer derives to its pinned identity.
    #[error("the Capsule encoded payload does not match its content pin")]
    ContentMismatch,

    /// The complete composed nametree no longer derives to its pinned identity.
    #[error("the Capsule nametree does not match its composition pin")]
    NametreeMismatch,

    /// An archive-level identity pin declares the wrong domain for its member.
    #[error("the {member:?} pin declares {actual:?}, expected {expected:?}")]
    WrongIdentityDomain {
        /// The affected Capsule member.
        member: CapsuleMember,
        /// The required domain declaration.
        expected: CapsuleIdentityDomain,
        /// The declaration carried by the malformed pin.
        actual: CapsuleIdentityDomain,
    },

    /// A required pin is absent.
    #[error("the Capsule {0:?} identity pin is missing")]
    MissingIdentityPin(CapsuleMember),
}

/// A typed failure while sealing an otherwise valid in-memory Capsule.
#[derive(Clone, Debug, Error)]
pub enum CapsuleSealError {
    /// The encoded form could not be archived to derive its content identity.
    #[error(transparent)]
    Archive(#[from] ArchiveError),

    /// The complete composed nametree could not be captured and pinned.
    #[error(transparent)]
    Nametree(#[from] CapsuleNameTreeError),
}

/// An encoded form that can provide its canonical portable archive bytes.
///
/// The blanket implementation keeps Capsule content generic while putting the
/// rkyv validation bounds at the one boundary where they are needed.
pub trait CapsuleContent: Clone {
    /// Canonical, validated portable archive bytes for content identity.
    fn canonical_archive_bytes(&self) -> Result<rkyv::util::AlignedVec, ArchiveError>;
}

impl<Value> CapsuleContent for Value
where
    Value: rkyv::Archive
        + Clone
        + for<'serialize> rkyv::Serialize<
            Strategy<
                Serializer<rkyv::util::AlignedVec, ArenaHandle<'serialize>, Share>,
                rancor::Error,
            >,
        >,
    Value::Archived: RkyvDeserialize<Value, HighDeserializer<rancor::Error>>
        + for<'validation> CheckBytes<
            Strategy<Validator<ArchiveValidator<'validation>, SharedValidator>, rancor::Error>,
        >,
{
    fn canonical_archive_bytes(&self) -> Result<rkyv::util::AlignedVec, ArchiveError> {
        rkyv::to_bytes::<rancor::Error>(self)
            .map_err(|error| ArchiveError::Serialize(error.to_string()))
    }
}

/// A self-contained exchange unit.
///
/// Required items are structural data only: encoded form, complete composed
/// nametree, both identity pins, and a short identifier. Verification and the
/// durable identity projection are provided once here for every implementation.
pub trait Capsule {
    /// The stringless encoded form this Capsule carries.
    type EncodedForm: CapsuleContent;
    /// The domain that identifies this encoded form's canonical bytes.
    type ContentDomain: HashDomain;

    /// The encoded program value.
    fn encoded_form(&self) -> &Self::EncodedForm;
    /// The complete composed nametree travelling with the value.
    fn nametree(&self) -> &CapsuleNameTree;
    /// The optionally present content pin, allowing missing archive pins to be
    /// rejected as a distinct typed verification failure.
    fn content_identity_pin(&self) -> Option<&CapsuleIdentityPin<Self::ContentDomain>>;
    /// The optionally present complete-nametree composition pin.
    fn nametree_identity_pin(&self) -> Option<&CapsuleIdentityPin<CapsuleNameTreeDomain>>;
    /// The agent-addressable loaded-slot identifier.
    fn short_identifier(&self) -> &ShortCode;

    /// Re-derive both pins and enforce their domain declarations.
    fn verify(&self) -> Result<(), CapsuleVerificationFailure> {
        let content_pin =
            self.content_identity_pin()
                .ok_or(CapsuleVerificationFailure::MissingIdentityPin(
                    CapsuleMember::Content,
                ))?;
        if content_pin.domain() != CapsuleIdentityDomain::Content {
            return Err(CapsuleVerificationFailure::WrongIdentityDomain {
                member: CapsuleMember::Content,
                expected: CapsuleIdentityDomain::Content,
                actual: content_pin.domain(),
            });
        }
        let actual_content = ContentHash::<Self::ContentDomain>::derive(
            self.encoded_form()
                .canonical_archive_bytes()
                .map_err(|_| CapsuleVerificationFailure::ContentMismatch)?
                .as_ref(),
        );
        if content_pin.hash() != actual_content {
            return Err(CapsuleVerificationFailure::ContentMismatch);
        }

        let nametree_pin =
            self.nametree_identity_pin()
                .ok_or(CapsuleVerificationFailure::MissingIdentityPin(
                    CapsuleMember::Nametree,
                ))?;
        if nametree_pin.domain() != CapsuleIdentityDomain::Nametree {
            return Err(CapsuleVerificationFailure::WrongIdentityDomain {
                member: CapsuleMember::Nametree,
                expected: CapsuleIdentityDomain::Nametree,
                actual: nametree_pin.domain(),
            });
        }
        let actual_nametree = self
            .nametree()
            .rederive_identity()
            .map_err(|_| CapsuleVerificationFailure::NametreeMismatch)?;
        if nametree_pin.hash() != actual_nametree {
            return Err(CapsuleVerificationFailure::NametreeMismatch);
        }
        Ok(())
    }

    /// Project the two durable identities for generation provenance after
    /// verification. The short identifier intentionally remains outside it.
    fn identity_projection(
        &self,
    ) -> Result<CapsuleIdentityProjection<Self::ContentDomain>, CapsuleVerificationFailure> {
        self.verify()?;
        let content = self
            .content_identity_pin()
            .ok_or(CapsuleVerificationFailure::MissingIdentityPin(
                CapsuleMember::Content,
            ))?
            .hash();
        let nametree = self
            .nametree_identity_pin()
            .ok_or(CapsuleVerificationFailure::MissingIdentityPin(
                CapsuleMember::Nametree,
            ))?
            .hash();
        Ok(CapsuleIdentityProjection { content, nametree })
    }
}

/// The standard generic Capsule exchange container.
///
/// It is an rkyv value under the family portable-archive discipline. Callers
/// must invoke [`Capsule::verify`] after an untrusted archive boundary before
/// accepting the pins as provenance.
#[derive(Clone, Debug, Eq, PartialEq, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize)]
pub struct ExchangeCapsule<Content, ContentDomain: HashDomain> {
    archive_layout: CapsuleArchiveLayout,
    encoded: Content,
    nametree: CapsuleNameTree,
    content_pin: Option<CapsuleIdentityPin<ContentDomain>>,
    nametree_pin: Option<CapsuleIdentityPin<CapsuleNameTreeDomain>>,
    short_identifier: ShortCode,
}

impl<Content, ContentDomain> ExchangeCapsule<Content, ContentDomain>
where
    Content: CapsuleContent,
    ContentDomain: HashDomain,
{
    /// Seal a valid encoded form and complete composed nametree into the
    /// standard exchange container.
    pub fn seal(
        encoded: Content,
        names: &NameTable,
        short_identifier: ShortCode,
    ) -> Result<Self, CapsuleSealError> {
        let nametree = CapsuleNameTree::from_name_table(names)?;
        let content =
            ContentHash::<ContentDomain>::derive(encoded.canonical_archive_bytes()?.as_ref());
        let name_identity = nametree.rederive_identity()?;
        Ok(Self {
            archive_layout: CapsuleArchiveLayout::Version1,
            encoded,
            nametree,
            content_pin: Some(CapsuleIdentityPin::new(
                CapsuleIdentityDomain::Content,
                content,
            )),
            nametree_pin: Some(CapsuleIdentityPin::new(
                CapsuleIdentityDomain::Nametree,
                name_identity,
            )),
            short_identifier,
        })
    }

    /// The archive layout in which this concrete Capsule is stored.
    pub fn archive_layout(&self) -> CapsuleArchiveLayout {
        self.archive_layout
    }
}

impl<Content, ContentDomain> Capsule for ExchangeCapsule<Content, ContentDomain>
where
    Content: CapsuleContent,
    ContentDomain: HashDomain,
{
    type EncodedForm = Content;
    type ContentDomain = ContentDomain;

    fn encoded_form(&self) -> &Self::EncodedForm {
        &self.encoded
    }

    fn nametree(&self) -> &CapsuleNameTree {
        &self.nametree
    }

    fn content_identity_pin(&self) -> Option<&CapsuleIdentityPin<Self::ContentDomain>> {
        self.content_pin.as_ref()
    }

    fn nametree_identity_pin(&self) -> Option<&CapsuleIdentityPin<CapsuleNameTreeDomain>> {
        self.nametree_pin.as_ref()
    }

    fn short_identifier(&self) -> &ShortCode {
        &self.short_identifier
    }
}

impl<Content, ContentDomain> ShortIdentifier for ExchangeCapsule<Content, ContentDomain>
where
    Content: CapsuleContent,
    ContentDomain: HashDomain,
{
    fn short_identifier(&self) -> &ShortCode {
        &self.short_identifier
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use content_identity::PortableArchive;
    use name_table::{Identifier, Name};

    #[derive(Clone, Debug, Eq, PartialEq, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize)]
    struct FixtureEncoded(u32);

    #[derive(Clone, Debug, Eq, PartialEq)]
    struct FixtureDomain;

    impl HashDomain for FixtureDomain {
        fn separation() -> DomainSeparation {
            DomainSeparation::Contextual {
                context: "capsule fixture encoded content",
                layout: LayoutVersion::new(1),
            }
        }
    }

    type FixtureCapsule = ExchangeCapsule<FixtureEncoded, FixtureDomain>;

    fn code() -> ShortCode {
        ShortCode::new("caps".to_owned()).expect("valid code")
    }

    fn composed_names(schema_name: &str, logos_name: &str) -> NameTable {
        let mut schema = NameTable::new(IdentifierNamespace::Schema);
        schema.intern(Name::new(schema_name)).expect("schema name");
        let mut logos = NameTable::new(IdentifierNamespace::Logos);
        logos.intern(Name::new(logos_name)).expect("logos name");
        logos.compose(&schema).expect("compose schema")
    }

    fn sealed(value: u32, schema_name: &str, logos_name: &str) -> FixtureCapsule {
        FixtureCapsule::seal(
            FixtureEncoded(value),
            &composed_names(schema_name, logos_name),
            code(),
        )
        .expect("seal fixture Capsule")
    }

    #[test]
    fn self_contained_capsule_archives_and_restores_complete_composition() {
        let capsule = sealed(7, "SchemaRoot", "LogosRoot");
        capsule.verify().expect("valid Capsule");
        assert_eq!(capsule.archive_layout(), CapsuleArchiveLayout::Version1);
        let bytes = capsule.to_archive_bytes().expect("archive Capsule");
        let restored = FixtureCapsule::from_archive_bytes(bytes.as_ref()).expect("restore Capsule");
        restored.verify().expect("verify restored Capsule");
        let names = restored
            .nametree()
            .restore()
            .expect("restore composed names");
        assert_eq!(
            names
                .resolve(Identifier::Schema(0))
                .expect("borrowed schema name"),
            &Name::new("SchemaRoot")
        );
        assert_eq!(
            names
                .resolve(Identifier::Logos(0))
                .expect("owned logos name"),
            &Name::new("LogosRoot")
        );
        assert_eq!(restored.nametree().borrowed().len(), 1);
    }

    #[test]
    fn tampered_payload_is_a_content_mismatch() {
        let mut capsule = sealed(7, "SchemaRoot", "LogosRoot");
        capsule.encoded = FixtureEncoded(8);
        assert_eq!(
            capsule.verify(),
            Err(CapsuleVerificationFailure::ContentMismatch)
        );
    }

    #[test]
    fn swapped_nametree_is_a_nametree_mismatch() {
        let mut capsule = sealed(7, "SchemaRoot", "LogosRoot");
        let other = sealed(7, "OtherSchema", "OtherLogos");
        capsule.nametree = other.nametree;
        assert_eq!(
            capsule.verify(),
            Err(CapsuleVerificationFailure::NametreeMismatch)
        );
    }

    #[test]
    fn wrong_domain_identity_is_rejected() {
        let mut capsule = sealed(7, "SchemaRoot", "LogosRoot");
        capsule.content_pin.as_mut().expect("content pin").domain = CapsuleIdentityDomain::Nametree;
        assert_eq!(
            capsule.verify(),
            Err(CapsuleVerificationFailure::WrongIdentityDomain {
                member: CapsuleMember::Content,
                expected: CapsuleIdentityDomain::Content,
                actual: CapsuleIdentityDomain::Nametree,
            })
        );
    }

    #[test]
    fn missing_identity_pin_is_rejected() {
        let mut capsule = sealed(7, "SchemaRoot", "LogosRoot");
        capsule.nametree_pin = None;
        assert_eq!(
            capsule.verify(),
            Err(CapsuleVerificationFailure::MissingIdentityPin(
                CapsuleMember::Nametree
            ))
        );
    }

    #[test]
    fn renaming_moves_only_the_complete_nametree_identity() {
        let first = sealed(7, "SchemaRoot", "LogosRoot");
        let renamed = sealed(7, "RenamedSchema", "RenamedLogos");
        assert_eq!(
            first
                .identity_projection()
                .expect("first projection")
                .content(),
            renamed
                .identity_projection()
                .expect("renamed projection")
                .content()
        );
        assert_ne!(
            first
                .identity_projection()
                .expect("first projection")
                .nametree(),
            renamed
                .identity_projection()
                .expect("renamed projection")
                .nametree()
        );
    }
}
