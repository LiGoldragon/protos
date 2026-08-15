struct Subject<T> {
    value: T,
}

impl<T> Subject<T>
where
    T: for<'a> Bound<'a>,
{
    fn method(&self) -> &T {
        &self.value
    }
}

trait Bound<'a> {}

struct r#for;

impl r#for {
    fn raw_identifier_method(&self) {}
}
