pub struct AttributedZst;

trait AttributedBehavior {
    fn act(&self);
}

impl AttributedBehavior for AttributedZst {
    fn act(&self) {}
}
