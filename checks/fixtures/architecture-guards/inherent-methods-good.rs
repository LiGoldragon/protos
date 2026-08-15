struct Subject {
    value: u8,
}

trait Methods {
    fn method(&self) -> u8;
}

impl Methods for Subject {
    fn method(&self) -> u8 {
        self.value
    }
}

trait GenericMethods<T> {
    fn generic_method(&self);
}

impl<T> GenericMethods<T> for Subject
where
    T: for<'a> Bound<'a>,
{
    fn generic_method(&self) {}
}

trait Bound<'a> {}
