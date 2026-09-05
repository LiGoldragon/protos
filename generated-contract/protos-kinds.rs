#![allow(dead_code)]
pub trait Serial: Sized + std::marker::Copy {
    fn first() -> Self;
    fn after(&self) -> Option<Self>;
}
pub trait Classifying {
    fn classify(&self) -> crate::Glyph;
}
pub trait Textualizable {
    fn textualize(&self) -> std::string::String;
}
pub trait Situating {
    fn situate(&self) -> crate::Situated<std::string::String>;
}
pub trait Protosizable {
    type Fault;
    fn protosize(&self) -> Result<crate::Delineation, Self::Fault>;
}
pub trait Conceivable<A: Sized> {
    type Fault;
    fn conceive(&self) -> Result<crate::Situated<A>, Self::Fault>;
}
pub trait Actualizable<A: Sized> {
    type Fault;
    type Budget;
    fn actualize(&self, input: Self::Budget) -> Result<A, Self::Fault>;
}
