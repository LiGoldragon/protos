#![allow(dead_code, non_camel_case_types, non_snake_case)]
#[rustfmt::skip]
#[derive(datom_codec::Datomizable, datom_codec::Compositional, Clone, Debug, PartialEq)]
pub struct Extent {
    pub first_integer: i64,
    pub second_integer: i64,
}
#[rustfmt::skip]
pub type Symbol = String;
#[rustfmt::skip]
#[derive(datom_codec::Datomizable, datom_codec::Compositional, Clone, Debug, PartialEq)]
pub enum Separator {
    Period,
    Exclamation,
    Colon,
}
#[rustfmt::skip]
#[derive(datom_codec::Datomizable, datom_codec::Compositional, Clone, Debug, PartialEq)]
pub enum Enclosure {
    Braced,
    Bracketed,
    Angled,
}
#[rustfmt::skip]
#[derive(datom_codec::Datomizable, datom_codec::Compositional, Clone, Debug, PartialEq)]
pub enum Boundary {
    Guillemets,
    Parentheses,
}
#[rustfmt::skip]
#[derive(datom_codec::Datomizable, datom_codec::Compositional, Clone, Debug, PartialEq)]
pub struct Headed_Data {
    pub extent: Extent,
    pub symbol: Symbol,
    pub protos_option: Option<std::boxed::Box<Protos>>,
    pub separator: Separator,
    pub protos: std::boxed::Box<Protos>,
}
#[derive(datom_codec::Datomizable, datom_codec::Compositional, Clone, Debug, PartialEq)]
pub struct Enclosed_Data {
    pub extent: Extent,
    pub enclosure: Enclosure,
    pub protos_vector: std::vec::Vec<Protos>,
}
#[derive(datom_codec::Datomizable, datom_codec::Compositional, Clone, Debug, PartialEq)]
pub struct Opaque_Data {
    pub extent: Extent,
    pub boundary: Boundary,
    pub string: String,
}
#[derive(datom_codec::Datomizable, datom_codec::Compositional, Clone, Debug, PartialEq)]
pub struct Bare_Data {
    pub extent: Extent,
    pub string: String,
}
#[derive(datom_codec::Datomizable, datom_codec::Compositional, Clone, Debug, PartialEq)]
pub enum Protos {
    Headed(Headed_Data),
    Enclosed(Enclosed_Data),
    Opaque(Opaque_Data),
    Bare(Bare_Data),
}
#[rustfmt::skip]
#[derive(datom_codec::Datomizable, datom_codec::Compositional, Clone, Debug, PartialEq)]
pub struct Error {
    pub extent: Extent,
    pub problem: Problem,
}
#[rustfmt::skip]
#[derive(datom_codec::Datomizable, datom_codec::Compositional, Clone, Debug, PartialEq)]
pub enum Problem {
    Empty,
    Multiple,
    Unclosed(String),
    Unexpected(String),
    MissingHead,
    MissingBody,
    Budget,
    Depth,
}
#[rustfmt::skip]
#[derive(datom_codec::Datomizable, datom_codec::Compositional, Clone, Debug, PartialEq)]
pub struct ReaderBudget {
    pub integer: i64,
}
#[rustfmt::skip]
pub trait Textualizable {
    fn textualize(&self) -> String;
}
#[rustfmt::skip]
pub trait Canonicalizable {}
