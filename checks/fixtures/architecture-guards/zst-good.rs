#[derive(Clone, Copy)]
pub(crate) struct Marker;
struct BracedMarker {}
struct TupleMarker();

struct Data {
    value: u8,
}

trait Acts {
    fn act(&self);
}

impl Acts for Data {
    fn act(&self) {}
}
