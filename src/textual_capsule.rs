use std::error::Error;

use crate::Capsule;

/// A fixed, bidirectional association between one textual representation and
/// one Capsule type.
///
/// Implement this trait on a type that owns the association, not on the
/// textual representation. The owner may be a type-level association marker or
/// a component type with that responsibility. Its associated types make the
/// relation single-valued: one implementation selects exactly one textual
/// representation and exactly one Capsule. Different association owners may
/// select the same Capsule, allowing several textual syntaxes to project one
/// semantic Capsule.
///
/// The textual representation is deliberately independent of
/// `structural_codec::Textual`. It may represent source text, a whole document,
/// or another textual view without exposing structural-codec's internal
/// `TextualForm` or scoped encoded identifiers.
///
/// A caller cannot select a different Capsule at a call site:
///
/// ```compile_fail
/// use protos::{Capsule, TextualCapsuleAssociation};
///
/// fn unview_as<Association, CallerSelectedCapsule>(
///     textual: &Association::TextualRepresentation,
/// ) -> CallerSelectedCapsule
/// where
///     Association: TextualCapsuleAssociation,
///     CallerSelectedCapsule: Capsule,
/// {
///     Association::unview_capsule(textual).expect("projection succeeds")
/// }
/// ```
///
/// Rust coherence also rejects two Capsule selections by one association
/// owner:
///
/// ```compile_fail,E0119
/// use std::convert::Infallible;
/// use std::marker::PhantomData;
///
/// use protos::{Capsule, TextualCapsuleAssociation};
///
/// struct Association<First, Second>(PhantomData<(First, Second)>);
///
/// impl<First: Capsule, Second> TextualCapsuleAssociation for Association<First, Second> {
///     type TextualRepresentation = String;
///     type Capsule = First;
///     type ViewError = Infallible;
///     type UnviewError = Infallible;
///
///     fn view_capsule(_: &Self::Capsule) -> Result<Self::TextualRepresentation, Self::ViewError> {
///         unreachable!()
///     }
///
///     fn unview_capsule(
///         _: &Self::TextualRepresentation,
///     ) -> Result<Self::Capsule, Self::UnviewError> {
///         unreachable!()
///     }
/// }
///
/// impl<First, Second: Capsule> TextualCapsuleAssociation for Association<First, Second> {
///     type TextualRepresentation = String;
///     type Capsule = Second;
///     type ViewError = Infallible;
///     type UnviewError = Infallible;
///
///     fn view_capsule(_: &Self::Capsule) -> Result<Self::TextualRepresentation, Self::ViewError> {
///         unreachable!()
///     }
///
///     fn unview_capsule(
///         _: &Self::TextualRepresentation,
///     ) -> Result<Self::Capsule, Self::UnviewError> {
///         unreachable!()
///     }
/// }
/// ```
pub trait TextualCapsuleAssociation {
    /// The complete textual representation participating in this projection.
    type TextualRepresentation;
    /// The one Capsule type fixed by this association owner.
    type Capsule: Capsule;
    /// A typed failure while viewing the Capsule textually.
    type ViewError: Error + Send + Sync + 'static;
    /// A typed failure while recovering the Capsule from the textual view.
    type UnviewError: Error + Send + Sync + 'static;

    /// Purely produce the associated textual view of a Capsule.
    fn view_capsule(
        capsule: &Self::Capsule,
    ) -> Result<Self::TextualRepresentation, Self::ViewError>;

    /// Purely recover the associated Capsule from its textual view.
    fn unview_capsule(
        textual: &Self::TextualRepresentation,
    ) -> Result<Self::Capsule, Self::UnviewError>;
}
