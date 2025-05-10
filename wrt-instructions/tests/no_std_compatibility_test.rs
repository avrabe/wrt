//! Test no_std compatibility for wrt-instructions
//!
//! This file validates that the wrt-instructions crate works correctly in no_std environments.

// For testing in a no_std environment
#![cfg_attr(not(feature = "std"), no_std)]

// External crate imports
#[cfg(all(not(feature = "std"), feature = "alloc"))]
extern crate alloc;

#[cfg(test)]
mod tests {
    // Import necessary types for no_std environment
    #[cfg(all(not(feature = "std"), feature = "alloc"))]
    use alloc::{format, string::String, vec, vec::Vec};

    #[cfg(feature = "std")]
    use std::{string::String, vec, vec::Vec};

    // Import from wrt-instructions
    use wrt_instructions::{
        arithmetic_ops::ArithmeticOp,
        comparison_ops::ComparisonOp,
        control_ops::ControlOp,
        conversion_ops::ConversionOp,
        execution::ExecutionEnvironment,
        instruction_traits::{InstructionExecution, PureInstruction},
        memory_ops::{MemoryLoad, MemoryStore},
        table_ops::TableOp,
        variable_ops::VariableOp,
    };

    // Import from wrt-types
    use wrt_types::{bounded::BoundedVec, safe_memory::SafeStack, types::ValueType, values::Value};

    // Mock execution environment for testing
    struct MockExecutionEnvironment {
        stack: SafeStack<Value>,
    }

    impl MockExecutionEnvironment {
        fn new() -> Self {
            Self { stack: SafeStack::new() }
        }
    }

    impl ExecutionEnvironment for MockExecutionEnvironment {
        fn push(&mut self, value: Value) -> wrt_error::Result<()> {
            self.stack.push(value).map_err(|e| {
                wrt_error::Error::new(
                    wrt_error::ErrorCategory::Runtime,
                    wrt_error::codes::STACK_OVERFLOW,
                    format!("Stack overflow: {}", e),
                )
            })
        }

        fn pop(&mut self) -> wrt_error::Result<Value> {
            self.stack.pop().ok_or_else(|| {
                wrt_error::Error::new(
                    wrt_error::ErrorCategory::Runtime,
                    wrt_error::codes::STACK_UNDERFLOW,
                    "Stack underflow".to_string(),
                )
            })
        }
    }

    #[test]
    fn test_arithmetic_operations() {
        // Test i32 add
        let i32_add = ArithmeticOp::I32Add;
        let mut env = MockExecutionEnvironment::new();

        // Push operands
        env.push(Value::I32(5)).unwrap();
        env.push(Value::I32(3)).unwrap();

        // Execute operation
        i32_add.execute(&mut env).unwrap();

        // Check result
        let result = env.pop().unwrap();
        assert_eq!(result, Value::I32(8));
    }

    #[test]
    fn test_comparison_operations() {
        // Test i32 eq
        let i32_eq = ComparisonOp::I32Eq;
        let mut env = MockExecutionEnvironment::new();

        // Push operands (equal)
        env.push(Value::I32(5)).unwrap();
        env.push(Value::I32(5)).unwrap();

        // Execute operation
        i32_eq.execute(&mut env).unwrap();

        // Check result (should be 1 for true)
        let result = env.pop().unwrap();
        assert_eq!(result, Value::I32(1));

        // Push operands (not equal)
        env.push(Value::I32(5)).unwrap();
        env.push(Value::I32(3)).unwrap();

        // Execute operation
        i32_eq.execute(&mut env).unwrap();

        // Check result (should be 0 for false)
        let result = env.pop().unwrap();
        assert_eq!(result, Value::I32(0));
    }

    #[test]
    fn test_conversion_operations() {
        // Test i32 to f32 conversion
        let i32_to_f32 = ConversionOp::I32TruncF32S;
        let mut env = MockExecutionEnvironment::new();

        // Push operand
        env.push(Value::F32(42.75)).unwrap();

        // Execute operation
        i32_to_f32.execute(&mut env).unwrap();

        // Check result
        let result = env.pop().unwrap();
        assert_eq!(result, Value::I32(42));
    }

    #[test]
    fn test_instruction_traits() {
        // Test pure instruction trait
        let i32_add = ArithmeticOp::I32Add;
        let i64_add = ArithmeticOp::I64Add;

        // Check that instructions can be compared
        assert_ne!(i32_add, i64_add);

        // Test that each instruction implements Debug
        let _ = format!("{:?}", i32_add);
    }
}
