// Copyright 2024 The WRT Project Authors

//! Instructions adapter for wrt
//!
//! This adapter bridges between `wrt` execution context and `wrt-instructions` execution context.
//! It provides the necessary adapters to use implementations from `wrt-instructions`
//! in the wrt runtime.

use crate::prelude::*;
use crate::stackless::StacklessEngine;
use crate::stackless_frame::StacklessFrame;

/// Re-export instruction types and traits from wrt-instructions
pub use wrt_instructions::{
    behavior::{
        ControlFlow, ControlFlowBehavior, FrameBehavior, InstructionExecutor, StackBehavior,
    },
    calls::CallInstruction,
    control::ControlInstruction,
    execution::{ExecutionContext, ExecutionResult},
    memory_ops::{MemoryArg, MemoryLoad, MemoryStore},
    numeric::NumericInstruction,
    Instruction, InstructionExecutable,
};

/// Execution context adapter for instructions
///
/// This adapter implements the ExecutionContext trait from wrt-instructions,
/// allowing the wrt runtime to execute instructions using the wrt-instructions
/// implementations.
pub struct WrtExecutionContextAdapter<'a> {
    /// The stack used for execution
    stack: &'a mut dyn StackLike,
    /// The current frame
    frame: &'a mut StacklessFrame,
    /// The engine
    engine: &'a mut StacklessEngine,
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
        Self { stack, frame, engine }
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

impl<'a> wrt_instructions::execution::ExecutionContext for WrtExecutionContextAdapter<'a> {
    fn push_value(&mut self, value: Value) -> wrt_error::Result<()> {
        self.stack.push(value).map_err(|e| wrt_error::Error::from(e))
    }

    fn pop_value(&mut self) -> wrt_error::Result<Value> {
        self.stack.pop().map_err(|e| wrt_error::Error::from(e))
    }

    fn pop_value_expected(&mut self, expected_type: ValueType) -> wrt_error::Result<Value> {
        let value = self.pop_value()?;
        if value.value_type() != expected_type {
            return Err(wrt_error::Error::new(
                wrt_error::ErrorCategory::Type,
                wrt_error::codes::TYPE_MISMATCH,
                format!("Expected {:?}, got {:?}", expected_type, value.value_type()),
            ));
        }
        Ok(value)
    }

    fn memory_size(&mut self, memory_idx: u32) -> wrt_error::Result<u32> {
        let memory = self
            .frame
            .get_memory(memory_idx, self.engine)
            .map_err(|e| wrt_error::Error::from(e))?;

        memory.size().map_err(|e| wrt_error::Error::from(e))
    }

    fn memory_grow(&mut self, memory_idx: u32, pages: u32) -> wrt_error::Result<u32> {
        let memory = self
            .frame
            .get_memory(memory_idx, self.engine)
            .map_err(|e| wrt_error::Error::from(e))?;

        memory.grow(pages).map_err(|e| wrt_error::Error::from(e))
    }

    fn memory_read(
        &mut self,
        memory_idx: u32,
        offset: u32,
        bytes: &mut [u8],
    ) -> wrt_error::Result<()> {
        let memory = self
            .frame
            .get_memory(memory_idx, self.engine)
            .map_err(|e| wrt_error::Error::from(e))?;

        memory.read(offset, bytes).map_err(|e| wrt_error::Error::from(e))
    }

    fn memory_write(
        &mut self,
        memory_idx: u32,
        offset: u32,
        bytes: &[u8],
    ) -> wrt_error::Result<()> {
        let memory = self
            .frame
            .get_memory(memory_idx, self.engine)
            .map_err(|e| wrt_error::Error::from(e))?;

        memory.write(offset, bytes).map_err(|e| wrt_error::Error::from(e))
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
