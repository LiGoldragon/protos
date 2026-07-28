use std::error::Error;
use std::fmt;
use std::marker::PhantomData;

use content_identity::{
    ArchiveError, CapsuleIdentity, CapsuleIdentityVariant, ContentAddressedHash, PortableArchive,
};
use rkyv::Deserialize as RkyvDeserialize;
use rkyv::api::high::HighDeserializer;
use rkyv::bytecheck::CheckBytes;
use rkyv::rancor::{self, Strategy};
use rkyv::validation::Validator;
use rkyv::validation::archive::ArchiveValidator;
use rkyv::validation::shared::SharedValidator;

mod sealed {
    use content_identity::{CapsuleIdentity, CapsuleIdentityVariant, ContentAddressedHash};

    pub trait Sealed {
        const IDENTITY_VARIANT: CapsuleIdentityVariant;

        fn wrap_identity(hash: ContentAddressedHash) -> CapsuleIdentity;
    }
}

/// A component kind admitted by [`Capsule`].
///
/// This trait is sealed: the three marker types exported by this crate are the
/// complete construction set. Rust is a textual projection of Logos and has no
/// Capsule marker.
///
/// ```compile_fail
/// use protos::CapsuleKind;
///
/// struct ForeignKind;
///
/// impl CapsuleKind for ForeignKind {}
/// ```
pub trait CapsuleKind: sealed::Sealed {}

/// The Ethos Capsule kind.
#[derive(Debug)]
pub enum Ethos {}

/// The Nomos Capsule kind.
#[derive(Debug)]
pub enum Nomos {}

/// The whole-Logos Capsule kind.
#[derive(Debug)]
pub enum Logos {}

impl sealed::Sealed for Ethos {
    const IDENTITY_VARIANT: CapsuleIdentityVariant = CapsuleIdentityVariant::Ethos;

    fn wrap_identity(hash: ContentAddressedHash) -> CapsuleIdentity {
        CapsuleIdentity::ethos(hash)
    }
}

impl sealed::Sealed for Nomos {
    const IDENTITY_VARIANT: CapsuleIdentityVariant = CapsuleIdentityVariant::Nomos;

    fn wrap_identity(hash: ContentAddressedHash) -> CapsuleIdentity {
        CapsuleIdentity::nomos(hash)
    }
}

impl sealed::Sealed for Logos {
    const IDENTITY_VARIANT: CapsuleIdentityVariant = CapsuleIdentityVariant::WholeLogos;

    fn wrap_identity(hash: ContentAddressedHash) -> CapsuleIdentity {
        CapsuleIdentity::whole_logos(hash)
    }
}

impl CapsuleKind for Ethos {}
impl CapsuleKind for Nomos {}
impl CapsuleKind for Logos {}

/// A kind-typed Capsule carrying its stored identity and opaque complete
/// NameTree pin.
///
/// Construction accepts a raw pure-content hash. The kind marker selects the
/// sole stored outer identity variant; no second runtime discriminator is
/// archived. The pin is retained without composition, verification, or query.
///
/// All fields are private:
///
/// ```compile_fail
/// use protos::{Capsule, ContentAddressedHash, Logos};
///
/// let capsule = Capsule::<Logos, [u8; 32]>::new(
///     ContentAddressedHash::from_bytes([0; 32]),
///     [0; 32],
/// );
/// let Capsule(identity, pin, marker) = capsule;
/// ```
///
/// The checked methods are the archive boundary; `Capsule` itself is not a
/// generic rkyv archive root:
///
/// ```compile_fail
/// use protos::{Capsule, Logos};
///
/// fn require_archive<Value: rkyv::Archive>() {}
///
/// require_archive::<Capsule<Logos, [u8; 32]>>();
/// ```
///
/// Rust has no Capsule marker:
///
/// ```compile_fail
/// use protos::{Capsule, ContentAddressedHash, Rust};
///
/// let _ = Capsule::<Rust, [u8; 32]>::new(
///     ContentAddressedHash::from_bytes([0; 32]),
///     [0; 32],
/// );
/// ```
pub struct Capsule<Kind: CapsuleKind, CompleteNameTreePin>(
    CapsuleIdentity,
    CompleteNameTreePin,
    PhantomData<fn() -> Kind>,
);

impl<Kind, CompleteNameTreePin> Capsule<Kind, CompleteNameTreePin>
where
    Kind: CapsuleKind,
{
    /// Construct a Capsule whose outer identity variant is fixed by `Kind`.
    pub fn new(hash: ContentAddressedHash, complete_nametree_pin: CompleteNameTreePin) -> Self {
        Self(
            <Kind as sealed::Sealed>::wrap_identity(hash),
            complete_nametree_pin,
            PhantomData,
        )
    }

    /// The stored whole-Capsule identity.
    pub const fn content_identity(&self) -> CapsuleIdentity {
        self.0
    }

    /// The inner pure-content hash.
    pub const fn content_addressed_hash(&self) -> ContentAddressedHash {
        self.0.content_addressed_hash()
    }

    /// The opaque complete NameTree pin.
    pub const fn complete_nametree_pin(&self) -> &CompleteNameTreePin {
        &self.1
    }

    /// Consume the Capsule into its two stored positional fields.
    pub fn into_parts(self) -> (CapsuleIdentity, CompleteNameTreePin) {
        (self.0, self.1)
    }

    /// The identity variant required by this Capsule's kind parameter.
    pub const fn identity_variant() -> CapsuleIdentityVariant {
        <Kind as sealed::Sealed>::IDENTITY_VARIANT
    }

    /// Serialize the two stored positional fields using the portable archive
    /// discipline.
    ///
    /// The private archive carrier intentionally excludes the type marker.
    #[allow(private_bounds)]
    pub fn to_archive_bytes(&self) -> Result<Vec<u8>, CapsuleArchiveError>
    where
        CompleteNameTreePin: Clone,
        CapsuleArchive<CompleteNameTreePin>: CapsuleArchiveCodec,
    {
        let stored = CapsuleArchive(self.0, self.1.clone());
        Ok(stored.encode()?)
    }

    /// Restore a Capsule after validating both the rkyv body and the stored
    /// outer identity variant.
    ///
    /// A well-formed archive for another Capsule kind fails with
    /// [`CapsuleKindMismatch`] before a `Capsule<Kind, _>` is constructed.
    #[allow(private_bounds)]
    pub fn from_archive_bytes(bytes: &[u8]) -> Result<Self, CapsuleArchiveError>
    where
        CapsuleArchive<CompleteNameTreePin>: CapsuleArchiveCodec,
    {
        let CapsuleArchive(identity, complete_nametree_pin) =
            CapsuleArchive::<CompleteNameTreePin>::decode(bytes)?;
        let expected = <Kind as sealed::Sealed>::IDENTITY_VARIANT;
        let actual = identity.variant();
        if actual != expected {
            return Err(CapsuleKindMismatch(expected, actual).into());
        }
        Ok(Self(identity, complete_nametree_pin, PhantomData))
    }
}

impl<Kind, CompleteNameTreePin> Clone for Capsule<Kind, CompleteNameTreePin>
where
    Kind: CapsuleKind,
    CompleteNameTreePin: Clone,
{
    fn clone(&self) -> Self {
        Self(self.0, self.1.clone(), PhantomData)
    }
}

impl<Kind, CompleteNameTreePin> fmt::Debug for Capsule<Kind, CompleteNameTreePin>
where
    Kind: CapsuleKind,
    CompleteNameTreePin: fmt::Debug,
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_tuple("Capsule")
            .field(&self.0)
            .field(&self.1)
            .finish()
    }
}

impl<Kind, CompleteNameTreePin> PartialEq for Capsule<Kind, CompleteNameTreePin>
where
    Kind: CapsuleKind,
    CompleteNameTreePin: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0 && self.1 == other.1
    }
}

impl<Kind, CompleteNameTreePin> Eq for Capsule<Kind, CompleteNameTreePin>
where
    Kind: CapsuleKind,
    CompleteNameTreePin: Eq,
{
}

#[derive(Clone, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize)]
struct CapsuleArchive<CompleteNameTreePin>(CapsuleIdentity, CompleteNameTreePin);

trait CapsuleArchiveCodec: Sized {
    fn encode(&self) -> Result<Vec<u8>, ArchiveError>;

    fn decode(bytes: &[u8]) -> Result<Self, ArchiveError>;
}

impl<Value> CapsuleArchiveCodec for Value
where
    Value: PortableArchive,
    Value::Archived: RkyvDeserialize<Value, HighDeserializer<rancor::Error>>
        + for<'validation> CheckBytes<
            Strategy<Validator<ArchiveValidator<'validation>, SharedValidator>, rancor::Error>,
        >,
{
    fn encode(&self) -> Result<Vec<u8>, ArchiveError> {
        let bytes = self.to_archive_bytes()?;
        Ok(bytes.as_ref().to_vec())
    }

    fn decode(bytes: &[u8]) -> Result<Self, ArchiveError> {
        Self::from_archive_bytes(bytes)
    }
}

/// A stored Capsule identity is valid but belongs to another kind parameter.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CapsuleKindMismatch(CapsuleIdentityVariant, CapsuleIdentityVariant);

impl CapsuleKindMismatch {
    /// The variant required by the requested kind parameter.
    pub const fn expected(&self) -> CapsuleIdentityVariant {
        self.0
    }

    /// The valid variant actually stored in the archive.
    pub const fn actual(&self) -> CapsuleIdentityVariant {
        self.1
    }
}

impl fmt::Display for CapsuleKindMismatch {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "stored Capsule kind mismatch: expected {:?}, found {:?}",
            self.expected(),
            self.actual()
        )
    }
}

impl Error for CapsuleKindMismatch {}

/// A failure crossing the checked Capsule archive boundary.
#[derive(Clone, Debug)]
pub enum CapsuleArchiveError {
    /// The portable rkyv body could not be serialized or validated.
    Archive(ArchiveError),
    /// The body was valid but its stored outer identity variant did not match
    /// the requested kind parameter.
    KindMismatch(CapsuleKindMismatch),
}

impl fmt::Display for CapsuleArchiveError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Archive(error) => error.fmt(formatter),
            Self::KindMismatch(error) => error.fmt(formatter),
        }
    }
}

impl Error for CapsuleArchiveError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Archive(error) => Some(error),
            Self::KindMismatch(error) => Some(error),
        }
    }
}

impl From<ArchiveError> for CapsuleArchiveError {
    fn from(error: ArchiveError) -> Self {
        Self::Archive(error)
    }
}

impl From<CapsuleKindMismatch> for CapsuleArchiveError {
    fn from(error: CapsuleKindMismatch) -> Self {
        Self::KindMismatch(error)
    }
}
