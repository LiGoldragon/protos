//! Compile-time laws for the universal Interface roles.

use std::fmt;

use protos::{Input, OpenedStream, Output, Refusal, Stream, StreamEvent, StreamIdentity, StreamOpen};

struct Command;
struct Event;

struct Initiation;

#[derive(Debug)]
struct Rejected;

struct Observation;

impl Input for Command {}
impl Output for Event {}
impl Input for Initiation {}
impl Refusal for Rejected {}
impl Stream for Observation {}

impl fmt::Display for Rejected {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("rejected")
    }
}

impl std::error::Error for Rejected {}

#[derive(Default)]
struct Runtime {
    pending: Option<Event>,
}

impl StreamOpen for Runtime {
    type Initiation = Initiation;
    type Event = Event;
    type InitiationRefusal = Rejected;

    fn open(
        &mut self,
        _initiation: Self::Initiation,
    ) -> Result<OpenedStream<Self::Event>, Self::InitiationRefusal> {
        self.pending = Some(Event);
        Ok(OpenedStream::new(StreamIdentity::new(7)))
    }
}

impl StreamEvent for Runtime {
    type Event = Event;

    fn next(&mut self, _stream: &OpenedStream<Self::Event>) -> Option<Self::Event> {
        self.pending.take()
    }
}

#[test]
fn positional_roles_are_implementation_free_and_refusal_is_a_rust_error() {
    fn accepts_input<Message: Input>() {}
    fn accepts_output<Message: Output>() {}
    fn accepts_refusal<Error: Refusal>(error: &Error) -> &dyn std::error::Error {
        error
    }

    fn accepts_stream<Declaration: Stream>() {}

    accepts_input::<Command>();
    accepts_output::<Event>();
    accepts_stream::<Observation>();
    assert_eq!(accepts_refusal(&Rejected).to_string(), "rejected");
}

#[test]
fn stream_contracts_preserve_event_typed_identity_without_runtime_storage() {
    let mut runtime = Runtime::default();
    let stream = runtime.open(Initiation).expect("establish stream");
    assert_eq!(stream.identity().value(), 7);
    assert!(runtime.next(&stream).is_some());
    assert!(runtime.next(&stream).is_none());

    let bytes = rkyv::to_bytes::<rkyv::rancor::Error>(&stream).expect("archive stream handle");
    let restored = rkyv::from_bytes::<OpenedStream<Event>, rkyv::rancor::Error>(&bytes)
        .expect("restore stream handle");
    assert_eq!(restored.identity().value(), stream.identity().value());
}
