// Copyright (c) 2025 Ralf Anton Beier
// Licensed under the MIT license.
// SPDX-License-Identifier: MIT

//! Control flow operations for WebAssembly instructions.
//!
//! This module provides pure implementations for WebAssembly control flow
//! instructions, including block, loop, if, branch, return, and call operations.
//!
//! # Control Flow Architecture
//!
//! This module separates control flow operations from the underlying execution
//! engine, allowing different execution engines to share the same control flow
//! code. The key components are:
//!
//! - Individual operation structs: `Return`, `CallIndirect`, `BrTable`
//! - `ControlOp` unified enum for instruction dispatching
//! - `ControlContext` trait defining the interface to execution engines
//! - `FunctionOperations` trait for function-related operations
//!
//! # Features
//!
//! - Support for all WebAssembly control flow operations
//! - Function call mechanisms (direct and indirect)
//! - Branch table implementation with fallback
//! - Structured control flow (blocks, loops, if/else)
//! - Function return with proper value handling
//!
//! # Usage
//!
//! ```no_run
//! use wrt_instructions::control_ops::{Return, CallIndirect, BrTable};
//! use wrt_instructions::Value;
//!
//! // Return from function
//! let return_op = Return::new();
//! // Execute with appropriate context
//!
//! // Indirect function call
//! let call_op = CallIndirect::new(0, 1); // table 0, type 1
//! // Execute with appropriate context
//!
//! // Branch table
//! let br_table = BrTable::new(vec![0, 1, 2], 3); // targets + default
//! // Execute with appropriate context
//! ```

#![allow(clippy::match_single_binding)]

// Remove unused imports

use crate::prelude::{BlockType, Debug, Error, ErrorCategory, PartialEq, PureInstruction, Result, Value, codes, str, BoundedVec, BoundedCapacity};
// use crate::validation::{Validate, ValidationContext}; // Currently unused


/// Branch target information
#[derive(Debug, Clone)]
pub struct BranchTarget {
    /// The label index to branch to
    pub label_idx: u32,
    /// The number of values to keep when branching
    pub keep_values: usize,
}

/// Type alias for block type used in control flow
pub type ControlBlockType = BlockType;

/// Represent blocks for the execution flow
#[derive(Debug, Clone)]
pub enum Block {
    /// Regular block
    Block(ControlBlockType),
    /// Loop block
    Loop(ControlBlockType),
    /// If block (with else branch)
    If(ControlBlockType),
    /// Try block
    Try(ControlBlockType),
}

/// Represents a pure control flow operation for WebAssembly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ControlOp {
    /// A basic block of instructions with a label that can be branched to
    Block(ControlBlockType),
    /// A loop block, where branching to it jumps to the beginning
    Loop(ControlBlockType),
    /// A conditional block, executing either the then or else branch
    If(ControlBlockType),
    /// The else part of an if block
    Else,
    /// End of a block, loop, if, or function
    End,
    /// Unconditional branch to a label
    Br(u32),
    /// Conditional branch to a label
    BrIf(u32),
    /// Branch to a label in a table
    BrTable {
        /// Table of branch target labels
        #[cfg(feature = "std")]
        table: Vec<u32>,
        /// Table of branch target labels (`no_std`)
        #[cfg(not(feature = "std"))]
        table: BoundedVec<u32, 256, wrt_foundation::NoStdProvider<8192>>,
        /// Default label to branch to if the index is out of bounds
        default: u32,
    },
    /// Return from a function
    Return,
    /// Call a function by index
    Call(u32),
    /// Calls a function through a table indirection
    CallIndirect {
        /// Index of the table to use for the call
        table_idx: u32,
        /// Type index for the function signature
        type_idx: u32,
    },
    /// Tail call a function by index (`return_call`)
    ReturnCall(u32),
    /// Tail call a function through table indirection (`return_call_indirect`)
    ReturnCallIndirect {
        /// Index of the table to use for the call
        table_idx: u32,
        /// Type index for the function signature
        type_idx: u32,
    },
    /// Execute a nop instruction (no operation)
    Nop,
    /// Execute an unreachable instruction (causes trap)
    Unreachable,
    /// Branch if reference is null (`br_on_null`)
    BrOnNull(u32),
    /// Branch if reference is not null (`br_on_non_null`)
    BrOnNonNull(u32),
}

impl Default for ControlOp {
    fn default() -> Self {
        ControlOp::Nop
    }
}

/// Return operation (return)
#[derive(Debug, Clone, PartialEq)]
pub struct Return;

impl Return {
    /// Create a new return operation
    #[must_use] pub fn new() -> Self {
        Self
    }
    
    /// Execute return operation
    ///
    /// # Arguments
    ///
    /// * `context` - The execution context
    ///
    /// # Returns
    ///
    /// Success or an error
    pub fn execute(&self, context: &mut impl ControlContext) -> Result<()> {
        context.execute_return()
    }
}

impl Default for Return {
    fn default() -> Self {
        Self::new()
    }
}

/// Call indirect operation (`call_indirect`)
#[derive(Debug, Clone, PartialEq)]
pub struct CallIndirect {
    /// Table index to use for the indirect call
    pub table_idx: u32,
    /// Expected function type index
    pub type_idx: u32,
}

impl CallIndirect {
    /// Create a new `call_indirect` operation
    #[must_use] pub fn new(table_idx: u32, type_idx: u32) -> Self {
        Self { table_idx, type_idx }
    }
    
    /// Execute `call_indirect` operation
    ///
    /// # Arguments
    ///
    /// * `context` - The execution context
    ///
    /// # Returns
    ///
    /// Success or an error
    pub fn execute(&self, context: &mut impl ControlContext) -> Result<()> {
        // Pop the function index from the stack
        let func_idx = context.pop_control_value()?.into_i32().map_err(|_| {
            Error::type_error("call_indirect expects i32 function index")
        })?;
        
        // Validate function index is not negative
        if func_idx < 0 {
            return Err(Error::runtime_error("Invalid function index for call_indirect"));
        }
        
        // Execute the indirect call with validation
        context.execute_call_indirect(self.table_idx, self.type_idx, func_idx)
    }
}

/// Return call indirect operation (`return_call_indirect`)
#[derive(Debug, Clone, PartialEq)]
pub struct ReturnCallIndirect {
    /// Table index to use for the indirect call
    pub table_idx: u32,
    /// Expected function type index
    pub type_idx: u32,
}

impl ReturnCallIndirect {
    /// Create a new `return_call_indirect` operation
    #[must_use] pub fn new(table_idx: u32, type_idx: u32) -> Self {
        Self { table_idx, type_idx }
    }
    
    /// Execute `return_call_indirect` operation
    ///
    /// This performs a tail call through a table. It's equivalent to:
    /// 1. Performing `call_indirect`
    /// 2. Immediately returning the result
    /// 
    /// But optimized to reuse the current call frame.
    ///
    /// # Arguments
    ///
    /// * `context` - The execution context
    ///
    /// # Returns
    ///
    /// Success or an error
    pub fn execute(&self, context: &mut impl ControlContext) -> Result<()> {
        // Pop the function index from the stack
        let func_idx = context.pop_control_value()?.into_i32().map_err(|_| {
            Error::type_error("return_call_indirect expects i32 function index")
        })?;
        
        // Validate function index is not negative
        if func_idx < 0 {
            return Err(Error::runtime_error("Invalid function index for return_call_indirect"));
        }
        
        // Execute the tail call indirect
        context.return_call_indirect(self.table_idx, self.type_idx)
    }
}

/// Branch table operation (`br_table`)
#[derive(Debug, Clone, PartialEq)]
pub struct BrTable {
    /// Table of branch target labels
    #[cfg(feature = "std")]
    pub table: Vec<u32>,
    
    #[cfg(not(feature = "std"))]
    pub table: wrt_foundation::BoundedVec<u32, 256, wrt_foundation::NoStdProvider<8192>>,
    
    /// Default label to branch to if the index is out of bounds
    pub default: u32,
}

impl BrTable {
    /// Binary std/no_std choice
    #[cfg(feature = "std")]
    pub fn new(table: Vec<u32>, default: u32) -> Self {
        Self { table, default }
    }
    
    /// Create a new `br_table` operation with `BoundedVec` (`no_std`)
    #[cfg(not(feature = "std"))]
    pub fn new_bounded(
        table: wrt_foundation::BoundedVec<u32, 256, wrt_foundation::NoStdProvider<8192>>, 
        default: u32
    ) -> Self {
        Self { table, default }
    }
    
    /// Create a `br_table` from a slice (works in all environments)
    pub fn from_slice(table_slice: &[u32], default: u32) -> Result<Self> {
        #[cfg(feature = "std")]
        {
            Ok(Self {
                table: table_slice.to_vec(),
                default,
            })
        }
        #[cfg(not(feature = "std"))]
        {
            let provider = wrt_foundation::safe_managed_alloc!(8192, wrt_foundation::budget_aware_provider::CrateId::Instructions)?;
            let mut table = wrt_foundation::BoundedVec::new(provider).map_err(|_| {
                Error::memory_error("Could not create BoundedVec")
            })?;
            for &label in table_slice {
                table.push(label).map_err(|_| {
                    Error::memory_error("Branch table exceeds maximum size")
                })?;
            }
            Ok(Self { table, default })
        }
    }
    
    /// Execute `br_table` operation
    ///
    /// # Arguments
    ///
    /// * `context` - The execution context
    ///
    /// # Returns
    ///
    /// Success or an error
    pub fn execute(&self, context: &mut impl ControlContext) -> Result<()> {
        // Pop the table index from the stack
        let index = context.pop_control_value()?.into_i32().map_err(|_| {
            Error::type_error("br_table expects i32 index")
        })?;
        
        // Execute the branch table operation with different approaches per feature
        #[cfg(feature = "std")]
        {
            context.execute_br_table(self.table.as_slice(), self.default, index)
        }
        #[cfg(not(feature = "std"))]
        {
            // For no_std, we create a temporary slice on the stack
            let mut slice_vec = [0u32; 256]; // Static array for no_std
            let len = core::cmp::min(BoundedCapacity::len(&self.table), 256);
            for i in 0..len {
                slice_vec[i] = self.table.get(i).map_err(|_| {
                    Error::runtime_error("Branch table index out of bounds")
                })?;
            }
            context.execute_br_table(&slice_vec[..len], self.default, index)
        }
    }
}

/// Function operations trait for call-related operations
pub trait FunctionOperations {
    /// Get function type signature by index
    fn get_function_type(&self, func_idx: u32) -> Result<u32>;
    
    /// Get table element (function reference) by index
    fn get_table_function(&self, table_idx: u32, elem_idx: u32) -> Result<u32>;
    
    /// Validate function signature matches expected type
    fn validate_function_signature(&self, func_idx: u32, expected_type: u32) -> Result<()>;
    
    /// Execute function call
    fn execute_function_call(&mut self, func_idx: u32) -> Result<()>;
}

/// Execution context for control flow operations
pub trait ControlContext {
    /// Push a value to the stack
    fn push_control_value(&mut self, value: Value) -> Result<()>;

    /// Pop a value from the stack
    fn pop_control_value(&mut self) -> Result<Value>;

    /// Get the current block depth
    fn get_block_depth(&self) -> usize;

    /// Start a new block
    fn enter_block(&mut self, block_type: Block) -> Result<()>;

    /// Exit the current block
    fn exit_block(&mut self) -> Result<Block>;

    /// Branch to a specific label
    fn branch(&mut self, target: BranchTarget) -> Result<()>;

    /// Return from the current function
    fn return_function(&mut self) -> Result<()>;

    /// Call a function by index
    fn call_function(&mut self, func_idx: u32) -> Result<()>;

    /// Call a function indirectly through a table
    fn call_indirect(&mut self, table_idx: u32, type_idx: u32) -> Result<()>;
    
    /// Tail call a function by index (`return_call`)
    fn return_call(&mut self, func_idx: u32) -> Result<()>;
    
    /// Tail call a function indirectly through a table (`return_call_indirect`)
    fn return_call_indirect(&mut self, table_idx: u32, type_idx: u32) -> Result<()>;

    /// Trap the execution (unreachable)
    fn trap(&mut self, message: &str) -> Result<()>;

    /// Get the current block
    fn get_current_block(&self) -> Option<&Block>;
    
    /// Get function operations interface
    fn get_function_operations(&mut self) -> Result<&mut dyn FunctionOperations>;
    
    /// Execute function return with value handling
    fn execute_return(&mut self) -> Result<()>;
    
    /// Execute `call_indirect` with full validation
    fn execute_call_indirect(&mut self, table_idx: u32, type_idx: u32, func_idx: i32) -> Result<()>;
    
    /// Execute branch table operation
    fn execute_br_table(&mut self, table: &[u32], default: u32, index: i32) -> Result<()>;
    
    /// Execute branch on null - branch if reference is null
    fn execute_br_on_null(&mut self, label: u32) -> Result<()>;
    
    /// Execute branch on non-null - branch if reference is not null  
    fn execute_br_on_non_null(&mut self, label: u32) -> Result<()>;
}

impl<T: ControlContext> PureInstruction<T, Error> for ControlOp {
    fn execute(&self, context: &mut T) -> Result<()> {
        match self {
            Self::Block(block_type) => context.enter_block(Block::Block(*block_type)),
            Self::Loop(block_type) => context.enter_block(Block::Loop(*block_type)),
            Self::If(block_type) => {
                let condition = context.pop_control_value()?.into_i32().map_err(|_| {
                    Error::type_error("Expected I32 for if condition")
                })?;

                if condition != 0 {
                    // Condition is true, enter the if block
                    context.enter_block(Block::If(*block_type))
                } else {
                    // Condition is false, skip to the else or end
                    // The runtime will handle this by setting a flag to skip instructions
                    // until the corresponding else or end is found
                    context.enter_block(Block::If(*block_type))
                }
            }
            Self::Else => {
                // The runtime will handle this by switching execution context
                // between the then and else branches
                Ok(())
            }
            Self::End => {
                // End the current block
                context.exit_block().map(|_| ())
            }
            Self::Br(label_idx) => {
                let target = BranchTarget {
                    label_idx: *label_idx,
                    keep_values: 0, // The runtime will resolve this based on block types
                };
                context.branch(target)
            }
            Self::BrIf(label_idx) => {
                let condition = context.pop_control_value()?.into_i32().map_err(|_| {
                    Error::type_error("Expected I32 for br_if condition")
                })?;

                if condition != 0 {
                    let target = BranchTarget {
                        label_idx: *label_idx,
                        keep_values: 0, // The runtime will resolve this based on block types
                    };
                    context.branch(target)
                } else {
                    // Do not branch
                    Ok(())
                }
            }
            Self::BrTable { table, default } => {
                // Use from_slice for unified interface across all feature configurations
                #[cfg(feature = "std")]
                let slice: &[u32] = table.as_slice();
                #[cfg(not(feature = "std"))]
                let slice: &[u32] = table.as_slice().map_err(|_| Error::runtime_error("Failed to get table slice"))?;
                
                let br_table = BrTable::from_slice(slice, *default)?;
                br_table.execute(context)
            }
            Self::Return => {
                let return_op = Return::new();
                return_op.execute(context)
            }
            Self::Call(func_idx) => context.call_function(*func_idx),
            Self::CallIndirect { table_idx, type_idx } => {
                let call_op = CallIndirect::new(*table_idx, *type_idx);
                call_op.execute(context)
            }
            Self::ReturnCall(func_idx) => context.return_call(*func_idx),
            Self::ReturnCallIndirect { table_idx, type_idx } => {
                let call_op = ReturnCallIndirect::new(*table_idx, *type_idx);
                call_op.execute(context)
            }
            Self::Nop => {
                // No operation, just return Ok
                Ok(())
            }
            Self::Unreachable => {
                // The unreachable instruction unconditionally traps
                context.trap("unreachable instruction executed")
            }
            Self::BrOnNull(label) => {
                // Pop reference from stack
                let reference = context.pop_control_value()?;
                
                // Check if reference is null and branch accordingly
                let should_branch = match reference {
                    Value::FuncRef(None) | Value::ExternRef(None) => true,
                    Value::FuncRef(Some(_)) | Value::ExternRef(Some(_)) => {
                        // Reference is not null, put it back on stack
                        context.push_control_value(reference)?;
                        false
                    }
                    _ => {
                        return Err(Error::type_error("br_on_null requires a reference type"));
                    }
                };
                
                if should_branch {
                    context.execute_br_on_null(*label)
                } else {
                    Ok(())
                }
            }
            Self::BrOnNonNull(label) => {
                // Pop reference from stack
                let reference = context.pop_control_value()?;
                
                // Check if reference is not null and branch accordingly
                let should_branch = match reference {
                    Value::FuncRef(None) | Value::ExternRef(None) => false,
                    Value::FuncRef(Some(_)) | Value::ExternRef(Some(_)) => {
                        // Reference is not null, keep it on stack for the branch target
                        context.push_control_value(reference)?;
                        true
                    }
                    _ => {
                        return Err(Error::type_error("br_on_non_null requires a reference type"));
                    }
                };
                
                if should_branch {
                    context.execute_br_on_non_null(*label)
                } else {
                    Ok(())
                }
            }
        }
    }
}

#[cfg(all(test, feature = "std"))]
mod tests {
        use std::vec;
        use std::vec::Vec;
    // Import Vec and vec! based on feature flags
    #[cfg(feature = "std")]
    use std::vec::Vec;

    use wrt_foundation::types::ValueType;

    use super::*;

    // A simplified control context for testing
    struct MockControlContext {
        stack: Vec<Value>,
        blocks: Vec<Block>,
        branched: Option<BranchTarget>,
        returned: bool,
        trapped: bool,
        func_called: Option<u32>,
        indirect_call: Option<(u32, u32)>,
    }

    impl MockControlContext {
        fn new() -> Self {
            Self {
                stack: Vec::new(),
                blocks: Vec::new(),
                branched: None,
                returned: false,
                trapped: false,
                func_called: None,
                indirect_call: None,
            }
        }
    }

    impl ControlContext for MockControlContext {
        fn push_control_value(&mut self, value: Value) -> Result<()> {
            self.stack.push(value);
            Ok(())
        }

        fn pop_control_value(&mut self) -> Result<Value> {
            self.stack.pop().ok_or_else(|| {
                Error::runtime_stack_underflow("Stack underflow")
            })
        }

        fn get_block_depth(&self) -> usize {
            self.blocks.len()
        }

        fn enter_block(&mut self, block_type: Block) -> Result<()> {
            self.blocks.push(block_type);
            Ok(())
        }

        fn exit_block(&mut self) -> Result<Block> {
            self.blocks.pop().ok_or_else(|| {
                Error::runtime_execution_error("Invalid branch target")
            })
        }

        fn branch(&mut self, target: BranchTarget) -> Result<()> {
            self.branched = Some(target);
            Ok(())
        }

        fn return_function(&mut self) -> Result<()> {
            self.returned = true;
            Ok(())
        }

        fn call_function(&mut self, func_idx: u32) -> Result<()> {
            self.func_called = Some(func_idx);
            Ok(())
        }

        fn call_indirect(&mut self, table_idx: u32, type_idx: u32) -> Result<()> {
            self.indirect_call = Some((table_idx, type_idx));
            Ok(())
        }

        fn trap(&mut self, _message: &str) -> Result<()> {
            self.trapped = true;
            Err(Error::runtime_trap_error("Execution trapped"))
        }

        fn get_current_block(&self) -> Option<&Block> {
            self.blocks.last()
        }
        
        fn get_function_operations(&mut self) -> Result<&mut dyn FunctionOperations> {
            Ok(self as &mut dyn FunctionOperations)
        }
        
        fn execute_return(&mut self) -> Result<()> {
            self.returned = true;
            Ok(())
        }
        
        fn execute_call_indirect(&mut self, table_idx: u32, type_idx: u32, func_idx: i32) -> Result<()> {
            if func_idx < 0 {
                return Err(Error::runtime_error("Invalid function index"));
            }
            self.indirect_call = Some((table_idx, type_idx));
            Ok(())
        }
        
        fn execute_br_table(&mut self, table: &[u32], default: u32, index: i32) -> Result<()> {
            let label_idx = if index >= 0 && (index as usize) < table.len() {
                table[index as usize]
            } else {
                default
            };
            
            let target = BranchTarget {
                label_idx,
                keep_values: 0,
            };
            self.branched = Some(target);
            Ok(())
        }
        
        fn execute_br_on_null(&mut self, label: u32) -> Result<()> {
            let target = BranchTarget {
                label_idx: label,
                keep_values: 0,
            };
            self.branched = Some(target);
            Ok(())
        }
        
        fn execute_br_on_non_null(&mut self, label: u32) -> Result<()> {
            let target = BranchTarget {
                label_idx: label,
                keep_values: 0,
            };
            self.branched = Some(target);
            Ok(())
        }
        
        fn return_call(&mut self, func_idx: u32) -> Result<()> {
            self.func_called = Some(func_idx);
            Ok(())
        }
        
        fn return_call_indirect(&mut self, table_idx: u32, type_idx: u32) -> Result<()> {
            self.indirect_call = Some((table_idx, type_idx));
            Ok(())
        }
    }
    
    impl FunctionOperations for MockControlContext {
        fn get_function_type(&self, func_idx: u32) -> Result<u32> {
            Ok(func_idx % 5) // Mock: 5 different function types
        }
        
        fn get_table_function(&self, table_idx: u32, elem_idx: u32) -> Result<u32> {
            Ok(table_idx * 100 + elem_idx) // Mock calculation
        }
        
        fn validate_function_signature(&self, func_idx: u32, expected_type: u32) -> Result<()> {
            let actual_type = self.get_function_type(func_idx)?;
            if actual_type == expected_type {
                Ok(())
            } else {
                Err(Error::type_error("Function signature mismatch"))
            }
        }
        
        fn execute_function_call(&mut self, func_idx: u32) -> Result<()> {
            self.func_called = Some(func_idx);
            Ok(())
        }
    }

    #[test]
    fn test_block_operations() {
        let mut context = MockControlContext::new();

        // Test block instruction
        let block_type = ControlBlockType::ValueType(Some(ValueType::I32));
        ControlOp::Block(block_type.clone()).execute(&mut context).unwrap();
        assert_eq!(context.get_block_depth(), 1);

        // Test end instruction
        ControlOp::End.execute(&mut context).unwrap();
        assert_eq!(context.get_block_depth(), 0);

        // Test loop instruction
        let loop_type = ControlBlockType::ValueType(None);
        ControlOp::Loop(loop_type).execute(&mut context).unwrap();
        assert_eq!(context.get_block_depth(), 1);

        // Check the block type
        match &context.blocks[0] {
            Block::Loop(_) => {} // Correct block type
            _ => panic!("Expected Loop block"),
        }
    }

    #[test]
    fn test_if_else() {
        let mut context = MockControlContext::new();

        // Test if instruction with true condition
        context.push_control_value(Value::I32(1)).unwrap(); // True condition
        let if_type = ControlBlockType::ValueType(None);
        ControlOp::If(if_type.clone()).execute(&mut context).unwrap();
        assert_eq!(context.get_block_depth(), 1);

        // Test else instruction
        ControlOp::Else.execute(&mut context).unwrap();
        assert_eq!(context.get_block_depth(), 1); // Still in the same block

        // Test end instruction
        ControlOp::End.execute(&mut context).unwrap();
        assert_eq!(context.get_block_depth(), 0);

        // Test if instruction with false condition
        context.push_control_value(Value::I32(0)).unwrap(); // False condition
        ControlOp::If(if_type).execute(&mut context).unwrap();
        assert_eq!(context.get_block_depth(), 1);
    }

    #[test]
    fn test_branching() {
        let mut context = MockControlContext::new();

        // Test br instruction
        ControlOp::Br(1).execute(&mut context).unwrap();
        assert!(context.branched.is_some());
        assert_eq!(context.branched.unwrap().label_idx, 1);

        // Reset branched flag
        context.branched = None;

        // Test br_if instruction with true condition
        context.push_control_value(Value::I32(1)).unwrap(); // True condition
        ControlOp::BrIf(2).execute(&mut context).unwrap();
        assert!(context.branched.is_some());
        assert_eq!(context.branched.unwrap().label_idx, 2);

        // Reset branched flag
        context.branched = None;

        // Test br_if instruction with false condition
        context.push_control_value(Value::I32(0)).unwrap(); // False condition
        ControlOp::BrIf(3).execute(&mut context).unwrap();
        assert!(context.branched.is_none()); // Should not branch
    }

    #[test]
    fn test_br_table() {
        let mut context = MockControlContext::new();

        // Test br_table instruction with in-range index
        context.push_control_value(Value::I32(1)).unwrap(); // Index 1
        let mut table = Vec::new();
        table.push(10);
        table.push(20);
        table.push(30);
        let default = 99;
        ControlOp::BrTable { table: table.clone(), default }.execute(&mut context).unwrap();
        assert!(context.branched.is_some());
        assert_eq!(context.branched.unwrap().label_idx, 20); // table[1]

        // Reset branched flag
        context.branched = None;

        // Test br_table instruction with out-of-range index
        context.push_control_value(Value::I32(5)).unwrap(); // Index out of range
        ControlOp::BrTable { table, default }.execute(&mut context).unwrap();
        assert!(context.branched.is_some());
        assert_eq!(context.branched.unwrap().label_idx, 99); // default
    }

    #[test]
    fn test_function_control() {
        let mut context = MockControlContext::new();

        // Test return instruction
        ControlOp::Return.execute(&mut context).unwrap();
        assert!(context.returned);

        // Test call instruction
        ControlOp::Call(42).execute(&mut context).unwrap();
        assert_eq!(context.func_called, Some(42));

        // Test call_indirect instruction
        ControlOp::CallIndirect { table_idx: 1, type_idx: 5 }.execute(&mut context).unwrap();
        assert_eq!(context.indirect_call, Some((1, 5)));
    }

    #[test]
    fn test_other_control() {
        let mut context = MockControlContext::new();

        // Test nop instruction
        ControlOp::Nop.execute(&mut context).unwrap();

        // Test unreachable instruction
        let result = ControlOp::Unreachable.execute(&mut context);
        assert!(result.is_err());
        assert!(context.trapped);
    }

    #[test]
    fn test_individual_control_flow_operations() {
        // We'll create a simpler test context to avoid trait issues
        println!("Testing individual control flow operations");
        
        // Test Return creation
        let return_op = Return::new();
        assert_eq!(return_op, Return::default());
        
        // Test CallIndirect creation
        let call_indirect = CallIndirect::new(0, 1);
        assert_eq!(call_indirect.table_idx, 0);
        assert_eq!(call_indirect.type_idx, 1);
        
        // Test BrTable creation from slice
        let br_table = BrTable::from_slice(&[1, 2, 3], 99);
        assert!(br_table.is_ok());
        let table = br_table.unwrap();
        assert_eq!(table.default, 99);
    }
}


