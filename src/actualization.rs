//! Potential: text that may become a value, and the descent that actualizes it.

use std::fmt;
use std::marker::PhantomData;

use crate::anatomy::{Delineation, Extent, Potential, Protoform, Situated, Situation};
use crate::kinds::{Actualizable, Conceivable, Incorporable, Protosizable, Texted};
use std::convert::Infallible;

impl<T, C> From<String> for Potential<T, C> {
    fn from(text: String) -> Self {
        Potential(text, PhantomData)
    }
}

impl<T, C> From<&str> for Potential<T, C> {
    fn from(text: &str) -> Self {
        Potential(text.to_owned(), PhantomData)
    }
}

impl<T, C> Texted for Potential<T, C> {
    fn text(&self) -> &str {
        &self.0
    }
}

impl<T, C> Clone for Potential<T, C> {
    fn clone(&self) -> Self {
        Potential(self.0.clone(), PhantomData)
    }
}

impl<T, C> fmt::Debug for Potential<T, C> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Potential").field(&self.0).finish()
    }
}

impl<T, C> PartialEq for Potential<T, C> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl<T, C> Eq for Potential<T, C> {}

impl Conceivable<Delineation> for Protoform {
    type Fault = Infallible;

    fn conceive(&self) -> Result<Situated<Delineation>, Self::Fault> {
        Ok(Situated(
            Situation {
                extent: Extent(0, 0),
                children: vec![],
            },
            Delineation(vec![Situated(
                Situation {
                    extent: Extent(0, 0),
                    children: vec![],
                },
                self.clone(),
            )]),
        ))
    }
}

impl Protosizable for Protoform {
    type Fault = Infallible;

    fn protosize(&self) -> Result<Delineation, Self::Fault> {
        Ok(self.conceive().expect("infallible protoform projection").1)
    }
}

impl Conceivable<Delineation> for Delineation {
    type Fault = Infallible;

    fn conceive(&self) -> Result<Situated<Delineation>, Self::Fault> {
        Ok(Situated(
            Situation {
                extent: Extent(0, 0),
                children: vec![],
            },
            self.clone(),
        ))
    }
}

impl Conceivable<Delineation> for Situated<Protoform> {
    type Fault = Infallible;

    fn conceive(&self) -> Result<Situated<Delineation>, Self::Fault> {
        Ok(Situated(self.0.clone(), Delineation(vec![self.clone()])))
    }
}

impl Protosizable for Delineation {
    type Fault = Infallible;

    fn protosize(&self) -> Result<Delineation, Self::Fault> {
        Ok(self.clone())
    }
}

/// The descent: protosize the text, conceive the concept, incorporate the value.
impl<T, C> Actualizable<T> for Potential<T, C>
where
    Delineation: Conceivable<C>,
    C: Incorporable<T>,
    <C as Incorporable<T>>::Fault:
        From<<Delineation as Conceivable<C>>::Fault> + From<crate::anatomy::Fault>,
{
    type Fault = <C as Incorporable<T>>::Fault;

    fn actualize(&self) -> Result<T, Self::Fault> {
        let delineation = self.0.protosize()?;
        let Situated(at, concept) = delineation.conceive()?;
        concept.incorporate(&at)
    }
}
