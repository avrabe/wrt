//! Control flow operations for WebAssembly instructions.
//!
//! This module provides pure implementations for WebAssembly control flow instructions,
//! including block, loop, if, branch, and return operations.

#![allow(clippy::match_single_binding)]

use crate::prelude::*;
use wrt_types::types::{FuncType, ValueType};

/// Branch target information
#[derive(Debug, Clone)]
pub struct BranchTarget {
    /// The label index to branch to
    pub label_idx: u32,
    /// The number of values to keep when branching
    pub keep_values: usize,
}

/// Block types for control flow operations
#[derive(Debug, Clone)]
pub enum ControlBlockType {
    /// Block with a specific function type (input/output types)
    FuncType(FuncType),
    /// Block with a single output type
    ValueType(Option<ValueType>),
}

impl From<BlockType> for ControlBlockType {
    fn from(bt: BlockType) -> Self {
        match bt {
            BlockType::Empty => ControlBlockType::ValueType(None),
            BlockType::Value(vt) => ControlBlockType::ValueType(Some(vt)),
            BlockType::FuncType(ft) => ControlBlockType::FuncType(ft),
            BlockType::TypeIndex(_) => ControlBlockType::ValueType(None), // Default handling for type indices
        }
    }
}

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
        table: Vec<u32>,
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
}

impl<T: ControlContext> PureInstruction<T, Error> for ControlOp {
    fn execute(&self, context: &mut T) -> Result<()> {
        match self {
            Self::Block(block_type) => context.enter_block(Block::Block(block_type.clone())),
            Self::Loop(block_type) => context.enter_block(Block::Loop(block_type.clone())),
            Self::If(block_type) => {
                let condition = context.pop_control_value()?.as_i32().ok_or_else(|| {
                    Error::invalid_type("Expected I32 for if condition".to_string())
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
                let condition = context.pop_control_value()?.as_i32().ok_or_else(|| {
                    Error::invalid_type("Expected I32 for br_if condition".to_string())
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
                let index = context.pop_control_value()?.as_i32().ok_or_else(|| {
                    Error::invalid_type("Expected I32 for br_table index".to_string())
                })?;

                // Determine which label to branch to
                let label_idx = if index >= 0 && (index as usize) < table.len() {
                    table[index as usize]
                } else {
                    *default
                };

                let target = BranchTarget {
                    label_idx,
                    keep_values: 0, // The runtime will resolve this based on block types
                };
                context.branch(target)
            }
            Self::Return => context.return_function(),
            Self::Call(func_idx) => context.call_function(*func_idx),
            Self::CallIndirect { table_idx, type_idx } => {
                context.call_indirect(*table_idx, *type_idx)
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

#[cfg(test)]
mod tests {
    use super::*;
    use wrt_types::types::ValueType;

    // Import Vec and vec! based on feature flags
    #[cfg(feature = "std")]
    use std::vec::Vec;

    #[cfg(all(not(feature = "std"), feature = "alloc"))]
    use alloc::vec::Vec;

    #[cfg(all(not(feature = "std"), feature = "alloc"))]
    use alloc::vec;

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
                    format!("Invalid branch target with depth: {}", self.blocks.len()),
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
        let table = vec![10, 20, 30];
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
}
