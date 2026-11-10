//! Under the `rkyv` feature, the protos types an ethos declaration may hold
//! in a position (an extent, a separator, a reading error and its problem)
//! archive and restore equal, so a generated contract holding them can
//! cross a wire.

use protos::{Error, Extent, ExtentAccess, Position, Problem, Separator};

#[test]
fn a_reading_error_and_a_separator_cross_the_wire() {
    let error = Error {
        extent: Extent::new(
            Position::try_from(3).unwrap(),
            Position::try_from(9).unwrap(),
        )
        .unwrap(),
        problem: Problem::Unclosed('{'),
    };
    let bytes = rkyv::to_bytes::<rkyv::rancor::Error>(&error).expect("error archives");
    let restored = rkyv::from_bytes::<Error, rkyv::rancor::Error>(&bytes).expect("error restores");
    assert_eq!(restored, error);
    for separator in [Separator::Period, Separator::Exclamation, Separator::Colon] {
        let bytes = rkyv::to_bytes::<rkyv::rancor::Error>(&separator).expect("separator archives");
        let restored =
            rkyv::from_bytes::<Separator, rkyv::rancor::Error>(&bytes).expect("separator restores");
        assert_eq!(restored, separator);
    }
}
