// WRT - wrt-foundation
// Module: Type Conversion Utilities
// SW-REQ-ID: REQ_018
//
// Copyright (c) 2025 Ralf Anton Beier
// Licensed under the MIT license.
// SPDX-License-Identifier: MIT

//! Type conversion utilities for WebAssembly types
//!
//! This module provides functions for converting between different
//! WebAssembly type representations, such as between ValType and BinaryType.

// Remove Vec-related imports as they are no longer needed at the top level or
// directly in func_type module #[cfg(all(feature = "alloc", not(feature =
// "std")))] extern crate alloc;

// #[cfg(feature = "alloc")]
// use alloc::vec::Vec; // This was for the module scope, func_type::create used
// its own.

use wrt_error::{codes, Error, Result};

use crate::{
    component_value::{ComponentValue, ValType},
    safe_memory::NoStdProvider,
    types::DEFAULT_FUNC_TYPE_PROVIDER_CAPACITY,
    values::Value,
    BlockType, FuncType, MemoryProvider, RefType, ValueType as CoreValueType,
};

/// Convert `RefType` to `ValueType`
///
/// Provides a standard way to convert between reference types
/// and value types across all crates.
#[must_use]
pub fn ref_type_to_val_type(ref_type: RefType) -> CoreValueType {
    match ref_type {
        RefType::Funcref => CoreValueType::FuncRef,
        RefType::Externref => CoreValueType::ExternRef,
    }
}

/// Convert `ValueType` to `RefType`
///
/// Provides a standard way to convert between value types
/// and reference types across all crates.
pub fn val_type_to_ref_type(val_type: CoreValueType) -> Result<RefType> {
    match val_type {
        CoreValueType::FuncRef => Ok(RefType::Funcref),
        CoreValueType::ExternRef => Ok(RefType::Externref),
        _ => Err(Error::new(
            wrt_error::ErrorCategory::Type,
            codes::CONVERSION_ERROR,
            "Invalid reference type",
        )),
    }
}

/// Block type utilities for converting between different representations
pub mod block_type {
    use super::BlockType;

    /// Convert a `FormatBlockType` (from wrt-format) to `BlockType`
    ///
    /// This utility makes it easier to work with block types across
    /// different modules without duplicating conversion logic.
    pub fn from_format_block_type(format_block_type: &impl ConvertToBlockType) -> BlockType {
        format_block_type.to_block_type()
    }

    /// Trait for types that can be converted to `BlockType`
    ///
    /// This trait allows for standardized conversion from different
    /// representations of block types to the core `BlockType` enum.
    pub trait ConvertToBlockType {
        /// Convert to `BlockType`
        fn to_block_type(&self) -> BlockType;
    }
}

/// `FuncType` utilities for working with function types consistently
pub mod func_type {
    // Remove Vec import, as params and results will be slices
    // #[cfg(not(feature = "std"))]
    // use alloc::vec::Vec;

    use wrt_error::Result;

    use super::{CoreValueType as ValueType, FuncType};
    use crate::{
        safe_memory::NoStdProvider, types::DEFAULT_FUNC_TYPE_PROVIDER_CAPACITY, MemoryProvider,
    };

    /// Verify that a function type conforms to WebAssembly constraints
    ///
    /// This is a utility function that can be used by any crate that
    /// needs to validate a function type.
    pub fn verify<P: MemoryProvider + Default + Clone + core::fmt::Debug + PartialEq + Eq>(
        func_type: &FuncType<P>,
    ) -> Result<()> {
        func_type.verify()
    }

    /// Create a function type from parameters and results slices
    ///
    /// This utility function ensures consistent creation of function types
    /// with proper validation.
    pub fn create<P: MemoryProvider + Default + Clone + core::fmt::Debug + PartialEq + Eq>(
        provider: P,
        params: &[ValueType],
        results: &[ValueType],
    ) -> Result<FuncType<P>> {
        let func_type = FuncType::new(provider, params.iter().copied(), results.iter().copied())?;
        verify(&func_type)?;
        Ok(func_type)
    }
}

#[cfg(test)]
mod tests {
    // Remove alloc::vec import for tests
    // #[cfg(not(feature = "std"))]
    // use alloc::vec;

    use wrt_error::Result;

    use super::*;
    use crate::{
        safe_memory::{NoStdProvider, DEFAULT_MEMORY_PROVIDER_CAPACITY},
        types::DEFAULT_FUNC_TYPE_PROVIDER_CAPACITY,
        values::Value,
    };

    #[test]
    fn test_val_type_conversions() {
        // Test binary conversions (now directly from ValueType)
        assert_eq!(CoreValueType::I32.to_binary(), 0x7F);
        assert_eq!(CoreValueType::I64.to_binary(), 0x7E);
        assert_eq!(CoreValueType::F32.to_binary(), 0x7D);
        assert_eq!(CoreValueType::F64.to_binary(), 0x7C);
        assert_eq!(CoreValueType::V128.to_binary(), 0x7B); // Added for V128
        assert_eq!(CoreValueType::FuncRef.to_binary(), 0x70);
        assert_eq!(CoreValueType::ExternRef.to_binary(), 0x6F);

        // Test binary to val type conversions (now directly from ValueType)
        assert_eq!(CoreValueType::from_binary(0x7F).unwrap(), CoreValueType::I32);
        assert_eq!(CoreValueType::from_binary(0x7E).unwrap(), CoreValueType::I64);
        assert_eq!(CoreValueType::from_binary(0x7D).unwrap(), CoreValueType::F32);
        assert_eq!(CoreValueType::from_binary(0x7C).unwrap(), CoreValueType::F64);
        assert_eq!(CoreValueType::from_binary(0x7B).unwrap(), CoreValueType::V128); // Added for V128
        assert_eq!(CoreValueType::from_binary(0x70).unwrap(), CoreValueType::FuncRef);
        assert_eq!(CoreValueType::from_binary(0x6F).unwrap(), CoreValueType::ExternRef);
        assert!(CoreValueType::from_binary(0x00).is_err());
    }

    #[test]
    fn test_ref_type_conversions() {
        // Test RefType to ValueType conversions
        assert_eq!(ref_type_to_val_type(RefType::Funcref), CoreValueType::FuncRef);
        assert_eq!(ref_type_to_val_type(RefType::Externref), CoreValueType::ExternRef);

        // Test ValueType to RefType conversions
        assert_eq!(val_type_to_ref_type(CoreValueType::FuncRef).unwrap(), RefType::Funcref);
        assert_eq!(val_type_to_ref_type(CoreValueType::ExternRef).unwrap(), RefType::Externref);
        assert!(val_type_to_ref_type(CoreValueType::I32).is_err());
        assert!(val_type_to_ref_type(CoreValueType::I64).is_err());
        assert!(val_type_to_ref_type(CoreValueType::F32).is_err());
        assert!(val_type_to_ref_type(CoreValueType::F64).is_err());
    }

    #[test]
    fn test_func_type_creation() {
        // Test valid function type creation using slices from arrays
        let params: [CoreValueType; 2] = [CoreValueType::I32, CoreValueType::I64];
        let results: [CoreValueType; 1] = [CoreValueType::F32];
        let provider = NoStdProvider::<DEFAULT_FUNC_TYPE_PROVIDER_CAPACITY>::default();
        let func_type = func_type::create(provider.clone(), &params, &results).unwrap();

        assert_eq!(func_type.params(), &[CoreValueType::I32, CoreValueType::I64]);
        assert_eq!(func_type.results(), &[CoreValueType::F32]);

        // Verification should pass
        assert!(func_type::verify(&func_type).is_ok());

        // Test creation with empty params and results
        let empty_params: [CoreValueType; 0] = [];
        let empty_results: [CoreValueType; 0] = [];
        let func_type_empty = func_type::create(provider, &empty_params, &empty_results).unwrap();
        assert_eq!(func_type_empty.params(), &[]);
        assert_eq!(func_type_empty.results(), &[]);
        assert!(func_type::verify(&func_type_empty).is_ok());

        // Example of a type that might exceed max (depending on
        // MAX_PARAMS_IN_FUNC_TYPE) This test might need adjustment
        // based on actual MAX_PARAMS_IN_FUNC_TYPE value For now, let's
        // assume it's small for demonstration if we wanted to test failure.
        // let too_many_params: [CoreValueType; 260] = [CoreValueType::I32;
        // 260]; // Example if MAX is 256 assert!(func_type::create(&
        // too_many_params, &empty_results).is_err());
    }
}
