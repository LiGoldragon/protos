pub struct RepeatedData {
    value: u8,
}

trait RepeatedBehavior {
    fn act(&self);
}

impl RepeatedBehavior for RepeatedData {
    fn act(&self) {}
}
