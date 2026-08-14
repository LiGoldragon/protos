struct Subject {
    value: u8,
}

trait Methods {
    fn method(&self) -> u8;
}

impl Methods for Subject {
    fn method(&self) -> u8 {
        self.value
    }
}
