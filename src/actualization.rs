//! Actualization: Potential to Corporate (the chain pass).
//!
//! Actualizable chains the whole descent: protosize, conceive,
//! incorporate. Situating looks up an extent by path.

use crate::{
    Actualizable, Conceivable, Delineation, Extent, Fault, Incorporable, Integer, Pathed,
    Potential, Situated, Situating, Texted,
};

// ---------------------------------------------------------------------------
// Actualizable for Potential: the universal chain
// ---------------------------------------------------------------------------

impl<C, T> Actualizable<T> for Potential<T, C>
where
    C: Incorporable<T>,
    Delineation: Conceivable<C>,
    <C as Incorporable<T>>::Fault:
        From<Fault> + From<<Delineation as Conceivable<C>>::Fault> + Pathed,
{
    type Fault = Situated<<C as Incorporable<T>>::Fault>;

    fn actualize(&self) -> Result<T, Self::Fault> {
        let delineation = <crate::Text as crate::Protosizable>::protosize(&self.text().to_owned())
            .map_err(|f| {
                let extent = Some(f.extent);
                Situated(extent, <C as Incorporable<T>>::Fault::from(f))
            })?;

        let concept: C = delineation.conceive().map_err(|f| {
            let fault = <C as Incorporable<T>>::Fault::from(f);
            let extent = delineation.situate(fault.path());
            Situated(extent, fault)
        })?;

        concept.incorporate().map_err(|f| {
            let extent = delineation.situate(f.path());
            Situated(extent, f)
        })
    }
}

// ---------------------------------------------------------------------------
// Situating for Delineation: lookup by path
// ---------------------------------------------------------------------------

impl Situating for Delineation {
    fn situate(&self, path: &[Integer]) -> Option<Extent> {
        self.situation.get(path).copied()
    }
}
