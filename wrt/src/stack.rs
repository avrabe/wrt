use crate::behavior::{self, Label as BehaviorLabel, StackBehavior};
use crate::prelude::TypesValue as Value;
use crate::StacklessEngine;
use wrt_error::{kinds, Error, Result};

// Import Vec for standard stack
#[cfg(feature = "std")]
use std::vec::Vec;

// Import SafeStack for memory-safe stack
use wrt_types::safe_memory::SafeStack;

/// Represents a control flow label on the stack (e.g., for blocks, loops, ifs).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Label {
    /// The number of values the instruction sequence associated with the label is expected to produce.
    pub arity: usize,
    /// The program counter (instruction index) where execution should resume after the block.
    pub pc: usize,
    /// The program counter for the continuation (e.g., the `else` branch of an `if`).
    pub continuation: usize,
    /// The depth of the value stack when this label was pushed (used for stack cleanup on branch).
    pub stack_depth: usize,
    /// Indicates if this label represents a loop (for `br` targeting).
    pub is_loop: bool,
    /// Indicates if this label represents an if block (for `else` handling).
    pub is_if: bool,
}

impl From<BehaviorLabel> for Label {
    fn from(label: BehaviorLabel) -> Self {
        Self {
            arity: label.arity,
            pc: label.pc,
            continuation: label.continuation,
            stack_depth: label.stack_depth,
            is_loop: label.is_loop,
            is_if: label.is_if,
        }
    }
}

impl From<Label> for BehaviorLabel {
    fn from(label: Label) -> Self {
        Self {
            arity: label.arity,
            pc: label.pc,
            continuation: label.continuation,
            stack_depth: label.stack_depth,
            is_loop: label.is_loop,
            is_if: label.is_if,
        }
    }
}

// Implementation for standard Vec-based stack (when std feature is enabled)
#[cfg(feature = "std")]
impl StackBehavior for Vec<Value> {
    fn push(&mut self, value: Value) -> Result<(), Error> {
        self.push(value);
        Ok(())
    }

    fn pop(&mut self) -> Result<Value, Error> {
        self.pop()
            .ok_or_else(|| kinds::StackUnderflowError("Stack underflow".to_string()).into())
    }

    fn peek(&self) -> Result<&Value, Error> {
        self.last()
            .ok_or_else(|| kinds::StackUnderflowError("Stack underflow".to_string()).into())
    }

    fn peek_mut(&mut self) -> Result<&mut Value, Error> {
        self.last_mut()
            .ok_or_else(|| kinds::StackUnderflowError("Stack underflow".to_string()).into())
    }

    fn values(&self) -> &[Value] {
        self.as_slice()
    }

    fn values_mut(&mut self) -> &mut [Value] {
        self.as_mut_slice()
    }

    fn len(&self) -> usize {
        Vec::len(self)
    }

    fn is_empty(&self) -> bool {
        Vec::is_empty(self)
    }

    fn push_label(&mut self, label: BehaviorLabel) -> Result<(), Error> {
        // Stack doesn't track labels directly - unsupported operation
        Err(kinds::NotImplementedError(
            "push_label not supported on raw stack".to_string(),
        ).into())
    }

    fn pop_label(&mut self) -> Result<BehaviorLabel, Error> {
        // Stack doesn't track labels directly - unsupported operation
        Err(kinds::NotImplementedError(
            "pop_label not supported on raw stack".to_string(),
        ).into())
    }

    fn get_label(&self, _index: usize) -> Option<&BehaviorLabel> {
        // Stack doesn't track labels directly
        None
    }

    fn push_n(&mut self, values: &[Value]) {
        self.extend_from_slice(values);
    }

    fn pop_n(&mut self, n: usize) -> Vec<Value> {
        if self.len() < n {
            // Since we can't return an error, return empty vec on underflow
            return Vec::new();
        }
        let new_len = self.len() - n;
        let result = self.split_off(new_len);
        result
    }

    fn pop_frame_label(&mut self) -> Result<BehaviorLabel, Error> {
        // Stack doesn't track frame labels directly - unsupported operation
        Err(kinds::NotImplementedError(
            "pop_frame_label not supported on raw stack".to_string(),
        ).into())
    }

    fn execute_function_call_direct(
        &mut self,
        _engine: &mut StacklessEngine,
        _caller_instance_idx: u32,
        _func_idx: u32,
        _args: Vec<Value>,
    ) -> Result<Vec<Value>, Error> {
        // Raw stack can't execute functions
        Err(kinds::NotImplementedError(
            "Function calls not supported on raw stack".to_string(),
        ).into())
    }

    fn as_any(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

// Implementation for SafeStack-based stack (usable in both std and no_std environments)
impl StackBehavior for SafeStack<Value> {
    fn push(&mut self, value: Value) -> Result<(), Error> {
        self.push(value)
            .map_err(|e| kinds::StackOverflowError(e.to_string()).into())
    }

    fn pop(&mut self) -> Result<Value, Error> {
        self.pop()
            .map_err(|e| kinds::StackUnderflowError(e.to_string()).into())
    }

    fn peek(&self) -> Result<&Value, Error> {
        // SafeStack doesn't provide a direct reference to elements
        // We'd need additional caching of the last value to support this
        Err(kinds::NotImplementedError(
            "peek with reference not supported for SafeStack".to_string(),
        ).into())
    }

    fn peek_mut(&mut self) -> Result<&mut Value, Error> {
        // SafeStack doesn't provide a direct mutable reference to elements
        // We'd need additional caching of the last value to support this
        Err(kinds::NotImplementedError(
            "peek_mut with reference not supported for SafeStack".to_string(),
        ).into())
    }

    fn values(&self) -> &[Value] {
        // SafeStack doesn't provide direct reference to values
        // This is a placeholder that returns an empty slice
        &[]
    }

    fn values_mut(&mut self) -> &mut [Value] {
        // SafeStack doesn't provide direct mutable reference to values
        // This is a placeholder that returns an empty slice
        &mut []
    }

    fn len(&self) -> usize {
        self.len()
    }

    fn is_empty(&self) -> bool {
        self.is_empty()
    }

    fn push_label(&mut self, label: BehaviorLabel) -> Result<(), Error> {
        // Stack doesn't track labels directly - unsupported operation
        Err(kinds::NotImplementedError(
            "push_label not supported on raw stack".to_string(),
        ).into())
    }

    fn pop_label(&mut self) -> Result<BehaviorLabel, Error> {
        // Stack doesn't track labels directly - unsupported operation
        Err(kinds::NotImplementedError(
            "pop_label not supported on raw stack".to_string(),
        ).into())
    }

    fn get_label(&self, _index: usize) -> Option<&BehaviorLabel> {
        // Stack doesn't track labels directly
        None
    }

    fn push_n(&mut self, values: &[Value]) {
        for value in values {
            // Ignore errors for now
            let _ = self.push(value.clone());
        }
    }

    fn pop_n(&mut self, n: usize) -> Vec<Value> {
        let mut result = Vec::with_capacity(n);
        for _ in 0..n {
            match self.pop() {
                Ok(value) => result.push(value),
                Err(_) => break,
            }
        }
        result
    }

    fn pop_frame_label(&mut self) -> Result<BehaviorLabel, Error> {
        // Stack doesn't track frame labels directly - unsupported operation
        Err(kinds::NotImplementedError(
            "pop_frame_label not supported on raw stack".to_string(),
        ).into())
    }

    fn execute_function_call_direct(
        &mut self,
        _engine: &mut StacklessEngine,
        _caller_instance_idx: u32,
        _func_idx: u32,
        _args: Vec<Value>,
    ) -> Result<Vec<Value>, Error> {
        // Raw stack can't execute functions
        Err(kinds::NotImplementedError(
            "Function calls not supported on raw stack".to_string(),
        ).into())
    }

    fn as_any(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
