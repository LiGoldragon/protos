struct Subject {
    value: u8,
}

impl Subject {
    fn method(&self) -> u8 {
        self.value
    }
}
