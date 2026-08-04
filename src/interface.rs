//! Universal roles and pure stream contracts supplied by positional Interfaces.

use std::marker::PhantomData;

/// A message accepted by a component.
///
/// Membership is supplied by an Interface document's input position; component
/// authors do not restate it on the declaration.
pub trait Input {}

/// A message emitted by a component.
///
/// Membership is supplied by an Interface document's output position.
pub trait Output {}

/// A public refusal returned across a component boundary.
///
/// Every refusal is also a standard Rust error. The generated assembly owns its
/// concrete [`std::fmt::Display`] and [`std::error::Error`] implementations.
pub trait Refusal: std::error::Error {}

/// Stable, event-typed identity of one established stream.
///
/// This is an opaque contract value. A component runtime allocates it and owns
/// all registry/queue behavior; `protos` deliberately owns neither.
#[derive(rkyv::Archive, rkyv::Deserialize, rkyv::Serialize, Clone, Debug, Eq, PartialEq)]
pub struct StreamIdentity<Event> {
    value: u64,
    event: PhantomData<fn() -> Event>,
}

// Contract-value exception: construction and observation preserve the opaque
// identity without adding component behavior.
impl<Event> StreamIdentity<Event> {
    /// Construct one runtime-allocated stream identity.
    pub const fn new(value: u64) -> Self {
        Self {
            value,
            event: PhantomData,
        }
    }

    /// The component-local stable identity value.
    pub const fn value(&self) -> u64 {
        self.value
    }
}

/// The direct successful output of a stream initiation.
///
/// The handle carries the typed identity required for later event routing and
/// termination. It is not a grant, receipt, subscription token, or queue.
#[derive(rkyv::Archive, rkyv::Deserialize, rkyv::Serialize, Clone, Debug, Eq, PartialEq)]
pub struct Stream<Event> {
    identity: StreamIdentity<Event>,
}

// Contract-value exception: this only exposes the typed handle's identity.
impl<Event> Stream<Event> {
    /// Construct the direct output handle for one established identity.
    pub const fn new(identity: StreamIdentity<Event>) -> Self {
        Self { identity }
    }

    /// Identity later supplied by the separate termination input.
    pub const fn identity(&self) -> &StreamIdentity<Event> {
        &self.identity
    }
}

/// Signature-only contract for a component that establishes a stream.
///
/// Implementations belong to the generated component/runtime. Successful
/// establishment returns [`Stream`] itself; refusal remains an ordinary typed
/// component failure.
pub trait StreamOpen {
    /// Strict initiation input.
    type Initiation: Input;
    /// Typed event carried by the established stream.
    type Event;
    /// Typed refusal from initiation.
    type InitiationRefusal: Refusal;

    /// Establish one stream from its strict initiation input.
    fn open(
        &mut self,
        initiation: Self::Initiation,
    ) -> Result<Stream<Self::Event>, Self::InitiationRefusal>;
}

/// Signature-only contract for delivering an event through an established stream.
///
/// This contract intentionally does not define closing. Closing is a separate
/// strict transformer input, not a third universal trait or a method on
/// [`Stream`].
pub trait StreamEvent {
    /// Typed event carried by the stream.
    type Event;

    /// Deliver the next event for one established stream, if available.
    fn next(&mut self, stream: &Stream<Self::Event>) -> Option<Self::Event>;
}
