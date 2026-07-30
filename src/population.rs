//! Neutral encoded-population carrier shared by Protos components.

/// One encoded form paired positionally with its complete NameTree value.
///
/// This type deliberately assigns no identity, Capsule-pin composition, slot,
/// authorization, or deployment semantics to either value. Those remain the
/// responsibility of the component that owns the population.
#[derive(Clone, Debug, Eq, PartialEq, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize)]
pub struct EncodedPopulation<EncodedForm, NameTree>(EncodedForm, NameTree);

impl<EncodedForm, NameTree> EncodedPopulation<EncodedForm, NameTree> {
    /// Pair an encoded form with its complete NameTree.
    pub const fn new(encoded_form: EncodedForm, name_tree: NameTree) -> Self {
        Self(encoded_form, name_tree)
    }

    /// Borrow the complete encoded form.
    pub const fn encoded_form(&self) -> &EncodedForm {
        &self.0
    }

    /// Borrow the complete NameTree value.
    pub const fn name_tree(&self) -> &NameTree {
        &self.1
    }

    /// Consume the carrier without changing either positional value.
    pub fn into_parts(self) -> (EncodedForm, NameTree) {
        (self.0, self.1)
    }
}
