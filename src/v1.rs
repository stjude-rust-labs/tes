//! Facilities related to v1.x of the specification.

#[cfg(feature = "client")]
pub mod client;

#[cfg(feature = "client")]
pub use client::Client;

#[cfg(feature = "types")]
pub mod types;
