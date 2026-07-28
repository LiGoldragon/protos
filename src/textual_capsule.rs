use std::error::Error;

use crate::{Capsule, CapsuleKind};

/// A fixed, bidirectional association between one textual representation and
/// one kind-typed Capsule.
///
/// Implement this trait on the type that owns the association. The associated
/// kind and complete NameTree-pin type make the Capsule selection single-valued
/// for that owner. Different owners may still select the same Capsule type, so
/// multiple textual syntaxes can project one semantic Capsule.
///
/// A caller cannot select another kind at the call site:
///
/// ```compile_fail
/// use protos::{Capsule, CapsuleKind, TextualCapsuleAssociation};
///
/// fn unview_as<Association, CallerKind>(
///     textual: &Association::TextualRepresentation,
/// ) -> Capsule<CallerKind, Association::CompleteNameTreePin>
/// where
///     Association: TextualCapsuleAssociation,
///     CallerKind: CapsuleKind,
/// {
///     Association::unview_capsule(textual).expect("projection succeeds")
/// }
/// ```
///
/// Rust coherence rejects two Capsule selections by one association owner:
///
/// ```compile_fail,E0119
/// use std::convert::Infallible;
/// use std::marker::PhantomData;
///
/// use protos::{Capsule, CapsuleKind, TextualCapsuleAssociation};
///
/// struct Association<First, Second>(PhantomData<(First, Second)>);
///
/// impl<First: CapsuleKind, Second> TextualCapsuleAssociation
///     for Association<First, Second>
/// {
///     type TextualRepresentation = String;
///     type Kind = First;
///     type CompleteNameTreePin = [u8; 32];
///     type ViewError = Infallible;
///     type UnviewError = Infallible;
///
///     fn view_capsule(
///         _: &Capsule<Self::Kind, Self::CompleteNameTreePin>,
///     ) -> Result<Self::TextualRepresentation, Self::ViewError> {
///         unreachable!()
///     }
///
///     fn unview_capsule(
///         _: &Self::TextualRepresentation,
///     ) -> Result<Capsule<Self::Kind, Self::CompleteNameTreePin>, Self::UnviewError> {
///         unreachable!()
///     }
/// }
///
/// impl<First, Second: CapsuleKind> TextualCapsuleAssociation
///     for Association<First, Second>
/// {
///     type TextualRepresentation = String;
///     type Kind = Second;
///     type CompleteNameTreePin = [u8; 32];
///     type ViewError = Infallible;
///     type UnviewError = Infallible;
///
///     fn view_capsule(
///         _: &Capsule<Self::Kind, Self::CompleteNameTreePin>,
///     ) -> Result<Self::TextualRepresentation, Self::ViewError> {
///         unreachable!()
///     }
///
///     fn unview_capsule(
///         _: &Self::TextualRepresentation,
///     ) -> Result<Capsule<Self::Kind, Self::CompleteNameTreePin>, Self::UnviewError> {
///         unreachable!()
///     }
/// }
/// ```
pub trait TextualCapsuleAssociation {
    /// The complete textual representation participating in this projection.
    type TextualRepresentation;
    /// The one Capsule kind fixed by this association owner.
    type Kind: CapsuleKind;
    /// The opaque complete NameTree-pin type carried by the fixed Capsule.
    type CompleteNameTreePin;
    /// A typed failure while viewing the Capsule textually.
    type ViewError: Error + Send + Sync + 'static;
    /// A typed failure while recovering the Capsule from the textual view.
    type UnviewError: Error + Send + Sync + 'static;

    /// Purely produce the associated textual view of a Capsule.
    fn view_capsule(
        capsule: &Capsule<Self::Kind, Self::CompleteNameTreePin>,
    ) -> Result<Self::TextualRepresentation, Self::ViewError>;

    /// Purely recover the fixed Capsule from its textual view.
    fn unview_capsule(
        textual: &Self::TextualRepresentation,
    ) -> Result<Capsule<Self::Kind, Self::CompleteNameTreePin>, Self::UnviewError>;
}
