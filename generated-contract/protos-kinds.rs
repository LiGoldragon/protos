#![allow(dead_code, non_camel_case_types, non_snake_case)]
#[rustfmt::skip]
pub trait BoundedProtosizable {
    fn protosize_with(
        &mut self,
        input: crate::ReaderBudget,
    ) -> std::result::Result<crate::Protos, crate::Error>;
}
#[rustfmt::skip]
pub trait Protosizable {
    type Output;
    fn protosize(&self) -> Self::Output;
}
#[rustfmt::skip]
pub trait Textualizable {
    fn textualize(&self) -> String;
}
#[rustfmt::skip]
pub trait Canonicalizable {}
