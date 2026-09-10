#![allow(dead_code, non_camel_case_types, non_snake_case)]
#[derive(datom_codec::Datomizable, datom_codec::Compositional)]
pub struct Extent {
    pub first_integer: i64,
    pub second_integer: i64,
}
pub type Symbol = String;
#[derive(datom_codec::Datomizable, datom_codec::Compositional)]
pub enum Separator {
    Period,
    Exclamation,
    Colon,
}
#[derive(datom_codec::Datomizable, datom_codec::Compositional)]
pub enum Enclosure {
    Braced,
    Bracketed,
    Angled,
}
#[derive(datom_codec::Datomizable, datom_codec::Compositional)]
pub enum Boundary {
    Guillemets,
    Parentheses,
}
#[derive(datom_codec::Datomizable, datom_codec::Compositional)]
pub struct Headed_Data {
    pub extent: Extent,
    pub symbol: Symbol,
    pub protos_option: Option<std::boxed::Box<Protos>>,
    pub separator: Separator,
    pub protos: std::boxed::Box<Protos>,
}
#[derive(datom_codec::Datomizable, datom_codec::Compositional)]
pub struct Enclosed_Data {
    pub extent: Extent,
    pub enclosure: Enclosure,
    pub protos_vector: std::vec::Vec<Protos>,
}
#[derive(datom_codec::Datomizable, datom_codec::Compositional)]
pub struct Opaque_Data {
    pub extent: Extent,
    pub boundary: Boundary,
    pub string: String,
}
#[derive(datom_codec::Datomizable, datom_codec::Compositional)]
pub struct Bare_Data {
    pub extent: Extent,
    pub string: String,
}
#[derive(datom_codec::Datomizable, datom_codec::Compositional)]
pub enum Protos {
    Headed(Headed_Data),
    Enclosed(Enclosed_Data),
    Opaque(Opaque_Data),
    Bare(Bare_Data),
}
#[derive(datom_codec::Datomizable, datom_codec::Compositional)]
pub struct Error {
    pub extent: Extent,
    pub problem: Problem,
}
#[derive(datom_codec::Datomizable, datom_codec::Compositional)]
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
#[derive(datom_codec::Datomizable, datom_codec::Compositional)]
pub struct ReaderBudget {
    pub integer: i64,
}
pub trait Textualizable {
    fn textualize(&self) -> String;
}
pub trait Canonicalizable {}
