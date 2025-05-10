// WRT - wrt-types
// Module: Type Conversion Utilities
// SW-REQ-ID: REQ_WASM_CORE_004 (Example: Relates to type system consistency)
//
// Copyright (c) 2024 Ralf Anton Beier
// Licensed under the MIT license.
// SPDX-License-Identifier: MIT

//! Type conversion utilities for WebAssembly types
//!
//! This module provides functions for converting between different
//! WebAssembly type representations, such as between ValType and BinaryType.

#[cfg(feature = "alloc")]
extern crate alloc;

use crate::{BlockType, FuncType, RefType, ValueType};
use wrt_error::{codes, Error, Result};

/// Convert RefType to ValueType
///
/// Provides a standard way to convert between reference types
/// and value types across all crates.
pub fn ref_type_to_val_type(ref_type: RefType) -> ValueType {
    match ref_type {
        RefType::Funcref => ValueType::FuncRef,
        RefType::Externref => ValueType::ExternRef,
    }
}

/// Convert ValueType to RefType
///
/// Provides a standard way to convert between value types
/// and reference types across all crates.
pub fn val_type_to_ref_type(val_type: ValueType) -> Result<RefType> {
    match val_type {
        ValueType::FuncRef => Ok(RefType::Funcref),
        ValueType::ExternRef => Ok(RefType::Externref),
        _ => Err(Error::new(
            wrt_error::ErrorCategory::Type,
            codes::CONVERSION_ERROR,
            "Invalid reference type",
        )),
    }
}

/// Block type utilities for converting between different representations
pub mod block_type {
    use super::*;

    /// Convert a FormatBlockType (from wrt-format) to BlockType
    ///
    /// This utility makes it easier to work with block types across
    /// different modules without duplicating conversion logic.
    pub fn from_format_block_type(format_block_type: &impl ConvertToBlockType) -> BlockType {
        format_block_type.to_block_type()
    }

    /// Trait for types that can be converted to BlockType
    ///
    /// This trait allows for standardized conversion from different
    /// representations of block types to the core BlockType enum.
    pub trait ConvertToBlockType {
        /// Convert to BlockType
        fn to_block_type(&self) -> BlockType;
    }
}

/// FuncType utilities for working with function types consistently
pub mod func_type {
    use super::*;
    use crate::Result;

    #[cfg(not(feature = "std"))]
    use alloc::vec::Vec;

    /// Verify that a function type conforms to WebAssembly constraints
    ///
    /// This is a utility function that can be used by any crate that
    /// needs to validate a function type.
    pub fn verify(func_type: &FuncType) -> Result<()> {
        func_type.verify()
    }

    /// Create a function type from parameters and results
    ///
    /// This utility function ensures consistent creation of function types
    /// with proper validation.
    pub fn create(params: Vec<ValueType>, results: Vec<ValueType>) -> Result<FuncType> {
        let func_type = FuncType::new(params, results)?;
        verify(&func_type)?;
        Ok(func_type)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(not(feature = "std"))]
    use alloc::vec;

    #[test]
    fn test_val_type_conversions() {
        // Test binary conversions (now directly from ValueType)
        assert_eq!(ValueType::I32.to_binary(), 0x7F);
        assert_eq!(ValueType::I64.to_binary(), 0x7E);
        assert_eq!(ValueType::F32.to_binary(), 0x7D);
        assert_eq!(ValueType::F64.to_binary(), 0x7C);
        assert_eq!(ValueType::V128.to_binary(), 0x7B); // Added for V128
        assert_eq!(ValueType::FuncRef.to_binary(), 0x70);
        assert_eq!(ValueType::ExternRef.to_binary(), 0x6F);

        // Test binary to val type conversions (now directly from ValueType)
        assert_eq!(ValueType::from_binary(0x7F).unwrap(), ValueType::I32);
        assert_eq!(ValueType::from_binary(0x7E).unwrap(), ValueType::I64);
        assert_eq!(ValueType::from_binary(0x7D).unwrap(), ValueType::F32);
        assert_eq!(ValueType::from_binary(0x7C).unwrap(), ValueType::F64);
        assert_eq!(ValueType::from_binary(0x7B).unwrap(), ValueType::V128); // Added for V128
        assert_eq!(ValueType::from_binary(0x70).unwrap(), ValueType::FuncRef);
        assert_eq!(ValueType::from_binary(0x6F).unwrap(), ValueType::ExternRef);
        assert!(ValueType::from_binary(0x00).is_err());
    }

    #[test]
    fn test_ref_type_conversions() {
        // Test RefType to ValueType conversions
        assert_eq!(ref_type_to_val_type(RefType::Funcref), ValueType::FuncRef);
        assert_eq!(ref_type_to_val_type(RefType::Externref), ValueType::ExternRef);

        // Test ValueType to RefType conversions
        assert_eq!(val_type_to_ref_type(ValueType::FuncRef).unwrap(), RefType::Funcref);
        assert_eq!(val_type_to_ref_type(ValueType::ExternRef).unwrap(), RefType::Externref);
        assert!(val_type_to_ref_type(ValueType::I32).is_err());
        assert!(val_type_to_ref_type(ValueType::I64).is_err());
        assert!(val_type_to_ref_type(ValueType::F32).is_err());
        assert!(val_type_to_ref_type(ValueType::F64).is_err());
    }

    #[test]
    fn test_func_type_creation() {
        // Test valid function type creation
        let func_type =
            func_type::create(vec![ValueType::I32, ValueType::I64], vec![ValueType::F32]).unwrap();

        assert_eq!(func_type.params, vec![ValueType::I32, ValueType::I64]);
        assert_eq!(func_type.results, vec![ValueType::F32]);

        // Verification should pass
        assert!(func_type::verify(&func_type).is_ok());
    }
}
