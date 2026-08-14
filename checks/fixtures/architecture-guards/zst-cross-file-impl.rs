trait CrossActs {
    fn act(&self);
}

impl CrossActs for (crate::CrossFile) {
    fn act(&self) {}
}
