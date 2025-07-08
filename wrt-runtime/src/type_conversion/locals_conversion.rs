//! Type conversion utilities for local variables
//!
//! This module provides conversion between different representations of local variables
//! used in the WRT execution pipeline.

use wrt_foundation::{
    types::{LocalEntry, ValueType},
    bounded::BoundedVec,
    safe_managed_alloc,
    budget_aware_provider::CrateId,
    MemoryProvider,
};
use wrt_error::Result;

#[cfg(any(feature = "std", feature = "alloc"))]
extern crate alloc;
#[cfg(any(feature = "std", feature = "alloc"))]
use alloc::vec::Vec;

use crate::bounded_runtime_infra::{RuntimeProvider, create_runtime_provider};

/// Convert a flat Vec<ValueType> to a BoundedVec<LocalEntry> by grouping consecutive
/// types of the same kind into LocalEntry structs with count and type.
pub fn convert_locals_to_bounded(
    locals: &[ValueType]
) -> Result<BoundedVec<LocalEntry, 64, RuntimeProvider>> {
    let provider = create_runtime_provider()?;
    
    let mut bounded_locals = BoundedVec::new(provider)?;
    
    if locals.is_empty() {
        return Ok(bounded_locals);
    }
    
    // Group consecutive locals of the same type
    let mut current_type = locals[0];
    let mut current_count = 1u32;
    
    for &local_type in &locals[1..] {
        if local_type == current_type {
            current_count += 1;
        } else {
            // Add the accumulated group
            bounded_locals.push(LocalEntry {
                count: current_count,
                value_type: current_type,
            })?;
            
            // Start new group
            current_type = local_type;
            current_count = 1;
        }
    }
    
    // Add the final group
    bounded_locals.push(LocalEntry {
        count: current_count,
        value_type: current_type,
    })?;
    
    Ok(bounded_locals)
}

/// Convert a BoundedVec<LocalEntry> back to a flat Vec<ValueType>
/// This is useful for compatibility with APIs that expect the flat representation
#[cfg(any(feature = "std", feature = "alloc"))]
pub fn expand_locals_to_flat(
    bounded_locals: &BoundedVec<LocalEntry, 64, RuntimeProvider>
) -> Result<Vec<ValueType>> {
    let mut flat_locals = Vec::new();
    
    for local_entry in bounded_locals.iter() {
        for _ in 0..local_entry.count {
            flat_locals.push(local_entry.value_type);
        }
    }
    
    Ok(flat_locals)
}

#[cfg(test)]
mod tests {
    use super::*;
    use wrt_foundation::types::ValueType;
    
    #[test]
    fn test_convert_empty_locals() {
        let locals = Vec::new();
        let result = convert_locals_to_bounded(&locals).unwrap();
        assert_eq!(result.len(), 0);
    }
    
    #[test] 
    fn test_convert_single_type_group() {
        let locals = vec![ValueType::I32, ValueType::I32, ValueType::I32];
        let result = convert_locals_to_bounded(&locals).unwrap();
        
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].count, 3);
        assert_eq!(result[0].value_type, ValueType::I32);
    }
    
    #[test]
    fn test_convert_multiple_type_groups() {
        let locals = vec![
            ValueType::I32, ValueType::I32, 
            ValueType::F64, 
            ValueType::I32
        ];
        let result = convert_locals_to_bounded(&locals).unwrap();
        
        assert_eq!(result.len(), 3);
        
        assert_eq!(result[0].count, 2);
        assert_eq!(result[0].value_type, ValueType::I32);
        
        assert_eq!(result[1].count, 1);
        assert_eq!(result[1].value_type, ValueType::F64);
        
        assert_eq!(result[2].count, 1);
        assert_eq!(result[2].value_type, ValueType::I32);
    }
    
    #[test]
    fn test_roundtrip_conversion() {
        let original = vec![
            ValueType::I32, ValueType::I32, 
            ValueType::F64, ValueType::F64, ValueType::F64,
            ValueType::I64
        ];
        
        let bounded = convert_locals_to_bounded(&original).unwrap();
        let expanded = expand_locals_to_flat(&bounded).unwrap();
        
        assert_eq!(original, expanded);
    }
}