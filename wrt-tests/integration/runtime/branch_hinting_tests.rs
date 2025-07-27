//! Tests for WebAssembly branch hinting and type reflection instructions.
//!
//! These tests verify that br_on_null, br_on_non_null, ref.is_null, 
//! ref.as_non_null, and ref.eq instructions work correctly.

use wrt_error::Result;
use wrt_foundation::{
    types::{Instruction, ValueType, RefType},
    values::{Value, FuncRef, ExternRef},
};
use wrt_instructions::{
    branch_hinting::{BrOnNull, BrOnNonNull, BranchHintOp},
    reference_ops::{RefIsNull, RefAsNonNull, RefEq, ReferenceOp},
    control_ops::ControlOp,
};

/// Test br_on_null instruction with various reference types
#[test]
fn test_br_on_null_instruction() -> Result<()> {
    // Test with null funcref - should branch
    let op = BrOnNull::new(0;
    let result = op.execute(&Value::FuncRef(None))?;
    assert!(result)); // Branch should be taken

    // Test with non-null funcref - should not branch
    let op = BrOnNull::new(1;
    let result = op.execute(&Value::FuncRef(Some(FuncRef { index: 42 })))?;
    assert!(!result)); // Branch should not be taken

    // Test with null externref - should branch
    let op = BrOnNull::new(2;
    let result = op.execute(&Value::ExternRef(None))?;
    assert!(result)); // Branch should be taken

    // Test with non-null externref - should not branch
    let op = BrOnNull::new(3;
    let result = op.execute(&Value::ExternRef(Some(ExternRef { index: 123 })))?;
    assert!(!result)); // Branch should not be taken

    Ok(())
}

/// Test br_on_non_null instruction with various reference types
#[test]
fn test_br_on_non_null_instruction() -> Result<()> {
    // Test with null funcref - should not branch
    let op = BrOnNonNull::new(0;
    let (branch_taken, value) = op.execute(&Value::FuncRef(None))?;
    assert!(!branch_taken)); // Branch should not be taken
    assert!(value.is_none())); // No value kept on stack

    // Test with non-null funcref - should branch
    let op = BrOnNonNull::new(1;
    let ref_value = Value::FuncRef(Some(FuncRef { index: 42 };
    let (branch_taken, value) = op.execute(&ref_value)?;
    assert!(branch_taken)); // Branch should be taken
    assert_eq!(value, Some(ref_value)); // Reference stays on stack

    // Test with null externref - should not branch
    let op = BrOnNonNull::new(2;
    let (branch_taken, value) = op.execute(&Value::ExternRef(None))?;
    assert!(!branch_taken)); // Branch should not be taken
    assert!(value.is_none();

    // Test with non-null externref - should branch
    let op = BrOnNonNull::new(3;
    let ref_value = Value::ExternRef(Some(ExternRef { index: 123 };
    let (branch_taken, value) = op.execute(&ref_value)?;
    assert!(branch_taken)); // Branch should be taken
    assert_eq!(value, Some(ref_value)); // Reference stays on stack

    Ok(())
}

/// Test ref.is_null instruction
#[test]
fn test_ref_is_null_instruction() -> Result<()> {
    let op = RefIsNull::new();

    // Test with null funcref
    let result = op.execute(Value::FuncRef(None))?;
    assert_eq!(result, Value::I32(1)); // Should return 1 (true)

    // Test with non-null funcref
    let result = op.execute(Value::FuncRef(Some(FuncRef { index: 42 })))?;
    assert_eq!(result, Value::I32(0)); // Should return 0 (false)

    // Test with null externref
    let result = op.execute(Value::ExternRef(None))?;
    assert_eq!(result, Value::I32(1)); // Should return 1 (true)

    // Test with non-null externref
    let result = op.execute(Value::ExternRef(Some(ExternRef { index: 123 })))?;
    assert_eq!(result, Value::I32(0)); // Should return 0 (false)

    // Test with non-reference type should error
    let result = op.execute(Value::I32(42;
    assert!(result.is_err();

    Ok(())
}

/// Test ref.as_non_null instruction
#[test]
fn test_ref_as_non_null_instruction() -> Result<()> {
    let op = RefAsNonNull::new();

    // Test with non-null funcref - should pass through
    let input = Value::FuncRef(Some(FuncRef { index: 42 };
    let result = op.execute(input.clone())?;
    assert_eq!(result, input;

    // Test with non-null externref - should pass through
    let input = Value::ExternRef(Some(ExternRef { index: 123 };
    let result = op.execute(input.clone())?;
    assert_eq!(result, input;

    // Test with null funcref - should error
    let result = op.execute(Value::FuncRef(None;
    assert!(result.is_err();

    // Test with null externref - should error
    let result = op.execute(Value::ExternRef(None;
    assert!(result.is_err();

    // Test with non-reference type - should error
    let result = op.execute(Value::I32(42;
    assert!(result.is_err();

    Ok(())
}

/// Test ref.eq instruction
#[test]
fn test_ref_eq_instruction() -> Result<()> {
    let op = RefEq::new();

    // Test null funcref equality
    let result = op.execute(Value::FuncRef(None), Value::FuncRef(None))?;
    assert_eq!(result, Value::I32(1)); // null == null

    // Test null externref equality
    let result = op.execute(Value::ExternRef(None), Value::ExternRef(None))?;
    assert_eq!(result, Value::I32(1)); // null == null

    // Test same funcref equality
    let ref1 = Value::FuncRef(Some(FuncRef { index: 42 };
    let ref2 = Value::FuncRef(Some(FuncRef { index: 42 };
    let result = op.execute(ref1, ref2)?;
    assert_eq!(result, Value::I32(1)); // same index == equal

    // Test different funcref inequality
    let ref1 = Value::FuncRef(Some(FuncRef { index: 42 };
    let ref2 = Value::FuncRef(Some(FuncRef { index: 43 };
    let result = op.execute(ref1, ref2)?;
    assert_eq!(result, Value::I32(0)); // different indices != equal

    // Test null vs non-null inequality
    let ref1 = Value::FuncRef(None;
    let ref2 = Value::FuncRef(Some(FuncRef { index: 42 };
    let result = op.execute(ref1, ref2)?;
    assert_eq!(result, Value::I32(0)); // null != non-null

    // Test different types inequality (even if both null)
    let ref1 = Value::FuncRef(None;
    let ref2 = Value::ExternRef(None;
    let result = op.execute(ref1, ref2)?;
    assert_eq!(result, Value::I32(0)); // funcref != externref

    // Test with non-reference types - should error
    let result = op.execute(Value::I32(42), Value::I32(42;
    assert!(result.is_err();

    Ok(())
}

/// Test branch hinting operations through the unified enum
#[test]
fn test_branch_hint_op_enum() -> Result<()> {
    // Test BrOnNull through enum
    let op = BranchHintOp::BrOnNull(BrOnNull::new(5;
    let (taken, label, value) = op.execute(&Value::FuncRef(None))?;
    assert!(taken);
    assert_eq!(label, Some(5;
    assert!(value.is_none();

    // Test BrOnNonNull through enum
    let op = BranchHintOp::BrOnNonNull(BrOnNonNull::new(10;
    let ref_value = Value::FuncRef(Some(FuncRef { index: 99 };
    let (taken, label, value) = op.execute(&ref_value)?;
    assert!(taken);
    assert_eq!(label, Some(10;
    assert_eq!(value, Some(ref_value;

    Ok(())
}

/// Test error handling for type mismatches
#[test]
fn test_error_handling() {
    // Test br_on_null with non-reference type
    let op = BrOnNull::new(0;
    let result = op.execute(&Value::I32(42;
    assert!(result.is_err();

    // Test br_on_non_null with non-reference type
    let op = BrOnNonNull::new(0;
    let result = op.execute(&Value::I64(123;
    assert!(result.is_err();
}

/// Integration test combining multiple operations
#[test]
fn test_integration_workflow() -> Result<()> {
    // Create some reference values
    let null_func = Value::FuncRef(None;
    let valid_func = Value::FuncRef(Some(FuncRef { index: 1 };
    let null_extern = Value::ExternRef(None;
    let valid_extern = Value::ExternRef(Some(ExternRef { index: 2 };

    // Test workflow: check if reference is null, then conditionally branch
    let is_null_op = RefIsNull::new();
    
    // Check null funcref
    let is_null_result = is_null_op.execute(null_func.clone())?;
    assert_eq!(is_null_result, Value::I32(1;
    
    // If it's null, br_on_null should branch
    let br_on_null_op = BrOnNull::new(1;
    let should_branch = br_on_null_op.execute(&null_func)?;
    assert!(should_branch);

    // Check valid funcref
    let is_null_result = is_null_op.execute(valid_func.clone())?;
    assert_eq!(is_null_result, Value::I32(0;
    
    // If it's not null, br_on_non_null should branch
    let br_on_non_null_op = BrOnNonNull::new(2;
    let (should_branch, kept_value) = br_on_non_null_op.execute(&valid_func)?;
    assert!(should_branch);
    assert_eq!(kept_value, Some(valid_func.clone();

    // Test ref.as_non_null with valid reference
    let as_non_null_op = RefAsNonNull::new();
    let result = as_non_null_op.execute(valid_func.clone())?;
    assert_eq!(result, valid_func;

    // Test ref.eq with same references
    let eq_op = RefEq::new();
    let eq_result = eq_op.execute(valid_func.clone(), valid_func.clone())?;
    assert_eq!(eq_result, Value::I32(1;

    // Test ref.eq with different types
    let eq_result = eq_op.execute(valid_func, valid_extern)?;
    assert_eq!(eq_result, Value::I32(0;

    Ok(())
}

/// Performance test for branch hinting operations
#[test]
#[cfg(feature = "std")]
fn test_performance() -> Result<()> {
    use std::time::Instant;

    let null_ref = Value::FuncRef(None;
    let valid_ref = Value::FuncRef(Some(FuncRef { index: 100 };

    let start = Instant::now);
    
    // Perform many operations to test performance
    for _ in 0..10000 {
        let br_on_null = BrOnNull::new(0;
        let _ = br_on_null.execute(&null_ref)?;
        
        let br_on_non_null = BrOnNonNull::new(1;
        let _ = br_on_non_null.execute(&valid_ref)?;
        
        let is_null = RefIsNull::new();
        let _ = is_null.execute(null_ref.clone())?;
        
        let as_non_null = RefAsNonNull::new();
        let _ = as_non_null.execute(valid_ref.clone())?;
        
        let eq_op = RefEq::new();
        let _ = eq_op.execute(valid_ref.clone(), valid_ref.clone())?;
    }
    
    let duration = start.elapsed);
    println!("10000 branch hinting operations took: {:?}", duration);
    
    // Should complete in reasonable time (less than 100ms on modern hardware)
    assert!(duration.as_millis() < 100);
    
    Ok(())
}

/// Test edge cases and boundary conditions
#[test]
fn test_edge_cases() -> Result<()> {
    // Test with maximum function index
    let max_func = Value::FuncRef(Some(FuncRef { index: u32::MAX };
    let is_null_op = RefIsNull::new();
    let result = is_null_op.execute(max_func.clone())?;
    assert_eq!(result, Value::I32(0)); // Should be non-null

    // Test as_non_null with max index
    let as_non_null_op = RefAsNonNull::new();
    let result = as_non_null_op.execute(max_func.clone())?;
    assert_eq!(result, max_func;

    // Test ref.eq with max indices
    let max_extern = Value::ExternRef(Some(ExternRef { index: u32::MAX };
    let eq_op = RefEq::new();
    let result = eq_op.execute(max_func, max_extern)?;
    assert_eq!(result, Value::I32(0)); // Different types

    Ok(())
}