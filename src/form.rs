/// The text-to-real direction. It is called on the textual type.
pub trait Realize {
    type Real;
    type Fault;

    fn realize(&self) -> Result<Self::Real, Self::Fault>;
}

/// The real-to-text direction. It is called on the real type.
pub trait Textualize {
    type Textual;

    fn textualize(&self) -> Self::Textual;
}
