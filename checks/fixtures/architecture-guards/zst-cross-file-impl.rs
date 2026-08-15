trait CrossActs {
    fn act(&self);
}

impl CrossActs for (crate::CrossFile) {
    fn act(&self) {}
}

mod data {
    struct CrossFile {
        value: u8,
    }

    trait Acts {
        fn act(&self);
    }

    impl Acts for CrossFile {
        fn act(&self) {}
    }
}
