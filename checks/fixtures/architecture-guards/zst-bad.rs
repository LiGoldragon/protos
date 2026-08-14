pub(crate) struct Empty {}
struct Tuple();

trait Acts {
    fn act(&self);
}

impl Acts for Empty {
    fn act(&self) {}
}

impl Acts for Tuple {
    fn act(&self) {}
}

impl Acts for (Empty) {
    fn act(&self) {}
}
