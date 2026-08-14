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
