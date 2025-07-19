//! WebAssembly Component Model runtime implementation.
//!
//! This module provides runtime support for the Component Model, including
//! instantiation, linking, and execution of components.

pub mod instantiate;
pub mod instantiation_types;

pub use instantiate::{
    ComponentInstantiator, InstantiationContext, InstantiationResult,
    CoreModuleInstantiator, LinkingError,
};
pub use instantiation_types::*;