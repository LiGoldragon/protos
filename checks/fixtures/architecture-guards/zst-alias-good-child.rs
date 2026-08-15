pub struct AttributedData {
    value: u8,
}

trait AttributedBehavior {
    fn act(&self);
}

impl AttributedBehavior for AttributedData {
    fn act(&self) {}
}
