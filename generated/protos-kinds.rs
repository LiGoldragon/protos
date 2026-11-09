#![allow(
    dead_code,
    non_camel_case_types,
    non_snake_case,
    clippy::type_complexity
)]
#[rustfmt::skip]
pub trait Spendable {
    fn spend(&mut self) -> bool;
}
#[rustfmt::skip]
pub trait BoundedProtosizable {
    fn protosize_with<N: Spendable>(
        &self,
        input: N,
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
pub trait Compactable {
    fn compact(&self) -> String;
}
#[rustfmt::skip]
pub trait Canonicalizable {}
#[rustfmt::skip]
const _: () = {
    fn assert_readerbudget_spendable<T: Spendable>() {}
    let _ = assert_readerbudget_spendable::<crate::ReaderBudget>;
};
