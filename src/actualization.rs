//! Potential: text that may become a value, and the descent that actualizes it.

use std::fmt;
use std::marker::PhantomData;

use crate::anatomy::{Delineation, Extent, Potential, Protoform, Situated, Situation};
use crate::kinds::{Actualizable, Conceivable, Protosizable, Route, Texted};
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

impl<T, C> Conceivable<C> for Box<T>
where
    T: Conceivable<C>,
{
    type Fault = T::Fault;

    fn conceive(&self) -> Result<Situated<C>, Self::Fault> {
        self.as_ref().conceive()
    }
}

impl Protosizable for Delineation {
    type Fault = Infallible;

    fn protosize(&self) -> Result<Delineation, Self::Fault> {
        Ok(self.clone())
    }
}

impl Protosizable for crate::Text {
    type Fault = crate::anatomy::Fault;

    fn protosize(&self) -> Result<Delineation, Self::Fault> {
        self.as_ref().protosize()
    }
}

impl<T, C> Actualizable<T> for Potential<T, C>
where
    C: Route<T>,
{
    type Fault = <C as Route<T>>::Fault;
    type Budget = <C as Route<T>>::Budget;

    fn actualize(&self, budget: Self::Budget) -> Result<T, Self::Fault> {
        C::run(&self.0, budget)
    }
}

impl Route<Protoform> for Protoform {
    type Fault = crate::anatomy::Fault;
    type Budget = ();

    fn run(text: &str, (): Self::Budget) -> Result<Protoform, Self::Fault> {
        let delineation = text.protosize()?;
        match delineation.0.as_slice() {
            [Situated(_, form)] => Ok(form.clone()),
            forms => Err(crate::anatomy::Fault {
                extent: Extent(0, text.len() as crate::Integer),
                problem: crate::anatomy::Problem::OneForm(forms.len() as crate::Integer),
            }),
        }
    }
}
