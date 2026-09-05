#![allow(dead_code)]
pub type Path = Vec<protos::Integer>;
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Extent(pub protos::Integer, pub protos::Integer);
impl datom_codec::Datomic for Extent {
    fn incorporate(site: datom_codec::Site<'_>) -> Result<Self, datom_codec::Fault> {
        let mut p = datom_codec::Sited::positions(site, 2)?;
        let p0: protos::Integer = datom_codec::Positional::position(&mut p)?;
        let p1: protos::Integer = datom_codec::Positional::position(&mut p)?;
        Ok(Self(p0, p1))
    }
    fn conceive(&self) -> datom_codec::Datom {
        datom_codec::Datom::Struct(
            vec![
                datom_codec::Datomic::conceive(& self.0),
                datom_codec::Datomic::conceive(& self.1)
            ],
        )
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Situation(pub Extent, pub Vec<Situation>);
impl datom_codec::Datomic for Situation {
    fn incorporate(site: datom_codec::Site<'_>) -> Result<Self, datom_codec::Fault> {
        let mut p = datom_codec::Sited::positions(site, 2)?;
        let p0: Extent = datom_codec::Positional::position(&mut p)?;
        let p1: Vec<Situation> = datom_codec::Positional::position(&mut p)?;
        Ok(Self(p0, p1))
    }
    fn conceive(&self) -> datom_codec::Datom {
        datom_codec::Datom::Struct(
            vec![
                datom_codec::Datomic::conceive(& self.0),
                datom_codec::Datomic::conceive(& self.1)
            ],
        )
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Refusal(pub crate::Text, pub protos::Integer);
impl datom_codec::Datomic for Refusal {
    fn incorporate(site: datom_codec::Site<'_>) -> Result<Self, datom_codec::Fault> {
        let mut p = datom_codec::Sited::positions(site, 2)?;
        let p0: crate::Text = datom_codec::Positional::position(&mut p)?;
        let p1: protos::Integer = datom_codec::Positional::position(&mut p)?;
        Ok(Self(p0, p1))
    }
    fn conceive(&self) -> datom_codec::Datom {
        datom_codec::Datom::Struct(
            vec![
                datom_codec::Datomic::conceive(& self.0),
                datom_codec::Datomic::conceive(& self.1)
            ],
        )
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Separator {
    Period,
    Exclamation,
    Colon,
}
impl datom_codec::Datomic for Separator {
    fn incorporate(site: datom_codec::Site<'_>) -> Result<Self, datom_codec::Fault> {
        let v = datom_codec::Sited::variant(site)?;
        match v.name {
            "Period" => {
                datom_codec::Headed::nothing(v)?;
                Ok(Self::Period)
            }
            "Exclamation" => {
                datom_codec::Headed::nothing(v)?;
                Ok(Self::Exclamation)
            }
            "Colon" => {
                datom_codec::Headed::nothing(v)?;
                Ok(Self::Colon)
            }
            _ => {
                Err(
                    datom_codec::Sited::refuse(
                        site,
                        datom_codec::Problem::UnknownVariant(v.name.to_owned()),
                    ),
                )
            }
        }
    }
    fn conceive(&self) -> datom_codec::Datom {
        match self {
            Self::Period => datom_codec::Datom::Word("Period".to_owned()),
            Self::Exclamation => datom_codec::Datom::Word("Exclamation".to_owned()),
            Self::Colon => datom_codec::Datom::Word("Colon".to_owned()),
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Enclosure {
    Braced,
    Bracketed,
    Angled,
}
impl datom_codec::Datomic for Enclosure {
    fn incorporate(site: datom_codec::Site<'_>) -> Result<Self, datom_codec::Fault> {
        let v = datom_codec::Sited::variant(site)?;
        match v.name {
            "Braced" => {
                datom_codec::Headed::nothing(v)?;
                Ok(Self::Braced)
            }
            "Bracketed" => {
                datom_codec::Headed::nothing(v)?;
                Ok(Self::Bracketed)
            }
            "Angled" => {
                datom_codec::Headed::nothing(v)?;
                Ok(Self::Angled)
            }
            _ => {
                Err(
                    datom_codec::Sited::refuse(
                        site,
                        datom_codec::Problem::UnknownVariant(v.name.to_owned()),
                    ),
                )
            }
        }
    }
    fn conceive(&self) -> datom_codec::Datom {
        match self {
            Self::Braced => datom_codec::Datom::Word("Braced".to_owned()),
            Self::Bracketed => datom_codec::Datom::Word("Bracketed".to_owned()),
            Self::Angled => datom_codec::Datom::Word("Angled".to_owned()),
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Boundary {
    CurlyQuotes,
    Parentheses,
}
impl datom_codec::Datomic for Boundary {
    fn incorporate(site: datom_codec::Site<'_>) -> Result<Self, datom_codec::Fault> {
        let v = datom_codec::Sited::variant(site)?;
        match v.name {
            "CurlyQuotes" => {
                datom_codec::Headed::nothing(v)?;
                Ok(Self::CurlyQuotes)
            }
            "Parentheses" => {
                datom_codec::Headed::nothing(v)?;
                Ok(Self::Parentheses)
            }
            _ => {
                Err(
                    datom_codec::Sited::refuse(
                        site,
                        datom_codec::Problem::UnknownVariant(v.name.to_owned()),
                    ),
                )
            }
        }
    }
    fn conceive(&self) -> datom_codec::Datom {
        match self {
            Self::CurlyQuotes => datom_codec::Datom::Word("CurlyQuotes".to_owned()),
            Self::Parentheses => datom_codec::Datom::Word("Parentheses".to_owned()),
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Glyph {
    Space,
    Comment,
    Open(Enclosure),
    Close(Enclosure),
    Bound(Boundary),
    Unbound(Boundary),
    Separate(Separator),
    Plain,
}
impl datom_codec::Datomic for Glyph {
    fn incorporate(site: datom_codec::Site<'_>) -> Result<Self, datom_codec::Fault> {
        let v = datom_codec::Sited::variant(site)?;
        match v.name {
            "Space" => {
                datom_codec::Headed::nothing(v)?;
                Ok(Self::Space)
            }
            "Comment" => {
                datom_codec::Headed::nothing(v)?;
                Ok(Self::Comment)
            }
            "Open" => Ok(Self::Open(datom_codec::Carrying::body(v)?)),
            "Close" => Ok(Self::Close(datom_codec::Carrying::body(v)?)),
            "Bound" => Ok(Self::Bound(datom_codec::Carrying::body(v)?)),
            "Unbound" => Ok(Self::Unbound(datom_codec::Carrying::body(v)?)),
            "Separate" => Ok(Self::Separate(datom_codec::Carrying::body(v)?)),
            "Plain" => {
                datom_codec::Headed::nothing(v)?;
                Ok(Self::Plain)
            }
            _ => {
                Err(
                    datom_codec::Sited::refuse(
                        site,
                        datom_codec::Problem::UnknownVariant(v.name.to_owned()),
                    ),
                )
            }
        }
    }
    fn conceive(&self) -> datom_codec::Datom {
        match self {
            Self::Space => datom_codec::Datom::Word("Space".to_owned()),
            Self::Comment => datom_codec::Datom::Word("Comment".to_owned()),
            Self::Open(p0) => {
                datom_codec::Datom::Variant(
                    "Open".to_owned(),
                    Box::new(datom_codec::Datomic::conceive(p0)),
                )
            }
            Self::Close(p0) => {
                datom_codec::Datom::Variant(
                    "Close".to_owned(),
                    Box::new(datom_codec::Datomic::conceive(p0)),
                )
            }
            Self::Bound(p0) => {
                datom_codec::Datom::Variant(
                    "Bound".to_owned(),
                    Box::new(datom_codec::Datomic::conceive(p0)),
                )
            }
            Self::Unbound(p0) => {
                datom_codec::Datom::Variant(
                    "Unbound".to_owned(),
                    Box::new(datom_codec::Datomic::conceive(p0)),
                )
            }
            Self::Separate(p0) => {
                datom_codec::Datom::Variant(
                    "Separate".to_owned(),
                    Box::new(datom_codec::Datomic::conceive(p0)),
                )
            }
            Self::Plain => datom_codec::Datom::Word("Plain".to_owned()),
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Head {
    Symbol(crate::Symbol),
    Qualified(crate::Symbol, Vec<Protoform>),
}
impl datom_codec::Datomic for Head {
    fn incorporate(site: datom_codec::Site<'_>) -> Result<Self, datom_codec::Fault> {
        let v = datom_codec::Sited::variant(site)?;
        match v.name {
            "Symbol" => Ok(Self::Symbol(datom_codec::Carrying::body(v)?)),
            "Qualified" => {
                let mut p = datom_codec::Headed::positions(v, 2)?;
                let p0: crate::Symbol = datom_codec::Positional::position(&mut p)?;
                let p1: Vec<Protoform> = datom_codec::Positional::position(&mut p)?;
                Ok(Self::Qualified(p0, p1))
            }
            _ => {
                Err(
                    datom_codec::Sited::refuse(
                        site,
                        datom_codec::Problem::UnknownVariant(v.name.to_owned()),
                    ),
                )
            }
        }
    }
    fn conceive(&self) -> datom_codec::Datom {
        match self {
            Self::Symbol(p0) => {
                datom_codec::Datom::Variant(
                    "Symbol".to_owned(),
                    Box::new(datom_codec::Datomic::conceive(p0)),
                )
            }
            Self::Qualified(p0, p1) => {
                datom_codec::Datom::Variant(
                    "Qualified".to_owned(),
                    Box::new(
                        datom_codec::Datom::Struct(
                            vec![
                                datom_codec::Datomic::conceive(p0),
                                datom_codec::Datomic::conceive(p1)
                            ],
                        ),
                    ),
                )
            }
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Protoform {
    Headed(Head, Separator, Box<Protoform>),
    Enclosed(Enclosure, Vec<Protoform>),
    Quoted(crate::Text),
    Parenthesized(crate::Opaque),
    Bare(crate::Bare),
    Qualified(crate::Symbol, Vec<Protoform>),
}
impl datom_codec::Datomic for Protoform {
    fn incorporate(site: datom_codec::Site<'_>) -> Result<Self, datom_codec::Fault> {
        let v = datom_codec::Sited::variant(site)?;
        match v.name {
            "Headed" => {
                let mut p = datom_codec::Headed::positions(v, 3)?;
                let p0: Head = datom_codec::Positional::position(&mut p)?;
                let p1: Separator = datom_codec::Positional::position(&mut p)?;
                let p2: Box<Protoform> = datom_codec::Positional::position(&mut p)?;
                Ok(Self::Headed(p0, p1, p2))
            }
            "Enclosed" => {
                let mut p = datom_codec::Headed::positions(v, 2)?;
                let p0: Enclosure = datom_codec::Positional::position(&mut p)?;
                let p1: Vec<Protoform> = datom_codec::Positional::position(&mut p)?;
                Ok(Self::Enclosed(p0, p1))
            }
            "Quoted" => Ok(Self::Quoted(datom_codec::Carrying::body(v)?)),
            "Parenthesized" => Ok(Self::Parenthesized(datom_codec::Carrying::body(v)?)),
            "Bare" => Ok(Self::Bare(datom_codec::Carrying::body(v)?)),
            "Qualified" => {
                let mut p = datom_codec::Headed::positions(v, 2)?;
                let p0: crate::Symbol = datom_codec::Positional::position(&mut p)?;
                let p1: Vec<Protoform> = datom_codec::Positional::position(&mut p)?;
                Ok(Self::Qualified(p0, p1))
            }
            _ => {
                Err(
                    datom_codec::Sited::refuse(
                        site,
                        datom_codec::Problem::UnknownVariant(v.name.to_owned()),
                    ),
                )
            }
        }
    }
    fn conceive(&self) -> datom_codec::Datom {
        match self {
            Self::Headed(p0, p1, p2) => {
                datom_codec::Datom::Variant(
                    "Headed".to_owned(),
                    Box::new(
                        datom_codec::Datom::Struct(
                            vec![
                                datom_codec::Datomic::conceive(p0),
                                datom_codec::Datomic::conceive(p1),
                                datom_codec::Datomic::conceive(p2)
                            ],
                        ),
                    ),
                )
            }
            Self::Enclosed(p0, p1) => {
                datom_codec::Datom::Variant(
                    "Enclosed".to_owned(),
                    Box::new(
                        datom_codec::Datom::Struct(
                            vec![
                                datom_codec::Datomic::conceive(p0),
                                datom_codec::Datomic::conceive(p1)
                            ],
                        ),
                    ),
                )
            }
            Self::Quoted(p0) => {
                datom_codec::Datom::Variant(
                    "Quoted".to_owned(),
                    Box::new(datom_codec::Datomic::conceive(p0)),
                )
            }
            Self::Parenthesized(p0) => {
                datom_codec::Datom::Variant(
                    "Parenthesized".to_owned(),
                    Box::new(datom_codec::Datomic::conceive(p0)),
                )
            }
            Self::Bare(p0) => {
                datom_codec::Datom::Variant(
                    "Bare".to_owned(),
                    Box::new(datom_codec::Datomic::conceive(p0)),
                )
            }
            Self::Qualified(p0, p1) => {
                datom_codec::Datom::Variant(
                    "Qualified".to_owned(),
                    Box::new(
                        datom_codec::Datom::Struct(
                            vec![
                                datom_codec::Datomic::conceive(p0),
                                datom_codec::Datomic::conceive(p1)
                            ],
                        ),
                    ),
                )
            }
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Delineation(pub Vec<crate::Situated<Protoform>>);
impl datom_codec::Datomic for Delineation {
    fn incorporate(site: datom_codec::Site<'_>) -> Result<Self, datom_codec::Fault> {
        let mut p = datom_codec::Sited::positions(site, 1)?;
        let p0: Vec<crate::Situated<Protoform>> = datom_codec::Positional::position(
            &mut p,
        )?;
        Ok(Self(p0))
    }
    fn conceive(&self) -> datom_codec::Datom {
        datom_codec::Datom::Struct(vec![datom_codec::Datomic::conceive(& self.0)])
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Problem {
    Unclosed(Enclosure),
    Unopened(Enclosure),
    Unterminated(Boundary),
    Stray(Boundary),
}
impl datom_codec::Datomic for Problem {
    fn incorporate(site: datom_codec::Site<'_>) -> Result<Self, datom_codec::Fault> {
        let v = datom_codec::Sited::variant(site)?;
        match v.name {
            "Unclosed" => Ok(Self::Unclosed(datom_codec::Carrying::body(v)?)),
            "Unopened" => Ok(Self::Unopened(datom_codec::Carrying::body(v)?)),
            "Unterminated" => Ok(Self::Unterminated(datom_codec::Carrying::body(v)?)),
            "Stray" => Ok(Self::Stray(datom_codec::Carrying::body(v)?)),
            _ => {
                Err(
                    datom_codec::Sited::refuse(
                        site,
                        datom_codec::Problem::UnknownVariant(v.name.to_owned()),
                    ),
                )
            }
        }
    }
    fn conceive(&self) -> datom_codec::Datom {
        match self {
            Self::Unclosed(p0) => {
                datom_codec::Datom::Variant(
                    "Unclosed".to_owned(),
                    Box::new(datom_codec::Datomic::conceive(p0)),
                )
            }
            Self::Unopened(p0) => {
                datom_codec::Datom::Variant(
                    "Unopened".to_owned(),
                    Box::new(datom_codec::Datomic::conceive(p0)),
                )
            }
            Self::Unterminated(p0) => {
                datom_codec::Datom::Variant(
                    "Unterminated".to_owned(),
                    Box::new(datom_codec::Datomic::conceive(p0)),
                )
            }
            Self::Stray(p0) => {
                datom_codec::Datom::Variant(
                    "Stray".to_owned(),
                    Box::new(datom_codec::Datomic::conceive(p0)),
                )
            }
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Fault(pub Extent, pub Problem);
impl datom_codec::Datomic for Fault {
    fn incorporate(site: datom_codec::Site<'_>) -> Result<Self, datom_codec::Fault> {
        let mut p = datom_codec::Sited::positions(site, 2)?;
        let p0: Extent = datom_codec::Positional::position(&mut p)?;
        let p1: Problem = datom_codec::Positional::position(&mut p)?;
        Ok(Self(p0, p1))
    }
    fn conceive(&self) -> datom_codec::Datom {
        datom_codec::Datom::Struct(
            vec![
                datom_codec::Datomic::conceive(& self.0),
                datom_codec::Datomic::conceive(& self.1)
            ],
        )
    }
}
const _: () = {
    fn assert_text_protosizable<T: crate::Protosizable>() {}
    let _ = assert_text_protosizable::<crate::Text>;
};
const _: () = {
    fn assert_protoform_textualizable<T: crate::Textualizable>() {}
    let _ = assert_protoform_textualizable::<Protoform>;
    fn assert_protoform_situating<T: crate::Situating>() {}
    let _ = assert_protoform_situating::<Protoform>;
    fn assert_protoform_protosizable<T: crate::Protosizable>() {}
    let _ = assert_protoform_protosizable::<Protoform>;
    fn assert_protoform_conceivable_delineation<T: crate::Conceivable<Delineation>>() {}
    let _ = assert_protoform_conceivable_delineation::<Protoform>;
};
const _: () = {
    fn assert_delineation_textualizable<T: crate::Textualizable>() {}
    let _ = assert_delineation_textualizable::<Delineation>;
    fn assert_delineation_protosizable<T: crate::Protosizable>() {}
    let _ = assert_delineation_protosizable::<Delineation>;
    fn assert_delineation_conceivable_delineation<T: crate::Conceivable<Delineation>>() {}
    let _ = assert_delineation_conceivable_delineation::<Delineation>;
};
const _: () = {
    fn assert_situation_locating<T: crate::Locating>() {}
    let _ = assert_situation_locating::<Situation>;
};
