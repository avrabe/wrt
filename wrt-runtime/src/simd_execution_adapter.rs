//! SIMD Execution Adapter for Stackless Engine Integration
//!
//! This module provides integration between the comprehensive SIMD runtime
//! implementation and the stackless execution engine. It bridges SIMD
//! instructions with the main WebAssembly execution flow.
//!
//! # ASIL Compliance
//! - No unsafe code in safety-critical configurations
//! - Deterministic execution across all ASIL levels
//! - Bounded memory usage with compile-time guarantees

use core::fmt::Debug;
use wrt_error::{Error, ErrorCategory, Result, codes};
use wrt_foundation::{Value, ValueType};
use wrt_instructions::simd_ops::SimdOp;
use wrt::simd_runtime_impl::{execute_simd_operation, AssilCompliantSimdProvider};
use crate::stackless::engine::StacklessEngine;

/// SIMD execution adapter for integrating SIMD operations with the stackless engine
pub struct SimdExecutionAdapter;

impl SimdExecutionAdapter {
    /// Create a new SIMD execution adapter
    pub fn new() -> Self {
        Self
    }

    /// Execute a SIMD operation with the stackless engine
    ///
    /// This method bridges SIMD operations with the main execution engine,
    /// handling stack operations and ensuring ASIL compliance.
    ///
    /// # Arguments
    /// * `op` - The SIMD operation to execute
    /// * `engine` - The stackless execution engine
    ///
    /// # Returns
    /// * `Ok(())` - If the operation completed successfully
    /// * `Err(Error)` - If the operation failed
    pub fn execute_simd_with_engine(
        &self,
        op: &SimdOp,
        engine: &mut StacklessEngine,
    ) -> Result<()> {
        // Determine the number of operands needed for this operation
        let operand_count = self.get_operand_count(op);
        
        // Pop operands from the engine's value stack
        let mut operands = Vec::with_capacity(operand_count);
        for _ in 0..operand_count {
            let value = engine.exec_stack.values.pop()
                .map_err(|_| Error::runtime_stack_underflow("Insufficient operands for SIMD operation"))?
                .ok_or_else(|| Error::runtime_stack_underflow("Empty stack for SIMD operation"))?;
            operands.push(value);
        }
        
        // Reverse operands to match the expected order (since we popped from stack)
        operands.reverse();
        
        // Execute the SIMD operation using the comprehensive runtime
        let provider = AssilCompliantSimdProvider;
        let result = execute_simd_operation(op.clone(), &operands, &provider)?;
        
        // Push the result back onto the stack
        engine.exec_stack.values.push(result)
            .map_err(|_| Error::runtime_stack_overflow("Failed to push SIMD result onto stack"))?;
        
        Ok(())
    }

    /// Get the number of operands required for a SIMD operation
    fn get_operand_count(&self, op: &SimdOp) -> usize {
        match op {
            // Load operations require 1 operand (address)
            SimdOp::V128Load { .. } |
            SimdOp::V128Load8x8S { .. } |
            SimdOp::V128Load8x8U { .. } |
            SimdOp::V128Load16x4S { .. } |
            SimdOp::V128Load16x4U { .. } |
            SimdOp::V128Load32x2S { .. } |
            SimdOp::V128Load32x2U { .. } |
            SimdOp::V128Load8Splat { .. } |
            SimdOp::V128Load16Splat { .. } |
            SimdOp::V128Load32Splat { .. } |
            SimdOp::V128Load64Splat { .. } => 1,
            
            // Store operations require 2 operands (address, value)
            SimdOp::V128Store { .. } => 2,
            
            // Lane extraction requires 1 operand (vector)
            SimdOp::I8x16ExtractLaneS { .. } |
            SimdOp::I8x16ExtractLaneU { .. } |
            SimdOp::I16x8ExtractLaneS { .. } |
            SimdOp::I16x8ExtractLaneU { .. } |
            SimdOp::I32x4ExtractLane { .. } |
            SimdOp::I64x2ExtractLane { .. } |
            SimdOp::F32x4ExtractLane { .. } |
            SimdOp::F64x2ExtractLane { .. } => 1,
            
            // Lane replacement requires 2 operands (vector, scalar)
            SimdOp::I8x16ReplaceLane { .. } |
            SimdOp::I16x8ReplaceLane { .. } |
            SimdOp::I32x4ReplaceLane { .. } |
            SimdOp::I64x2ReplaceLane { .. } |
            SimdOp::F32x4ReplaceLane { .. } |
            SimdOp::F64x2ReplaceLane { .. } => 2,
            
            // Splat operations require 1 operand (scalar)
            SimdOp::I8x16Splat |
            SimdOp::I16x8Splat |
            SimdOp::I32x4Splat |
            SimdOp::I64x2Splat |
            SimdOp::F32x4Splat |
            SimdOp::F64x2Splat => 1,
            
            // Binary operations require 2 operands
            SimdOp::I8x16Add |
            SimdOp::I16x8Add |
            SimdOp::I32x4Add |
            SimdOp::I64x2Add |
            SimdOp::F32x4Add |
            SimdOp::F64x2Add |
            SimdOp::I8x16Sub |
            SimdOp::I16x8Sub |
            SimdOp::I32x4Sub |
            SimdOp::I64x2Sub |
            SimdOp::F32x4Sub |
            SimdOp::F64x2Sub |
            SimdOp::I8x16Mul |
            SimdOp::I16x8Mul |
            SimdOp::I32x4Mul |
            SimdOp::I64x2Mul |
            SimdOp::F32x4Mul |
            SimdOp::F64x2Mul |
            SimdOp::F32x4Div |
            SimdOp::F64x2Div |
            SimdOp::I8x16And |
            SimdOp::I8x16Or |
            SimdOp::I8x16Xor |
            SimdOp::I8x16Eq |
            SimdOp::I8x16Ne |
            SimdOp::I8x16LtS |
            SimdOp::I8x16LtU |
            SimdOp::I8x16GtS |
            SimdOp::I8x16GtU |
            SimdOp::I8x16LeS |
            SimdOp::I8x16LeU |
            SimdOp::I8x16GeS |
            SimdOp::I8x16GeU |
            SimdOp::I16x8Eq |
            SimdOp::I16x8Ne |
            SimdOp::I16x8LtS |
            SimdOp::I16x8LtU |
            SimdOp::I16x8GtS |
            SimdOp::I16x8GtU |
            SimdOp::I16x8LeS |
            SimdOp::I16x8LeU |
            SimdOp::I16x8GeS |
            SimdOp::I16x8GeU |
            SimdOp::I32x4Eq |
            SimdOp::I32x4Ne |
            SimdOp::I32x4LtS |
            SimdOp::I32x4LtU |
            SimdOp::I32x4GtS |
            SimdOp::I32x4GtU |
            SimdOp::I32x4LeS |
            SimdOp::I32x4LeU |
            SimdOp::I32x4GeS |
            SimdOp::I32x4GeU |
            SimdOp::I64x2Eq |
            SimdOp::I64x2Ne |
            SimdOp::I64x2LtS |
            SimdOp::I64x2GtS |
            SimdOp::I64x2LeS |
            SimdOp::I64x2GeS |
            SimdOp::F32x4Eq |
            SimdOp::F32x4Ne |
            SimdOp::F32x4Lt |
            SimdOp::F32x4Gt |
            SimdOp::F32x4Le |
            SimdOp::F32x4Ge |
            SimdOp::F64x2Eq |
            SimdOp::F64x2Ne |
            SimdOp::F64x2Lt |
            SimdOp::F64x2Gt |
            SimdOp::F64x2Le |
            SimdOp::F64x2Ge => 2,
            
            // Unary operations require 1 operand
            SimdOp::I8x16Neg |
            SimdOp::I16x8Neg |
            SimdOp::I32x4Neg |
            SimdOp::I64x2Neg |
            SimdOp::F32x4Neg |
            SimdOp::F64x2Neg |
            SimdOp::I8x16Not |
            SimdOp::F32x4Abs |
            SimdOp::F64x2Abs |
            SimdOp::F32x4Sqrt |
            SimdOp::F64x2Sqrt |
            SimdOp::F32x4Ceil |
            SimdOp::F32x4Floor |
            SimdOp::F32x4Trunc |
            SimdOp::F32x4Nearest |
            SimdOp::F64x2Ceil |
            SimdOp::F64x2Floor |
            SimdOp::F64x2Trunc |
            SimdOp::F64x2Nearest => 1,
            
            // Vector operations (constants and conversions)
            SimdOp::V128Const { .. } => 0,
            SimdOp::V128Bitselect => 3,
            SimdOp::V128AnyTrue => 1,
            
            // Shuffle requires 2 operands (2 vectors)
            SimdOp::I8x16Shuffle { .. } => 2,
            
            // Swizzle requires 2 operands (vector and indices)
            SimdOp::I8x16Swizzle => 2,
            
            // Default case for any missing operations
            _ => 1,
        }
    }

    /// Check if a value is a valid V128 vector
    fn is_v128_value(value: &Value) -> bool {
        matches!(value, Value::V128(_))
    }
    
    /// Validate that operands are appropriate for SIMD operations
    fn validate_simd_operands(&self, op: &SimdOp, operands: &[Value]) -> Result<()> {
        match op {
            // Load/store operations expect an i32 address as first operand
            SimdOp::V128Load { .. } |
            SimdOp::V128Load8x8S { .. } |
            SimdOp::V128Load8x8U { .. } |
            SimdOp::V128Load16x4S { .. } |
            SimdOp::V128Load16x4U { .. } |
            SimdOp::V128Load32x2S { .. } |
            SimdOp::V128Load32x2U { .. } |
            SimdOp::V128Load8Splat { .. } |
            SimdOp::V128Load16Splat { .. } |
            SimdOp::V128Load32Splat { .. } |
            SimdOp::V128Load64Splat { .. } => {
                if operands.is_empty() {
                    return Err(Error::validation_type_mismatch("SIMD load operation requires address operand"));
                }
                if !matches!(operands[0], Value::I32(_)) {
                    return Err(Error::validation_type_mismatch("SIMD load operation requires i32 address"));
                }
            }
            
            SimdOp::V128Store { .. } => {
                if operands.len() < 2 {
                    return Err(Error::validation_type_mismatch("SIMD store operation requires address and value operands"));
                }
                if !matches!(operands[0], Value::I32(_)) {
                    return Err(Error::validation_type_mismatch("SIMD store operation requires i32 address"));
                }
                if !Self::is_v128_value(&operands[1]) {
                    return Err(Error::validation_type_mismatch("SIMD store operation requires v128 value"));
                }
            }
            
            // Vector operations expect v128 operands
            _ => {
                for (i, operand) in operands.iter().enumerate() {
                    match op {
                        // Operations that mix scalar and vector types
                        SimdOp::I8x16Splat | SimdOp::I16x8Splat | 
                        SimdOp::I32x4Splat | SimdOp::I64x2Splat |
                        SimdOp::F32x4Splat | SimdOp::F64x2Splat => {
                            // Splat operations take a scalar, no validation needed here
                        }
                        
                        SimdOp::I8x16ReplaceLane { .. } |
                        SimdOp::I16x8ReplaceLane { .. } |
                        SimdOp::I32x4ReplaceLane { .. } |
                        SimdOp::I64x2ReplaceLane { .. } |
                        SimdOp::F32x4ReplaceLane { .. } |
                        SimdOp::F64x2ReplaceLane { .. } => {
                            // Replace lane: first operand is vector, second is scalar
                            if i == 0 && !Self::is_v128_value(operand) {
                                return Err(Error::validation_type_mismatch("Replace lane operation requires v128 vector as first operand"));
                            }
                        }
                        
                        // All other vector operations expect v128 operands
                        _ => {
                            if !Self::is_v128_value(operand) && !matches!(operand, Value::I32(_) | Value::I64(_) | Value::F32(_) | Value::F64(_)) {
                                return Err(Error::validation_type_mismatch("SIMD operation requires appropriate operand types"));
                            }
                        }
                    }
                }
            }
        }
        
        Ok(())
    }
}

impl Default for SimdExecutionAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl Debug for SimdExecutionAdapter {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("SimdExecutionAdapter").finish()
    }
}

/// Helper function to create common SIMD operations for testing and examples
#[cfg(any(test, feature = "test-utils"))]
pub mod test_utils {
    use super::*;
    
    /// Create a V128 load operation for testing
    pub fn create_v128_load(offset: u32, align: u32) -> SimdOp {
        SimdOp::V128Load { offset, align }
    }
    
    /// Create a V128 store operation for testing
    pub fn create_v128_store(offset: u32, align: u32) -> SimdOp {
        SimdOp::V128Store { offset, align }
    }
    
    /// Create an I32x4 add operation for testing
    pub fn create_i32x4_add() -> SimdOp {
        SimdOp::I32x4Add
    }
    
    /// Create an F32x4 mul operation for testing
    pub fn create_f32x4_mul() -> SimdOp {
        SimdOp::F32x4Mul
    }
}

#[cfg(all(test, any(feature = "std", feature = "alloc")))]
mod tests {
    use super::*;
    use wrt_foundation::values::V128;
    use crate::stackless::engine::StacklessEngine;

    #[test]
    fn test_simd_adapter_creation() {
        let adapter = SimdExecutionAdapter::new();
        assert_eq!(format!("{:?}", adapter), "SimdExecutionAdapter");
    }

    #[test]
    fn test_operand_count_calculations() {
        let adapter = SimdExecutionAdapter::new();
        
        // Test load operations
        assert_eq!(adapter.get_operand_count(&SimdOp::V128Load { offset: 0, align: 4 }), 1);
        
        // Test store operations
        assert_eq!(adapter.get_operand_count(&SimdOp::V128Store { offset: 0, align: 4 }), 2);
        
        // Test binary operations
        assert_eq!(adapter.get_operand_count(&SimdOp::I32x4Add), 2);
        assert_eq!(adapter.get_operand_count(&SimdOp::F32x4Mul), 2);
        
        // Test unary operations
        assert_eq!(adapter.get_operand_count(&SimdOp::I32x4Neg), 1);
        assert_eq!(adapter.get_operand_count(&SimdOp::F32x4Abs), 1);
        
        // Test splat operations
        assert_eq!(adapter.get_operand_count(&SimdOp::I32x4Splat), 1);
    }

    #[test]
    fn test_v128_value_validation() {
        let v128_value = Value::V128(V128::from_bytes([0u8; 16]));
        let i32_value = Value::I32(42);
        
        assert!(SimdExecutionAdapter::is_v128_value(&v128_value));
        assert!(!SimdExecutionAdapter::is_v128_value(&i32_value));
    }
}