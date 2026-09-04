use protos::{Extent, Fault, Problem, Situated};

/// Witness that Situated<Fault> supports PartialEq and Eq.
#[test]
fn situated_fault_eq() {
    let extent = Extent(0, 3);
    let fault = Fault {
        extent,
        problem: Problem::MissingHead,
    };

    let a = Situated(Some(extent), fault.clone());
    let b = Situated(Some(extent), fault.clone());
    let c = Situated(
        None,
        Fault {
            extent,
            problem: Problem::MissingBody,
        },
    );

    assert_eq!(a, b);
    assert_ne!(a, c);
}

/// Witness that Situated<Fault> supports Clone and Debug.
#[test]
fn situated_fault_clone_debug() {
    let situated = Situated(
        Some(Extent(0, 5)),
        Fault {
            extent: Extent(0, 5),
            problem: Problem::EmptyInput,
        },
    );
    let cloned = situated.clone();
    assert_eq!(situated, cloned);
    // Debug must not panic.
    let _ = format!("{:?}", situated);
}
