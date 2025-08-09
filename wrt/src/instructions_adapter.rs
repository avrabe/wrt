// Copyright 2024 The WRT Project Authors

//! Instructions adapter for wrt
//!
//! This adapter bridges between `wrt` execution context and `wrt-instructions`
//! execution context. It provides the necessary adapters to use implementations
//! from `wrt-instructions` in the wrt runtime.

/// Re-export instruction types and traits from wrt-instructions
pub use wrt_instructions::{
    aggregate_ops::{
        AggregateOp,
        AggregateOperations,
    },
    behavior::{
        ControlFlow,
        ControlFlowBehavior,
        FrameBehavior,
        InstructionExecutor,
        StackBehavior,
    },
    calls::CallInstruction,
    control::ControlInstruction,
    execution::{
        ExecutionContext as InstructionExecutionContext,
        PureExecutionContext,
    },
    memory_ops::{
        MemoryArg,
        MemoryLoad,
        MemoryOperations,
        MemoryStore,
    },
    numeric::NumericInstruction,
    simd_ops::{
        SimdContext,
        SimdExecutionContext,
        SimdInstruction,
        SimdOp,
    },
    Instruction,
    InstructionExecutable,
};
use wrt_runtime::stackless::{
    StacklessEngine,
    StacklessFrame,
};

use crate::prelude::*;

/// Comprehensive execution context trait that combines stack and memory
/// operations
///
/// This trait provides the interface that the WRT execution adapter implements
/// to bridge between the wrt runtime and the wrt-instructions implementations.
pub trait ExecutionContext {
    /// Push a value onto the stack
    fn push_value(&mut self, value: Value) -> Result<()>;

    /// Pop a value from the stack
    fn pop_value(&mut self) -> Result<Value>;

    /// Pop a value with type checking
    fn pop_value_expected(&mut self, expected_type: ValueType) -> Result<Value>;

    /// Get memory size in pages
    fn memory_size(&mut self, memory_idx: u32) -> Result<u32>;

    /// Grow memory by the specified number of pages
    fn memory_grow(&mut self, memory_idx: u32, pages: u32) -> Result<u32>;

    /// Read bytes from memory
    fn memory_read(&mut self, memory_idx: u32, offset: u32, bytes: &mut [u8]) -> Result<()>;

    /// Write bytes to memory
    fn memory_write(&mut self, memory_idx: u32, offset: u32, bytes: &[u8]) -> Result<()>;
}

#[cfg(feature = "platform")]
mod simd_runtime_impl;

#[cfg(feature = "platform")]
use wrt_platform::simd::SimdRuntime;

/// Execution context adapter for instructions
///
/// This adapter implements the ExecutionContext trait from wrt-instructions,
/// allowing the wrt runtime to execute instructions using the wrt-instructions
/// implementations.
pub struct WrtExecutionContextAdapter<'a> {
    /// The stack used for execution
    stack:        &'a mut dyn StackLike,
    /// The current frame
    frame:        &'a mut StacklessFrame,
    /// The engine
    engine:       &'a mut StacklessEngine,
    /// SIMD runtime for SIMD operations
    #[cfg(feature = "platform")]
    simd_runtime: SimdRuntime,
}

impl<'a> WrtExecutionContextAdapter<'a> {
    /// Create a new execution context adapter
    ///
    /// # Arguments
    ///
    /// * `stack` - The stack to use for execution
    /// * `frame` - The current frame
    /// * `engine` - The engine
    ///
    /// # Returns
    ///
    /// A new execution context adapter
    pub fn new(
        stack: &'a mut dyn StackLike,
        frame: &'a mut StacklessFrame,
        engine: &'a mut StacklessEngine,
    ) -> Self {
        Self {
            stack,
            frame,
            engine,
            #[cfg(feature = "platform")]
            simd_runtime: SimdRuntime::new(),
        }
    }
}

/// Stack-like trait for interfacing with different stack implementations
pub trait StackLike {
    /// Push a value onto the stack
    fn push(&mut self, value: Value) -> Result<()>;

    /// Pop a value from the stack
    fn pop(&mut self) -> Result<Value>;

    /// Peek at the top value without removing it
    fn peek(&self) -> Result<Value>;

    /// Get the current stack depth
    fn depth(&self) -> usize;
}

impl<'a> ExecutionContext for WrtExecutionContextAdapter<'a> {
    fn push_value(&mut self, value: Value) -> Result<()> {
        self.stack.push(value)
    }

    fn pop_value(&mut self) -> Result<Value> {
        self.stack.pop()
    }

    fn pop_value_expected(&mut self, expected_type: ValueType) -> Result<Value> {
        let value = self.pop_value()?;
        if value.value_type() != expected_type {
            return Err(Error::runtime_execution_error("Expected {:?}, got {:?}"));
        }
        Ok(value)
    }

    fn memory_size(&mut self, memory_idx: u32) -> Result<u32> {
        let memory = self.frame.get_memory(memory_idx, self.engine)?;

        memory.size()
    }

    fn memory_grow(&mut self, memory_idx: u32, pages: u32) -> Result<u32> {
        let memory = self.frame.get_memory(memory_idx, self.engine)?;

        memory.grow(pages)
    }

    fn memory_read(&mut self, memory_idx: u32, offset: u32, bytes: &mut [u8]) -> Result<()> {
        let memory = self.frame.get_memory(memory_idx, self.engine)?;

        memory.read(offset, bytes)
    }

    fn memory_write(&mut self, memory_idx: u32, offset: u32, bytes: &[u8]) -> Result<()> {
        let memory = self.frame.get_memory(memory_idx, self.engine)?;

        memory.write(offset, bytes)
    }
}

#[cfg(feature = "platform")]
impl<'a> SimdContext for WrtExecutionContextAdapter<'a> {
    fn execute_simd_op(&mut self, op: SimdOp, inputs: &[Value]) -> Result<Value> {
        // Use the comprehensive SIMD implementation
        let provider = self.simd_runtime.provider();
        simd_runtime_impl::execute_simd_operation(op, inputs, provider.as_ref())
    }
}

/// Extract v128 bytes from a Value
#[cfg(feature = "platform")]
fn extract_v128_bytes(value: &Value) -> Result<[u8; 16]> {
    match value {
        Value::V128(bytes) => Ok(*bytes),
        _ => Err(Error::runtime_execution_error(
            "Expected v128 value, got {:?}",
        )),
    }
}

#[cfg(feature = "platform")]
impl<'a> SimdExecutionContext for WrtExecutionContextAdapter<'a> {
    fn pop_value(&mut self) -> Result<Value> {
        self.stack.pop()
    }

    fn push_value(&mut self, value: Value) -> Result<()> {
        self.stack.push(value)
    }

    fn simd_context(&mut self) -> &mut dyn SimdContext {
        self as &mut dyn SimdContext
    }
}

/// Implementation of AggregateOperations for WrtExecutionContextAdapter
impl<'a> AggregateOperations for WrtExecutionContextAdapter<'a> {
    fn get_struct_type(&self, type_index: u32) -> Result<Option<u32>> {
        // In a full implementation, this would query the module's type section
        // For now, we'll assume types 0-99 exist (mock implementation)
        if type_index < 100 {
            Ok(Some(type_index))
        } else {
            Ok(None)
        }
    }

    fn get_array_type(&self, type_index: u32) -> Result<Option<u32>> {
        // In a full implementation, this would query the module's type section
        // For now, we'll assume types 0-99 exist (mock implementation)
        if type_index < 100 {
            Ok(Some(type_index))
        } else {
            Ok(None)
        }
    }

    fn validate_struct_type(&self, type_index: u32) -> Result<()> {
        // In a full implementation, this would validate against the module's type
        // section
        if type_index < 100 {
            Ok(())
        } else {
            Err(Error::runtime_execution_error(
                "Invalid struct type index: {}",
            ))
        }
    }

    fn validate_array_type(&self, type_index: u32) -> Result<()> {
        // In a full implementation, this would validate against the module's type
        // section
        if type_index < 100 {
            Ok(())
        } else {
            Err(Error::runtime_execution_error(
                "Invalid array type index: {}",
            ))
        }
    }
}

/// Execute an instruction using the wrt runtime
///
/// This function executes a WebAssembly instruction using the wrt runtime,
/// bridging between the wrt-instructions implementations and the wrt runtime.
///
/// # Arguments
///
/// * `instruction` - The instruction to execute
/// * `stack` - The stack to use for execution
/// * `frame` - The current frame
/// * `engine` - The engine
///
/// # Returns
///
/// A Result indicating whether the execution was successful
pub fn execute_instruction<'a, T>(
    instruction: &T,
    stack: &'a mut dyn StackLike,
    frame: &'a mut StacklessFrame,
    engine: &'a mut StacklessEngine,
) -> Result<()>
where
    T: wrt_instructions::InstructionExecutable<WrtExecutionContextAdapter<'a>>,
{
    // Create an adapter for the execution context
    let mut context_adapter = WrtExecutionContextAdapter::new(stack, frame, engine);

    // Execute the instruction
    instruction.execute(&mut context_adapter).map_err(|e| Error::from(e))
}

// Additional adapter traits can be added here as needed:

// Memory adapter for pure memory instructions
// Table adapter for pure table instructions
// etc.
