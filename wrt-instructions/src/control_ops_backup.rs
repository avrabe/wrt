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

use crate::prelude::*;


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
#[derive(Debug, Clone)]
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
        /// Table of branch target labels (no_std)
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
    /// Execute a nop instruction (no operation)
    Nop,
    /// Execute an unreachable instruction (causes trap)
    Unreachable,
}

/// Return operation (return)
#[derive(Debug, Clone, PartialEq)]
pub struct Return;

impl Return {
    /// Create a new return operation
    pub fn new() -> Self {
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

/// Call indirect operation (call_indirect)
#[derive(Debug, Clone, PartialEq)]
pub struct CallIndirect {
    /// Table index to use for the indirect call
    pub table_idx: u32,
    /// Expected function type index
    pub type_idx: u32,
}

impl CallIndirect {
    /// Create a new call_indirect operation
    pub fn new(table_idx: u32, type_idx: u32) -> Self {
        Self { table_idx, type_idx }
    }
    
    /// Execute call_indirect operation
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

/// Branch table operation (br_table)
#[derive(Debug, Clone, PartialEq)]
pub struct BrTable {
    /// Table of branch target labels
    #[cfg(feature = "std")]
    pub table: Vec<u32>,
    
    #[cfg(not(any(feature = "std", )))]
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
    
    /// Create a new br_table operation with BoundedVec (no_std)
    #[cfg(not(any(feature = "std", )))]
    pub fn new_bounded(
        table: wrt_foundation::BoundedVec<u32, 256, wrt_foundation::NoStdProvider<8192>>, 
        default: u32
    ) -> Self {
        Self { table, default }
    }
    
    /// Create a br_table from a slice (works in all environments)
    pub fn from_slice(table_slice: &[u32], default: u32) -> Result<Self> {
        #[cfg(feature = "std")]
        {
            Ok(Self {
                table: table_slice.to_vec(),
                default,
            })
        }
        #[cfg(not(any(feature = "std", )))]
        {
            let guard = safe_managed_alloc!(8192, CrateId::Instructions)?;

            let provider = unsafe { guard.release() };\n            let mut table = wrt_foundation::BoundedVec::new(provider).map_err(|_| {\n                Error::memory_error(\"Could not create BoundedVec\")\n            })?;
            for &label in table_slice {
                table.push(label).map_err(|_| {
                    Error::memory_error("Branch table exceeds maximum size")
                })?;
            }
            Ok(Self { table, default })
        }
    }
    
    /// Execute br_table operation
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
        
        // Convert to slice for unified execution
        #[cfg(feature = "std")]
        let table_slice = self.table.as_slice();
        #[cfg(not(any(feature = "std", )))]
        let table_slice = {
            let mut slice_vec = [0u32; 256]; // Static array for no_std
            let len = core::cmp::min(self.table.len(), 256);
            for i in 0..len {
                slice_vec[i] = self.table.get(i).map_err(|_| {
                    Error::runtime_error("Branch table index out of bounds")
                })?;
            }
            &slice_vec[..len]
        };
        
        // Execute the branch table operation
        context.execute_br_table(table_slice, self.default, index)
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

    /// Trap the execution (unreachable)
    fn trap(&mut self, message: &str) -> Result<()>;

    /// Get the current block
    fn get_current_block(&self) -> Option<&Block>;
    
    /// Get function operations interface
    fn get_function_operations(&mut self) -> Result<&mut dyn FunctionOperations>;
    
    /// Execute function return with value handling
    fn execute_return(&mut self) -> Result<()>;
    
    /// Execute call_indirect with full validation
    fn execute_call_indirect(&mut self, table_idx: u32, type_idx: u32, func_idx: i32) -> Result<()>;
    
    /// Execute branch table operation
    fn execute_br_table(&mut self, table: &[u32], default: u32, index: i32) -> Result<()>;
}

impl<T: ControlContext> PureInstruction<T, Error> for ControlOp {
    fn execute(&self, context: &mut T) -> Result<()> {
        match self {
            Self::Block(block_type) => context.enter_block(Block::Block(block_type.clone())),
            Self::Loop(block_type) => context.enter_block(Block::Loop(block_type.clone())),
            Self::If(block_type) => {
                let condition = context.pop_control_value()?.into_i32().map_err(|_| {
                    Error::new(ErrorCategory::Type, codes::INVALID_TYPE, "Expected I32 for if condition")
                })?;

                if condition != 0 {
                    // Condition is true, enter the if block
                    context.enter_block(Block::If(block_type.clone()))
                } else {
                    // Condition is false, skip to the else or end
                    // The runtime will handle this by setting a flag to skip instructions
                    // until the corresponding else or end is found
                    context.enter_block(Block::If(block_type.clone()))
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
                    Error::new(ErrorCategory::Type, codes::INVALID_TYPE, "Expected I32 for br_if condition")
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
                #[cfg(feature = "std")]
                let br_table = BrTable::new(table.clone(), *default);
                #[cfg(not(feature = "std"))]
                let br_table = {
                    let guard = safe_managed_alloc!(8192, CrateId::Instructions)?;

                    let provider = unsafe { guard.release() };\n                    let mut bounded_table = wrt_foundation::BoundedVec::new(provider).map_err(|_| {\n                        Error::new(ErrorCategory::Runtime, codes::MEMORY_ERROR, \"Could not create BoundedVec\")\n                    })?;
                    for &label in table.iter() {
                        bounded_table.push(label).map_err(|_| {
                            Error::new(ErrorCategory::Runtime, codes::MEMORY_ERROR, "Branch table too large")
                        })?;
                    }
                    BrTable::new_bounded(bounded_table, *default)
                };
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
            Self::Nop => {
                // No operation, just return Ok
                Ok(())
            }
            Self::Unreachable => {
                // The unreachable instruction unconditionally traps
                context.trap("unreachable instruction executed")
            }
        }
    }
}

#[cfg(all(test, any(feature = "std", )))]
mod tests {
        use std::vec;
        use std::vec::Vec;
    // Import Vec and vec! based on feature flags
    #[cfg(feature = "std")]
    use std::vec::Vec;

    use wrt_foundation::{types::ValueType, safe_managed_alloc};

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
                Error::new(ErrorCategory::Runtime, codes::STACK_UNDERFLOW, "Stack underflow")
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
                Error::new(
                    ErrorCategory::Runtime,
                    codes::EXECUTION_ERROR,
                    "Invalid branch target",
                )
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
            Err(Error::new(ErrorCategory::Runtime, codes::EXECUTION_ERROR, "Execution trapped"))
        }

        fn get_current_block(&self) -> Option<&Block> {
            self.blocks.last()
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


