//! Compile-time laws for the universal Interface roles.

use std::fmt;

use protos::{Input, Output, Refusal};

struct Command;
struct Event;

#[derive(Debug)]
struct Rejected;

impl Input for Command {}
impl Output for Event {}
impl Refusal for Rejected {}

impl fmt::Display for Rejected {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("rejected")
    }
}

impl std::error::Error for Rejected {}

#[test]
fn positional_roles_are_implementation_free_and_refusal_is_a_rust_error() {
    fn accepts_input<Message: Input>() {}
    fn accepts_output<Message: Output>() {}
    fn accepts_refusal<Error: Refusal>(error: &Error) -> &dyn std::error::Error {
        error
    }

    accepts_input::<Command>();
    accepts_output::<Event>();
    assert_eq!(accepts_refusal(&Rejected).to_string(), "rejected");
}
