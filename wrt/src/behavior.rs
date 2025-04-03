use crate::StacklessEngine;
use crate::{
    error::{Error, Result},
    global::Global,
    memory::{DefaultMemory, MemoryBehavior},
    stack::Stack,
    table::Table,
    types::{BlockType, FuncType, GlobalType, ValueType},
    values::Value,
    Vec,
};
use std::sync::Arc;
use std::sync::MutexGuard;

/// Defines the basic behavior of a value stack.
pub trait StackBehavior: std::fmt::Debug {
    /// Pushes a value onto the stack.
    fn push(&mut self, value: Value) -> Result<()>;
    /// Pops a value from the stack.
    fn pop(&mut self) -> Result<Value>;

    /// Pops a value and expects it to be a boolean (i32, 0 or 1).
    fn pop_bool(&mut self) -> Result<bool> {
        match self.pop()? {
            Value::I32(0) => Ok(false),
            Value::I32(1) => Ok(true),
            _ => Err(Error::InvalidType(
                "Expected boolean (i32 0 or 1)".to_string(),
            )),
        }
    }

    /// Pops a value and expects it to be an i32.
    fn pop_i32(&mut self) -> Result<i32> {
        match self.pop()? {
            Value::I32(v) => Ok(v),
            _ => Err(Error::InvalidType("Expected i32".to_string())),
        }
    }

    /// Pops a value and expects it to be a v128.
    fn pop_v128(&mut self) -> Result<[u8; 16]> {
        match self.pop()? {
            Value::V128(bytes) => Ok(bytes),
            other => Err(Error::InvalidType(format!(
                "Expected v128, found {}",
                other.type_()
            ))),
        }
    }

    /// Pops a value and expects it to be an i64.
    fn pop_i64(&mut self) -> Result<i64> {
        match self.pop()? {
            Value::I64(val) => Ok(val),
            other => Err(Error::InvalidType(format!(
                "Expected i64, found {}",
                other.type_()
            ))),
        }
    }

    /// Returns a reference to the top value on the stack without removing it.
    fn peek(&self) -> Result<&Value>;
    /// Returns a mutable reference to the top value on the stack without removing it.
    fn peek_mut(&mut self) -> Result<&mut Value>;
    /// Returns a slice containing all values currently on the stack.
    fn values(&self) -> &[Value];
    /// Returns a mutable slice containing all values currently on the stack.
    fn values_mut(&mut self) -> &mut [Value];
    /// Returns the number of values on the stack.
    fn len(&self) -> usize;
    /// Returns `true` if the stack contains no values.
    fn is_empty(&self) -> bool;

    /// Pushes a label onto the conceptual label stack (implementation specific).
    fn push_label(&mut self, arity: usize, pc: usize);
    /// Pops a label from the conceptual label stack (implementation specific).
    fn pop_label(&mut self) -> Result<Label>;
    /// Gets a reference to a label by index from the conceptual label stack (implementation specific).
    fn get_label(&self, index: usize) -> Option<&Label>;
}

/// Trait for accessing the frame state
pub trait FrameBehavior: ControlFlowBehavior {
    /// Get locals
    fn locals(&mut self) -> &mut Vec<Value>;

    /// Get a local variable by index
    fn get_local(&self, idx: usize) -> Result<Value>;

    /// Set a local variable by index
    fn set_local(&mut self, idx: usize, value: Value) -> Result<()>;

    /// Get a global variable by index (returns Arc)
    fn get_global(&self, idx: usize) -> Result<Arc<Global>>;

    /// Set a global variable by index (takes &self due to interior mutability)
    fn set_global(&mut self, idx: usize, value: Value) -> Result<()>;

    /// Get a memory instance by index
    fn get_memory(&self, idx: usize) -> Result<Arc<dyn MemoryBehavior>>;

    /// Get a mutable memory instance by index
    fn get_memory_mut(&mut self, idx: usize) -> Result<Arc<dyn MemoryBehavior>>;

    /// Get a table instance by index (returns Arc)
    fn get_table(&self, idx: usize) -> Result<Arc<Table>>;

    /// Get a mutable table instance by index (added)
    fn get_table_mut(&mut self, idx: usize) -> Result<Arc<Table>>;

    /// Get the function type for a given function index
    fn get_function_type(&self, func_idx: u32) -> Result<FuncType>;

    /// Get the current program counter
    fn pc(&self) -> usize;

    /// Set the program counter
    fn set_pc(&mut self, pc: usize);

    /// Get the current function index
    fn func_idx(&self) -> u32;

    /// Get the current instance index
    fn instance_idx(&self) -> u32;

    /// Get the number of locals
    fn locals_len(&self) -> usize;

    /// Get mutable access to the label stack
    fn label_stack(&mut self) -> &mut Vec<Label>;

    /// Get the arity of the current block/frame
    fn arity(&self) -> usize;

    /// Set the arity of the current block/frame
    fn set_arity(&mut self, arity: usize);

    /// Get the expected label arity for the current context
    fn label_arity(&self) -> usize;

    /// Get the return program counter for the current frame
    fn return_pc(&self) -> usize;

    /// Set the return program counter for the current frame
    fn set_return_pc(&mut self, pc: usize);

    /// Get the frame as a mutable Any reference for downcasting
    fn as_any(&mut self) -> &mut dyn std::any::Any;

    // Memory access methods (take &self due to interior mutability)
    fn load_i32(&self, addr: usize, align: u32) -> Result<i32>;
    fn load_i64(&self, addr: usize, align: u32) -> Result<i64>;
    fn load_f32(&self, addr: usize, align: u32) -> Result<f32>;
    fn load_f64(&self, addr: usize, align: u32) -> Result<f64>;
    fn load_i8(&self, addr: usize, align: u32) -> Result<i8>;
    fn load_u8(&self, addr: usize, align: u32) -> Result<u8>;
    fn load_i16(&self, addr: usize, align: u32) -> Result<i16>;
    fn load_u16(&self, addr: usize, align: u32) -> Result<u16>;
    fn load_v128(&self, addr: usize, align: u32) -> Result<[u8; 16]>;
    fn store_i32(&mut self, addr: usize, align: u32, value: i32) -> Result<()>;
    fn store_i64(&mut self, addr: usize, align: u32, value: i64) -> Result<()>;
    fn store_f32(&mut self, addr: usize, align: u32, value: f32) -> Result<()>;
    fn store_f64(&mut self, addr: usize, align: u32, value: f64) -> Result<()>;
    fn store_i8(&mut self, addr: usize, align: u32, value: i8) -> Result<()>;
    fn store_u8(&mut self, addr: usize, align: u32, value: u8) -> Result<()>;
    fn store_i16(&mut self, addr: usize, align: u32, value: i16) -> Result<()>;
    fn store_u16(&mut self, addr: usize, align: u32, value: u16) -> Result<()>;
    fn store_v128(&mut self, addr: usize, align: u32, value: [u8; 16]) -> Result<()>;
    fn memory_size(&self) -> Result<u32>;
    fn memory_grow(&mut self, pages: u32) -> Result<u32>;

    // Table access methods (take &self due to interior mutability)
    fn table_get(&self, table_idx: u32, idx: u32) -> Result<Value>;
    fn table_set(&mut self, table_idx: u32, idx: u32, value: Value) -> Result<()>;
    fn table_size(&self, table_idx: u32) -> Result<u32>;
    fn table_grow(&mut self, table_idx: u32, delta: u32, value: Value) -> Result<u32>;
    fn table_init(
        &mut self,
        table_idx: u32,
        elem_idx: u32,
        dst: u32,
        src: u32,
        n: u32,
    ) -> Result<()>;
    fn table_copy(
        &mut self,
        dst_table: u32,
        src_table: u32,
        dst: u32,
        src: u32,
        n: u32,
    ) -> Result<()>;
    fn elem_drop(&mut self, elem_idx: u32) -> Result<()>;
    fn table_fill(&mut self, table_idx: u32, dst: u32, val: Value, n: u32) -> Result<()>;

    // Stack interaction helpers (might not belong here, could be separate trait?)
    // These might still need &mut self if they directly manipulate a mutable stack reference
    fn pop_bool(&mut self, stack: &mut dyn Stack) -> Result<bool>;
    fn pop_i32(&mut self, stack: &mut dyn Stack) -> Result<i32>;

    /// Get two tables and return a tuple of MutexGuard<Table>
    fn get_two_tables_mut(
        &mut self,
        _idx1: u32,
        _idx2: u32,
    ) -> Result<(MutexGuard<Table>, MutexGuard<Table>)>;

    /// Add method to get instance index
    fn instance_idx(&self) -> u32;
}

/// Defines behaviors related to control flow instructions.
pub trait ControlFlowBehavior {
    /// Called when entering a `block` instruction.
    fn enter_block(&mut self, ty: BlockType, stack_len: usize) -> Result<()>;
    /// Called when entering a `loop` instruction.
    fn enter_loop(&mut self, ty: BlockType, stack_len: usize) -> Result<()>;
    /// Called when entering an `if` instruction.
    fn enter_if(&mut self, ty: BlockType, stack_len: usize, condition: bool) -> Result<()>;
    /// Called when entering an `else` branch.
    fn enter_else(&mut self, stack_len: usize) -> Result<()>;
    /// Called when exiting a block (`end` instruction).
    fn exit_block(&mut self, stack: &mut dyn Stack) -> Result<()>;
    /// Called for `br` and `br_if` instructions.
    fn branch(&mut self, label_idx: u32, stack: &mut dyn Stack) -> Result<()>;
    /// Called for the `return` instruction.
    fn return_(&mut self, stack: &mut dyn Stack) -> Result<()>;
    /// Called for the `call` instruction.
    fn call(&mut self, func_idx: u32, stack: &mut dyn Stack) -> Result<()>;
    /// Called for the `call_indirect` instruction.
    fn call_indirect(
        &mut self,
        type_idx: u32,
        table_idx: u32,
        entry: u32,
        stack: &mut dyn Stack,
    ) -> Result<()>;
    /// Sets the arity for the current label (used for stack validation).
    fn set_label_arity(&mut self, arity: usize);
}

/// Trait for executing WebAssembly instructions
pub trait InstructionExecutor: std::fmt::Debug {
    /// Execute the instruction in the given context
    ///
    /// # Arguments
    /// * `stack` - The execution stack
    /// * `frame` - The current execution frame
    /// * `engine` - The stackless engine
    ///
    /// # Returns
    /// * `Ok(())` - If the instruction executed successfully
    /// * `Err(Error)` - If an error occurred
    fn execute(
        &self,
        stack: &mut dyn Stack,
        frame: &mut dyn FrameBehavior,
        engine: &StacklessEngine,
    ) -> Result<()>;
}

/// Represents a control-flow label used by behavior traits.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Label {
    /// The number of values expected on the stack after the block corresponding to this label completes.
    pub arity: usize,
    /// The program counter (instruction index) pointing to the instruction *after* the block's end.
    pub pc: usize,
    /// The program counter (instruction index) pointing to the continuation of the block (e.g., the `else` part of an `if`).
    pub continuation: usize,
}

impl From<crate::stack::Label> for Label {
    fn from(label: crate::stack::Label) -> Self {
        Self {
            arity: label.arity,
            pc: label.pc,
            continuation: label.continuation,
        }
    }
}

/// Represents the execution context (frame) used by behavior traits.
#[derive(Debug, Clone, Default)]
pub struct Frame {
    /// Local variables, including arguments, for the current function frame.
    pub locals: Vec<Value>,
    /// The program counter, indicating the next instruction to execute within the current function.
    pub pc: usize,
    /// The index of the function currently being executed.
    pub func_idx: u32,
    /// The number of return values expected by the caller of the current function.
    pub arity: usize,
    /// The arity (number of expected stack values) of the innermost control flow block.
    pub label_arity: usize,
    /// The program counter in the caller function to return to after the current function completes.
    pub return_pc: usize,
    /// The stack of active control flow labels (`block`, `loop`, `if`).
    pub label_stack: Vec<Label>,
}

/// A behavior that does nothing and returns default values
#[derive(Debug, Default)]
pub struct NullBehavior {
    pub locals: Vec<Value>,
    pub pc: usize,
    pub func_idx: u32,
    pub arity: usize,
    pub label_arity: usize,
    pub return_pc: usize,
    pub label_stack: Vec<Label>,
}

impl FrameBehavior for NullBehavior {
    fn locals(&mut self) -> &mut Vec<Value> {
        &mut self.locals
    }

    fn get_local(&self, idx: usize) -> Result<Value> {
        self.locals
            .get(idx)
            .cloned()
            .ok_or_else(|| Error::InvalidLocal(format!("Local index out of bounds: {idx}")))
    }

    fn as_any(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn set_local(&mut self, idx: usize, value: Value) -> Result<()> {
        if idx < self.locals.len() {
            self.locals[idx] = value;
            Ok(())
        } else {
            Err(Error::InvalidLocalIndex(idx))
        }
    }

    fn get_global(&self, _idx: usize) -> Result<Arc<Global>> {
        Err(Error::InvalidGlobalIndex(_idx))
    }

    fn set_global(&mut self, _idx: usize, _value: Value) -> Result<()> {
        Err(Error::InvalidGlobalIndex(_idx))
    }

    fn get_memory(&self, _idx: usize) -> Result<Arc<dyn MemoryBehavior>> {
        Err(Error::InvalidMemoryIndex(_idx))
    }

    fn get_memory_mut(&mut self, _idx: usize) -> Result<Arc<dyn MemoryBehavior>> {
        Err(Error::InvalidMemoryIndex(_idx))
    }

    fn get_table(&self, _idx: usize) -> Result<Arc<Table>> {
        Err(Error::InvalidTableIndex(_idx))
    }

    fn get_table_mut(&mut self, idx: usize) -> Result<Arc<Table>> {
        Err(Error::InvalidTableIndex(idx))
    }

    fn pc(&self) -> usize {
        self.pc
    }

    fn set_pc(&mut self, pc: usize) {
        self.pc = pc;
    }

    fn func_idx(&self) -> u32 {
        self.func_idx
    }

    fn instance_idx(&self) -> u32 {
        0
    }

    fn locals_len(&self) -> usize {
        self.locals.len()
    }

    fn label_stack(&mut self) -> &mut Vec<Label> {
        &mut self.label_stack
    }

    fn arity(&self) -> usize {
        self.arity
    }

    fn set_arity(&mut self, arity: usize) {
        self.arity = arity;
    }

    fn label_arity(&self) -> usize {
        self.label_arity
    }

    fn return_pc(&self) -> usize {
        self.return_pc
    }

    fn set_return_pc(&mut self, pc: usize) {
        self.return_pc = pc;
    }

    fn load_i32(&self, _addr: usize, _align: u32) -> Result<i32> {
        Err(Error::InvalidMemoryIndex(0))
    }

    fn load_i64(&self, _addr: usize, _align: u32) -> Result<i64> {
        Err(Error::InvalidMemoryIndex(0))
    }

    fn load_f32(&self, _addr: usize, _align: u32) -> Result<f32> {
        Err(Error::InvalidMemoryIndex(0))
    }

    fn load_f64(&self, _addr: usize, _align: u32) -> Result<f64> {
        Err(Error::InvalidMemoryIndex(0))
    }

    fn load_i8(&self, _addr: usize, _align: u32) -> Result<i8> {
        Err(Error::InvalidMemoryIndex(0))
    }

    fn load_u8(&self, _addr: usize, _align: u32) -> Result<u8> {
        Err(Error::InvalidMemoryIndex(0))
    }

    fn load_i16(&self, _addr: usize, _align: u32) -> Result<i16> {
        Err(Error::InvalidMemoryIndex(0))
    }

    fn load_u16(&self, _addr: usize, _align: u32) -> Result<u16> {
        Err(Error::InvalidMemoryIndex(0))
    }

    fn load_v128(&self, _addr: usize, _align: u32) -> Result<[u8; 16]> {
        Err(Error::InvalidMemoryIndex(0))
    }

    fn store_i32(&mut self, _addr: usize, _align: u32, _value: i32) -> Result<()> {
        Err(Error::InvalidMemoryIndex(0))
    }

    fn store_i64(&mut self, _addr: usize, _align: u32, _value: i64) -> Result<()> {
        Err(Error::InvalidMemoryIndex(0))
    }

    fn store_f32(&mut self, _addr: usize, _align: u32, _value: f32) -> Result<()> {
        Err(Error::InvalidMemoryIndex(0))
    }

    fn store_f64(&mut self, _addr: usize, _align: u32, _value: f64) -> Result<()> {
        Err(Error::InvalidMemoryIndex(0))
    }

    fn store_i8(&mut self, _addr: usize, _align: u32, _value: i8) -> Result<()> {
        Err(Error::InvalidMemoryIndex(0))
    }

    fn store_u8(&mut self, _addr: usize, _align: u32, _value: u8) -> Result<()> {
        Err(Error::InvalidMemoryIndex(0))
    }

    fn store_i16(&mut self, _addr: usize, _align: u32, _value: i16) -> Result<()> {
        Err(Error::InvalidMemoryIndex(0))
    }

    fn store_u16(&mut self, _addr: usize, _align: u32, _value: u16) -> Result<()> {
        Err(Error::InvalidMemoryIndex(0))
    }

    fn store_v128(&mut self, _addr: usize, _align: u32, _value: [u8; 16]) -> Result<()> {
        Err(Error::InvalidMemoryIndex(0))
    }

    fn memory_size(&self) -> Result<u32> {
        Err(Error::InvalidMemoryIndex(0))
    }

    fn memory_grow(&mut self, _pages: u32) -> Result<u32> {
        Err(Error::InvalidMemoryIndex(0))
    }

    fn table_get(&self, _table_idx: u32, _idx: u32) -> Result<Value> {
        Err(Error::InvalidTableIndex(_table_idx as usize))
    }

    fn table_set(&mut self, _table_idx: u32, _idx: u32, _value: Value) -> Result<()> {
        Err(Error::InvalidTableIndex(_table_idx as usize))
    }

    fn table_size(&self, _table_idx: u32) -> Result<u32> {
        Err(Error::InvalidTableIndex(_table_idx as usize))
    }

    fn table_grow(&mut self, _table_idx: u32, _delta: u32, _value: Value) -> Result<u32> {
        Err(Error::InvalidTableIndex(_table_idx as usize))
    }

    fn table_init(
        &mut self,
        _table_idx: u32,
        _elem_idx: u32,
        _dst: u32,
        _src: u32,
        _n: u32,
    ) -> Result<()> {
        Err(Error::InvalidTableIndex(_table_idx as usize))
    }

    fn table_copy(
        &mut self,
        _dst_table: u32,
        _src_table: u32,
        _dst: u32,
        _src: u32,
        _n: u32,
    ) -> Result<()> {
        Err(Error::InvalidTableIndex(_dst_table as usize))
    }

    fn elem_drop(&mut self, _elem_idx: u32) -> Result<()> {
        Err(Error::Unimplemented("elem_drop NullBehavior".to_string()))
    }

    fn table_fill(&mut self, _table_idx: u32, _dst: u32, _val: Value, _n: u32) -> Result<()> {
        Err(Error::InvalidTableIndex(_table_idx as usize))
    }

    fn pop_bool(&mut self, stack: &mut dyn Stack) -> Result<bool> {
        stack.pop_bool()
    }

    fn pop_i32(&mut self, stack: &mut dyn Stack) -> Result<i32> {
        stack.pop_i32()
    }

    fn get_function_type(&self, func_idx: u32) -> Result<FuncType> {
        Err(Error::InvalidFunctionIndex(func_idx as usize))
    }

    fn get_two_tables_mut(
        &mut self,
        _idx1: u32,
        _idx2: u32,
    ) -> Result<(MutexGuard<Table>, MutexGuard<Table>)> {
        unimplemented!()
    }
}

impl ControlFlowBehavior for NullBehavior {
    fn enter_block(&mut self, _ty: BlockType, _stack_len: usize) -> Result<()> {
        Ok(())
    }
    fn enter_loop(&mut self, _ty: BlockType, _stack_len: usize) -> Result<()> {
        Ok(())
    }
    fn enter_if(&mut self, _ty: BlockType, _stack_len: usize, _condition: bool) -> Result<()> {
        Ok(())
    }
    fn enter_else(&mut self, _stack_len: usize) -> Result<()> {
        Ok(())
    }
    fn exit_block(&mut self, _stack: &mut dyn Stack) -> Result<()> {
        Ok(())
    }
    fn branch(&mut self, _label_idx: u32, _stack: &mut dyn Stack) -> Result<()> {
        Ok(())
    }
    fn return_(&mut self, _stack: &mut dyn Stack) -> Result<()> {
        Ok(())
    }
    fn call(&mut self, _func_idx: u32, _stack: &mut dyn Stack) -> Result<()> {
        Ok(())
    }
    fn call_indirect(
        &mut self,
        _type_idx: u32,
        _table_idx: u32,
        _entry: u32,
        _stack: &mut dyn Stack,
    ) -> Result<()> {
        Ok(())
    }
    fn set_label_arity(&mut self, _arity: usize) {}
}
