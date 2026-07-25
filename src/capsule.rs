use std::error::Error;
use std::fmt;

use content_identity::{CapsuleNameTreeDomain, ContentHash, HashDomain, ShortCode};

/// The complete set of canonical component Capsule roles.
///
/// Rust is a textual projection of Logos, never a fourth Capsule kind.
///
/// ```compile_fail
/// use protos::CapsuleKind;
///
/// let _ = CapsuleKind::Rust;
/// ```
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum CapsuleKind {
    /// The schema component's encoded truth.
    Schema,
    /// The logos component's encoded truth.
    Logos,
    /// The nomos component's encoded truth.
    Nomos,
}

impl CapsuleKind {
    /// Every canonical Capsule kind, in family order.
    pub const ALL: [Self; 3] = [Self::Schema, Self::Logos, Self::Nomos];
}

/// A value carrying the canonical numeric short identifier.
pub trait ShortIdentifier {
    /// The value's already-issued canonical code.
    fn short_identifier(&self) -> ShortCode;
}

/// A typed Capsule verification failure.
pub enum CapsuleVerificationError<ContentDomain, ContentIdentityError, NameTreeIdentityError>
where
    ContentDomain: HashDomain,
{
    /// Re-deriving the encoded truth's identity failed.
    ContentIdentityDerivation(ContentIdentityError),
    /// Re-deriving the complete nametree identity failed.
    NameTreeIdentityDerivation(NameTreeIdentityError),
    /// The required content pin differs from the derived identity.
    ContentMismatch {
        /// The identity stored in the Capsule.
        pinned: ContentHash<ContentDomain>,
        /// The identity derived from the encoded truth.
        actual: ContentHash<ContentDomain>,
    },
    /// The required nametree pin differs from the derived identity.
    NameTreeMismatch {
        /// The identity stored in the Capsule.
        pinned: ContentHash<CapsuleNameTreeDomain>,
        /// The identity derived from the complete nametree.
        actual: ContentHash<CapsuleNameTreeDomain>,
    },
}

impl<ContentDomain, ContentIdentityError, NameTreeIdentityError> fmt::Debug
    for CapsuleVerificationError<ContentDomain, ContentIdentityError, NameTreeIdentityError>
where
    ContentDomain: HashDomain,
    ContentIdentityError: fmt::Debug,
    NameTreeIdentityError: fmt::Debug,
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ContentIdentityDerivation(error) => formatter
                .debug_tuple("ContentIdentityDerivation")
                .field(error)
                .finish(),
            Self::NameTreeIdentityDerivation(error) => formatter
                .debug_tuple("NameTreeIdentityDerivation")
                .field(error)
                .finish(),
            Self::ContentMismatch { pinned, actual } => formatter
                .debug_struct("ContentMismatch")
                .field("pinned", pinned)
                .field("actual", actual)
                .finish(),
            Self::NameTreeMismatch { pinned, actual } => formatter
                .debug_struct("NameTreeMismatch")
                .field("pinned", pinned)
                .field("actual", actual)
                .finish(),
        }
    }
}

impl<ContentDomain, ContentIdentityError, NameTreeIdentityError> fmt::Display
    for CapsuleVerificationError<ContentDomain, ContentIdentityError, NameTreeIdentityError>
where
    ContentDomain: HashDomain,
    ContentIdentityError: fmt::Display,
    NameTreeIdentityError: fmt::Display,
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ContentIdentityDerivation(error) => {
                write!(formatter, "content identity derivation failed: {error}")
            }
            Self::NameTreeIdentityDerivation(error) => {
                write!(formatter, "nametree identity derivation failed: {error}")
            }
            Self::ContentMismatch { pinned, actual } => {
                write!(
                    formatter,
                    "content identity mismatch: pinned {pinned}, actual {actual}"
                )
            }
            Self::NameTreeMismatch { pinned, actual } => {
                write!(
                    formatter,
                    "nametree identity mismatch: pinned {pinned}, actual {actual}"
                )
            }
        }
    }
}

impl<ContentDomain, ContentIdentityError, NameTreeIdentityError> Error
    for CapsuleVerificationError<ContentDomain, ContentIdentityError, NameTreeIdentityError>
where
    ContentDomain: HashDomain,
    ContentIdentityError: Error + 'static,
    NameTreeIdentityError: Error + 'static,
{
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::ContentIdentityDerivation(error) => Some(error),
            Self::NameTreeIdentityDerivation(error) => Some(error),
            Self::ContentMismatch { .. } | Self::NameTreeMismatch { .. } => None,
        }
    }
}

/// The typed verification result for a particular Capsule implementation.
pub type CapsuleVerificationResult<CapsuleType> = Result<
    (),
    CapsuleVerificationError<
        <CapsuleType as Capsule>::ContentDomain,
        <CapsuleType as Capsule>::ContentIdentityError,
        <CapsuleType as Capsule>::NameTreeIdentityError,
    >,
>;

/// The implementation-free contract shared by the three component Capsules.
pub trait Capsule: ShortIdentifier {
    /// Which one of the three closed component roles this Capsule carries.
    const KIND: CapsuleKind;

    /// The component's stringless encoded truth.
    type EncodedForm: structural_codec::EncodedForm;
    /// The typed hash domain of the encoded truth.
    type ContentDomain: HashDomain;
    /// The complete component-owned name tree travelling with the truth.
    type NameTree;
    /// A pure encoded-identity derivation failure.
    type ContentIdentityError: Error + Send + Sync + 'static;
    /// A pure nametree-identity derivation failure.
    type NameTreeIdentityError: Error + Send + Sync + 'static;

    /// The encoded truth.
    fn encoded_form(&self) -> &Self::EncodedForm;
    /// The complete name tree.
    fn nametree(&self) -> &Self::NameTree;

    /// The required, typed content pin.
    fn content_identity_pin(&self) -> ContentHash<Self::ContentDomain>;
    /// The required, typed complete-nametree pin.
    fn nametree_identity_pin(&self) -> ContentHash<CapsuleNameTreeDomain>;

    /// Purely re-derive the content identity.
    fn rederive_content_identity(
        &self,
    ) -> Result<ContentHash<Self::ContentDomain>, Self::ContentIdentityError>;

    /// Purely re-derive the complete-nametree identity.
    fn rederive_nametree_identity(
        &self,
    ) -> Result<ContentHash<CapsuleNameTreeDomain>, Self::NameTreeIdentityError>;

    /// Verify both required pins against freshly derived values.
    fn verify(&self) -> CapsuleVerificationResult<Self> {
        let pinned_content = self.content_identity_pin();
        let actual_content = self
            .rederive_content_identity()
            .map_err(CapsuleVerificationError::ContentIdentityDerivation)?;
        if pinned_content != actual_content {
            return Err(CapsuleVerificationError::ContentMismatch {
                pinned: pinned_content,
                actual: actual_content,
            });
        }

        let pinned_nametree = self.nametree_identity_pin();
        let actual_nametree = self
            .rederive_nametree_identity()
            .map_err(CapsuleVerificationError::NameTreeIdentityDerivation)?;
        if pinned_nametree != actual_nametree {
            return Err(CapsuleVerificationError::NameTreeMismatch {
                pinned: pinned_nametree,
                actual: actual_nametree,
            });
        }
        Ok(())
    }
}
