use std::error::Error;

use content_identity::ShortCode;
use name_table::NameTable;
use structural_codec::{ScopedEncodedTypeId, Textual, TextualForm};

use crate::Capsule;

/// Component-owned state used while a textual view becomes a Capsule.
///
/// The mutable nametree is explicit. Implementations may carry additional typed
/// construction state, but this contract provides no mint or hidden authority.
pub trait CapsuleUnviewContext {
    /// The explicit nametree mutated while the component constructs a Capsule.
    fn nametree(&mut self) -> &mut NameTable;
}

impl CapsuleUnviewContext for NameTable {
    fn nametree(&mut self) -> &mut NameTable {
        self
    }
}

/// Associates one textual projection type with one fixed Capsule type.
///
/// Rust coherence permits only one implementation for a projection type.
/// Several different projection types may select the same Capsule. The encoded
/// equality prevents a projection-specific semantic identity.
///
/// A second association implementation for one projection conflicts by Rust
/// coherence:
///
/// ```compile_fail
/// use protos::TextualCapsuleAssociation;
///
/// struct OneProjection;
///
/// impl TextualCapsuleAssociation for OneProjection {
///     type Capsule = ();
///     type UnviewContext = name_table::NameTable;
///     type AssociationError = std::convert::Infallible;
///
///     fn unview_capsule(
///         &self,
///         _: structural_codec::ScopedEncodedTypeId,
///         _: &structural_codec::TextualForm<Self::Language>,
///         _: Self::UnviewContext,
///         _: content_identity::ShortCode,
///     ) -> Result<Self::Capsule, Self::AssociationError> {
///         unreachable!()
///     }
///
///     fn view_capsule(
///         &self,
///         _: structural_codec::ScopedEncodedTypeId,
///         _: &Self::Capsule,
///     ) -> Result<structural_codec::TextualForm<Self::Language>, Self::AssociationError> {
///         unreachable!()
///     }
/// }
///
/// impl TextualCapsuleAssociation for OneProjection {
///     type Capsule = ();
///     type UnviewContext = name_table::NameTable;
///     type AssociationError = std::convert::Infallible;
///     // A projection cannot select a second Capsule type.
/// }
/// ```
///
/// The associated Capsule's encoded truth cannot differ from the projection's
/// `Textual::Encoded` type:
///
/// ```compile_fail
/// use protos::{Capsule, TextualCapsuleAssociation};
/// use structural_codec::Textual;
///
/// fn incompatible<Projection>(
///     capsule: &<Projection as TextualCapsuleAssociation>::Capsule,
/// )
/// where
///     Projection: TextualCapsuleAssociation + Textual<Encoded = u8>,
/// {
///     let _: &u16 = capsule.encoded_form();
/// }
/// ```
pub trait TextualCapsuleAssociation: Textual {
    /// The one Capsule type selected by this projection.
    type Capsule: Capsule<EncodedForm = <Self as Textual>::Encoded>;
    /// Explicit component-owned state needed to construct that Capsule.
    type UnviewContext: CapsuleUnviewContext;
    /// The component's typed association failure.
    type AssociationError: Error + From<<Self as Textual>::Error> + Send + Sync + 'static;

    /// Build a Capsule from a textual view using only explicit context and an
    /// already-issued short identifier.
    fn unview_capsule(
        &self,
        expected: ScopedEncodedTypeId,
        view: &TextualForm<Self::Language>,
        context: Self::UnviewContext,
        short_identifier: ShortCode,
    ) -> Result<Self::Capsule, Self::AssociationError>;

    /// Produce this projection's textual view of a Capsule.
    fn view_capsule(
        &self,
        expected: ScopedEncodedTypeId,
        capsule: &Self::Capsule,
    ) -> Result<TextualForm<Self::Language>, Self::AssociationError>;
}
