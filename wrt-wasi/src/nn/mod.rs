//! WASI Neural Network (WASI-NN) Implementation
//!
//! This module provides neural network inference capabilities for WebAssembly
//! modules through the WASI-NN interface. It is designed to be preview-agnostic,
//! working with both WASI Preview2 (synchronous) and Preview3 (asynchronous).
//!
//! The implementation uses WRT's capability-based system to provide different
//! safety guarantees based on the verification level, supporting various safety
//! standards including ASIL, DO-178C, IEC 62304, and others.

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(not(feature = "std"))]
extern crate alloc;

use crate::prelude::*;
use std::sync::{Mutex, OnceLock};

// Core modules
pub mod capabilities;
pub mod backend;
pub mod tensor;
pub mod graph;
pub mod execution;
pub mod wit_types;
pub mod monitoring;
mod sha256;

// ASIL compliance verification
#[cfg(kani)]
mod formal_verification;

// Test modules
#[cfg(test)]
mod tests;

// Backend implementations
#[cfg(feature = "tract")]
pub mod tract_backend;

// Preview-specific bridges
#[cfg(feature = "nn-preview2")]
pub mod sync_bridge;

#[cfg(feature = "nn-preview3")]
pub mod async_bridge;

// Re-export key types
pub use capabilities::{NeuralNetworkCapability, NNOperation, NNResourceLimits, ModelFormat, VerificationLevel, NNVerificationLevel, ResourceUsageStats};
pub use backend::{
    NeuralNetworkBackend, ModelCapability, TensorCapability, ComputeCapable, DynBackend,
    BackendProvider, initialize_backends, get_backend_registry,
};
pub use tensor::{Tensor, TensorType, TensorDimensions, TensorBuilder};
pub use graph::{
    Graph, GraphEncoding, ExecutionTarget, initialize_graph_store, get_graph_store,
};
pub use execution::{
    ExecutionContext, ComputeError, initialize_context_store, get_context_store,
    execute_inference,
};
pub use wit_types::{ErrorCode, WitTypeConversion};

// Preview-specific re-exports
#[cfg(all(feature = "nn-preview2", feature = "std"))]
pub use sync_bridge::{nn_load, nn_init_execution_context, nn_set_input, nn_compute, nn_get_output};

#[cfg(feature = "nn-preview3")]
pub use async_bridge::{nn_load_async, nn_init_execution_context_async, nn_set_input_async, nn_compute_async, nn_get_output_async};

// Test modules are defined inline below

// Global capability storage using Mutex for thread safety (ASIL-compliant)
static NN_CAPABILITY: Mutex<Option<Box<dyn NeuralNetworkCapability>>> = Mutex::new(None);

/// Initialize the WASI-NN subsystem with the given capability
pub fn initialize_nn(capability: Box<dyn NeuralNetworkCapability>) -> Result<()> {
    // Validate capability before setting
    let verification_level = capability.verification_level();
    let resource_limits = capability.resource_limits();
    
    // Validate resource limits are reasonable
    if resource_limits.max_tensor_memory == 0 {
        return Err(Error::wasi_invalid_argument("Capability has invalid memory limits"));
    }
    if resource_limits.max_concurrent_models == 0 {
        return Err(Error::wasi_invalid_argument("Capability has invalid model limits"));
    }
    if resource_limits.max_concurrent_contexts == 0 {
        return Err(Error::wasi_invalid_argument("Capability has invalid context limits"));
    }
    
    let mut guard = NN_CAPABILITY.lock()
        .map_err(|_| Error::wasi_runtime_error("Failed to acquire capability lock"))?;
    
    if guard.is_some() {
        return Err(Error::wasi_capability_unavailable("Neural network capability already initialized"));
    }
    
    *guard = Some(capability);
    Ok(())
}

/// Get the current neural network capability  
/// Note: This returns a temporary guard that must be used carefully
pub fn get_nn_capability() -> Result<std::sync::MutexGuard<'static, Option<Box<dyn NeuralNetworkCapability>>>> {
    NN_CAPABILITY.lock()
        .map_err(|_| Error::wasi_runtime_error("Failed to acquire capability lock"))
}

/// Helper function to execute an operation with the neural network capability
pub fn with_nn_capability<F, R>(f: F) -> Result<R>
where
    F: FnOnce(&dyn NeuralNetworkCapability) -> Result<R>,
{
    let guard = get_nn_capability()?;
    let capability = guard.as_ref()
        .ok_or_else(|| Error::wasi_capability_unavailable("Neural network capability not initialized"))?;
    f(capability.as_ref())
}


/// WASI-NN version information
pub const WASI_NN_VERSION: &str = "0.2.0";

/// Check if WASI-NN is available with the current configuration
pub fn is_nn_available() -> bool {
    get_nn_capability().is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_wasi_nn_version() {
        assert_eq!(WASI_NN_VERSION, "0.2.0");
    }
    
    #[test]
    fn test_nn_not_available_by_default() {
        assert!(!is_nn_available());
    }
}