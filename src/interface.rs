//! Universal roles supplied by positional Interface sections.

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
