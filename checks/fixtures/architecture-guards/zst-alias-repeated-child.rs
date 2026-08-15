pub struct RepeatedZst;

trait RepeatedBehavior {
    fn act(&self);
}

impl RepeatedBehavior for RepeatedZst {
    fn act(&self) {}
}
