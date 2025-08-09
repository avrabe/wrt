//! Stackless WebAssembly execution engine
//!
//! This module implements a stackless version of the WebAssembly execution
//! engine that doesn't rely on the host language's call stack, making it
//! suitable for environments with limited stack space and for no_std contexts.
//!
//! The stackless engine uses a state machine approach to track execution state
//! and allows for pausing and resuming execution at any point.

pub mod engine;
pub mod extensions;
mod frame;

#[cfg(feature = "std")]
pub mod tail_call;

#[cfg(test)]
mod engine_tests;

pub use engine::{
    StacklessCallbackRegistry, StacklessEngine, StacklessExecutionState, StacklessStack,
};
pub use frame::StacklessFrame;
