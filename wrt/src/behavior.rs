//! Behavior traits defining the core interfaces for the WebAssembly runtime engine.

// use std::any::Any; // Covered by prelude
// use std::sync::Arc; // Covered by prelude

use crate::prelude::*; // Should bring in Value, FuncType, Error, Result, Arc, Vec, Box, Mutex, MutexGuard, etc.

use crate::{
    global::Global, // This will be wrt::global::Global - needs to become prelude::Global from wrt_runtime
    module::{Data, Element, Function}, // These need to be prelude versions from wrt_runtime
    stackless::StacklessEngine, // This is wrt::stackless::StacklessEngine - needs to become prelude::StacklessEngine from wrt_runtime
};

// No longer needed due to prelude::*
// use wrt_error::{Error, Result, kinds}; 

// No longer needed due to prelude::*
// use crate::prelude::{
//     BlockType, FuncType, InstructionType, Limits, MemoryType, Mutex, MutexGuard,
//     TypesValue as Value,
// };

// These should come from prelude (wrt_runtime::Memory, wrt_runtime::Table)
use wrt_runtime::Memory; 
use wrt_runtime::Table;
// SafeSlice is in prelude from wrt_types::safe_memory
// use wrt_types::safe_memory::SafeSlice;

/// Represents the outcome of executing a single instruction, guiding the engine's next action.
#[derive(Debug)]
pub enum ControlFlow {
    /// Continue to the next instruction sequentially.
    Continue,
    /// Branch to a different instruction PC, potentially adjusting the stack.
    Branch {
        target_pc: usize,
        values_to_keep: usize,
    },
    /// Return from the current function call.
    Return { values: Vec<Value> },
    /// Initiate a new function call.
    Call {
        func_idx: u32,
        args: Vec<Value>,
        return_pc: usize,
    },
    /// Halt execution due to a trap.
    Trap(Error),
}

/// Defines the basic behavior of a value stack.
pub trait StackBehavior: std::fmt::Debug {
    /// Pushes a value onto the stack.
    fn push(&mut self, value: Value) -> Result<(), Error>;
    /// Pops a value from the stack.
    fn pop(&mut self) -> Result<Value, Error>;

    /// Pops a value and expects it to be a boolean (i32, 0 or 1).
    fn pop_bool(&mut self) -> Result<bool, Error> {
        match self.pop()? {
            Value::I32(0) => Ok(false),
            Value::I32(1) => Ok(true),
            _ => Err(Error::new(prelude::ErrorCategory::Type, prelude::codes::TYPE_MISMATCH_ERROR, "Expected boolean (i32 0 or 1)".to_string())),
        }
    }

    /// Pops a value and expects it to be an i32.
    fn pop_i32(&mut self) -> Result<i32, Error> {
        match self.pop()? {
            Value::I32(v) => Ok(v),
            _ => Err(Error::new(prelude::ErrorCategory::Type, prelude::codes::TYPE_MISMATCH_ERROR, "Expected i32".to_string())),
        }
    }

    /// Pops a value and expects it to be a v128.
    fn pop_v128(&mut self) -> Result<[u8; 16], Error> {
        match self.pop()? {
            Value::V128(bytes) => Ok(bytes),
            other => Err(Error::new(prelude::ErrorCategory::Type, prelude::codes::TYPE_MISMATCH_ERROR, format!("Expected v128, found {}", other.value_type()))),
        }
    }

    /// Pops a value and expects it to be an i64.
    fn pop_i64(&mut self) -> Result<i64, Error> {
        match self.pop()? {
            Value::I64(val) => Ok(val),
            other => Err(Error::new(prelude::ErrorCategory::Type, prelude::codes::TYPE_MISMATCH_ERROR, format!("Expected i64, found {}", other.value_type()))),
        }
    }

    /// Returns a reference to the top value on the stack without removing it.
    fn peek(&self) -> Result<&Value, Error>;
    /// Returns a mutable reference to the top value on the stack without removing it.
    fn peek_mut(&mut self) -> Result<&mut Value, Error>;
    /// Returns a slice containing all values currently on the stack.
    fn values(&self) -> &[Value];
    /// Returns a mutable slice containing all values currently on the stack.
    fn values_mut(&mut self) -> &mut [Value];
    /// Returns the number of values on the stack.
    fn len(&self) -> usize;
    /// Returns `true` if the stack contains no values.
    fn is_empty(&self) -> bool;

    /// Pushes a label onto the conceptual label stack
    fn push_label(&mut self, label: Label) -> Result<(), Error>;

    /// Pops a label from the conceptual label stack
    fn pop_label(&mut self) -> Result<Label, Error>;

    /// Gets a reference to a label at the given depth (relative to top of stack)
    fn get_label(&self, depth: usize) -> Option<&Label>;

    /// Pushes multiple values onto the stack.
    fn push_n(&mut self, values: &[Value]);

    /// Pops `n` values from the stack.
    fn pop_n(&mut self, n: usize) -> Vec<Value>;

    /// Pops a label specifically associated with a frame boundary.
    fn pop_frame_label(&mut self) -> Result<Label, Error>;

    /// Executes a direct function call using the stack context.
    fn execute_function_call_direct(
        &mut self,
        engine: &mut StacklessEngine,
        caller_instance_idx: u32,
        func_idx: u32,
        args: Vec<Value>,
    ) -> Result<Vec<Value>, Error>;

    /// Get mutable reference as Any for downcasting
    fn as_any(&mut self) -> &mut dyn std::any::Any;
}

/// Trait for accessing the frame state
pub trait FrameBehavior: Send + Sync + std::fmt::Debug + Any {
    /// Get locals
    fn locals(&mut self) -> &mut Vec<Value>;

    /// Get a local variable by index
    fn get_local(&self, idx: usize) -> Result<Value, Error>;

    /// Set a local variable by index
    fn set_local(&mut self, idx: usize, value: Value) -> Result<(), Error>;

    /// Get a global variable by index (returns Arc)
    fn get_global(&self, idx: usize, engine: &StacklessEngine) -> Result<Arc<Global>, Error>;

    /// Set a global variable by index (takes &self due to interior mutability)
    fn set_global(
        &mut self,
        idx: usize,
        value: Value,
        engine: &StacklessEngine,
    ) -> Result<(), Error>;

    /// Get mutable access to a global's value
    fn get_global_mut(&mut self, idx: usize) -> Result<MutexGuard<Value>, Error>;

    /// Get a memory instance by index
    fn get_memory(&self, idx: usize, engine: &StacklessEngine) -> Result<Arc<Memory>, Error>;

    /// Get a mutable memory instance by index
    fn get_memory_mut(
        &mut self,
        idx: usize,
        engine: &StacklessEngine,
    ) -> Result<Arc<Memory>, Error>;

    /// Get a table instance by index (returns Arc)
    fn get_table(&self, idx: usize, engine: &StacklessEngine) -> Result<Arc<Table>, Error>;

    /// Get a mutable table instance by index (added)
    fn get_table_mut(&mut self, idx: usize, engine: &StacklessEngine) -> Result<Arc<Table>, Error>;

    /// Get the function type for a given function index (Doesn't need engine)
    fn get_function_type(&self, func_idx: u32) -> Result<FuncType, Error>;

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

    /// Get mutable access to the local values
    fn locals_mut(&mut self) -> &mut [Value];

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
    fn set_return_pc(&mut self, pc: Option<usize>);

    /// Get the frame as a mutable Any reference for downcasting
    fn as_any(&mut self) -> &mut dyn std::any::Any;

    /// Push a label onto the frame's label stack
    fn push_label(&mut self, label: Label) -> Result<(), Error>;

    /// Pop a label from the frame's label stack
    fn pop_label(&mut self) -> Result<Label, Error>;

    /// Get a reference to a label at the given depth (relative to top of stack)
    fn get_label(&self, depth: usize) -> Option<&Label>;

    // Memory access methods (need engine context)
    fn load_i32(&self, addr: usize, align: u32, engine: &StacklessEngine) -> Result<i32, Error>;
    fn load_i64(&self, addr: usize, align: u32, engine: &StacklessEngine) -> Result<i64, Error>;
    fn load_f32(&self, addr: usize, align: u32, engine: &StacklessEngine) -> Result<f32, Error>;
    fn load_f64(&self, addr: usize, align: u32, engine: &StacklessEngine) -> Result<f64, Error>;
    fn load_i8(&self, addr: usize, align: u32, engine: &StacklessEngine) -> Result<i8, Error>;
    fn load_u8(&self, addr: usize, align: u32, engine: &StacklessEngine) -> Result<u8, Error>;
    fn load_i16(&self, addr: usize, align: u32, engine: &StacklessEngine) -> Result<i16, Error>;
    fn load_u16(&self, addr: usize, align: u32, engine: &StacklessEngine) -> Result<u16, Error>;
    fn load_v128(
        &self,
        addr: usize,
        align: u32,
        engine: &StacklessEngine,
    ) -> Result<[u8; 16], Error>;
    fn store_i32(
        &mut self,
        addr: usize,
        align: u32,
        value: i32,
        engine: &StacklessEngine,
    ) -> Result<(), Error>;
    fn store_i64(
        &mut self,
        addr: usize,
        align: u32,
        value: i64,
        engine: &StacklessEngine,
    ) -> Result<(), Error>;
    fn store_f32(
        &mut self,
        addr: usize,
        align: u32,
        value: f32,
        engine: &StacklessEngine,
    ) -> Result<(), Error>;
    fn store_f64(
        &mut self,
        addr: usize,
        align: u32,
        value: f64,
        engine: &StacklessEngine,
    ) -> Result<(), Error>;
    fn store_i8(
        &mut self,
        addr: usize,
        align: u32,
        value: i8,
        engine: &StacklessEngine,
    ) -> Result<(), Error>;
    fn store_u8(
        &mut self,
        addr: usize,
        align: u32,
        value: u8,
        engine: &StacklessEngine,
    ) -> Result<(), Error>;
    fn store_i16(
        &mut self,
        addr: usize,
        align: u32,
        value: i16,
        engine: &StacklessEngine,
    ) -> Result<(), Error>;
    fn store_u16(
        &mut self,
        addr: usize,
        align: u32,
        value: u16,
        engine: &StacklessEngine,
    ) -> Result<(), Error>;
    fn store_v128(
        &mut self,
        addr: usize,
        align: u32,
        value: [u8; 16],
        engine: &StacklessEngine,
    ) -> Result<(), Error>;
    fn memory_size(&self, engine: &StacklessEngine) -> Result<u32, Error>;
    fn memory_grow(&mut self, pages: u32, engine: &StacklessEngine) -> Result<u32, Error>;

    // Table access methods (need engine context)
    fn table_get(
        &self,
        _table_idx: u32,
        _idx: u32,
        engine: &StacklessEngine,
    ) -> Result<Value, Error>;
    fn table_set(
        &mut self,
        table_idx: u32,
        idx: u32,
        value: Value,
        engine: &StacklessEngine,
    ) -> Result<(), Error>;
    fn table_size(&self, table_idx: u32, engine: &StacklessEngine) -> Result<u32, Error>;
    fn table_grow(
        &mut self,
        table_idx: u32,
        delta: u32,
        value: Value,
        engine: &StacklessEngine,
    ) -> Result<u32, Error>;
    fn table_init(
        &mut self,
        table_idx: u32,
        elem_idx: u32,
        dst: u32,
        src: u32,
        n: u32,
        engine: &StacklessEngine,
    ) -> Result<(), Error>;
    fn table_copy(
        &mut self,
        dst_table: u32,
        src_table: u32,
        dst: u32,
        src: u32,
        n: u32,
        engine: &StacklessEngine,
    ) -> Result<(), Error>;
    fn elem_drop(&mut self, elem_idx: u32, engine: &StacklessEngine) -> Result<(), Error>;
    fn table_fill(
        &mut self,
        table_idx: u32,
        dst: u32,
        val: Value,
        n: u32,
        engine: &StacklessEngine,
    ) -> Result<(), Error>;

    /// Get an element segment by index (needs instance)
    fn get_element_segment(
        &self,
        elem_idx: u32,
        engine: &StacklessEngine,
    ) -> Result<Arc<Element>, Error>;

    /// Get the data segment by index (needs instance)
    fn get_data_segment(&self, data_idx: u32, engine: &StacklessEngine)
        -> Result<Arc<Data>, Error>;

    /// Drop a data segment (needs instance)
    fn drop_data_segment(&mut self, data_idx: u32, engine: &StacklessEngine) -> Result<(), Error>;

    // Stack interaction helpers (might not belong here, could be separate trait?)
    // These might still need &mut self if they directly manipulate a mutable stack reference
    fn pop_bool(&mut self, stack: &mut dyn StackBehavior) -> Result<bool, Error>;
    fn pop_i32(&mut self, stack: &mut dyn StackBehavior) -> Result<i32, Error>;

    /// Get two tables and return a tuple of Arc<Table>
    fn get_two_tables_mut(
        &mut self,
        _idx1: u32,
        _idx2: u32,
        engine: &StacklessEngine,
    ) -> Result<(Arc<Table>, Arc<Table>), Error>;

    // Added for bulk memory/table operations
    fn set_data_segment(&mut self, _idx: u32, _segment: Arc<Data>) -> Result<(), Error>;
}

/// Defines behaviors related to control flow instructions.
pub trait ControlFlowBehavior {
    /// Called when entering a `block` instruction.
    fn enter_block(&mut self, ty: BlockType, stack_len: usize) -> Result<(), Error>;
    /// Called when entering a `loop` instruction.
    fn enter_loop(&mut self, ty: BlockType, stack_len: usize) -> Result<(), Error>;
    /// Called when entering an `if` instruction.
    fn enter_if(&mut self, ty: BlockType, stack_len: usize, condition: bool) -> Result<(), Error>;
    /// Called when entering an `else` branch.
    fn enter_else(&mut self, stack_len: usize) -> Result<(), Error>;
    /// Called when exiting a block (`end` instruction).
    fn exit_block(&mut self, stack: &mut dyn StackBehavior) -> Result<(), Error>;
    /// Called for `br` and `br_if` instructions.
    fn branch(&mut self, depth: u32) -> Result<(usize, usize), Error>;
    /// Called for the `return` instruction.
    fn return_(&mut self) -> Result<(usize, usize), Error>;
    /// Called for the `call` instruction.
    fn call(&mut self, func_idx: u32, stack: &mut dyn StackBehavior) -> Result<(), Error>;
    /// Called for the `call_indirect` instruction.
    fn call_indirect(
        &mut self,
        type_idx: u32,
        table_idx: u32,
        entry: u32,
        stack: &mut dyn StackBehavior,
    ) -> Result<(), Error>;
    /// Sets the arity for the current label (used for stack validation).
    fn set_label_arity(&mut self, arity: usize);
}

/// Trait for executing WebAssembly instructions
pub trait InstructionExecutor: std::fmt::Debug {
    /// Execute the instruction in the given context
    ///
    /// # Arguments
    /// * `stack` - The operand stack.
    /// * `frame` - The current execution frame.
    /// * `engine` - The stackless engine (provides access to instances, etc.).
    ///
    /// # Returns
    /// * `Ok(ControlFlow)` - Indicates the next control flow action for the engine.
    /// * `Err(Error)` - If an error occurred during execution.
    fn execute(
        &self,
        stack: &mut dyn StackBehavior,
        frame: &mut dyn FrameBehavior,
        engine: &mut StacklessEngine,
    ) -> Result<ControlFlow, Error>;

    /// Execute the instruction using a frame index to avoid multiple mutable borrows
    ///
    /// This is a default implementation that can be overridden for efficiency
    ///
    /// # Arguments
    /// * `stack` - The operand stack which contains the frames
    /// * `frame_idx` - The index of the frame to use
    /// * `engine` - The stackless engine
    ///
    /// # Returns
    /// * `Ok(ControlFlow)` - Indicates the next control flow action for the engine.
    /// * `Err(Error)` - If an error occurred during execution.
    fn execute_with_frame_idx(
        &self,
        _stack: &mut dyn StackBehavior,
        _frame_idx: usize,
        _engine: &mut StacklessEngine,
    ) -> Result<ControlFlow, Error> {
        // This implementation is no longer needed as stackless.rs directly uses execute
        // with a cloned frame, avoiding the borrow checker issues
        Err(Error::new(prelude::ErrorCategory::Execution, prelude::codes::EXECUTION_ERROR, "execute_with_frame_idx is deprecated, use execute directly".to_string()))
    }
}

/// Represents a control-flow label used by behavior traits.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Label {
    /// The number of values expected on the stack after the block corresponding to this label completes.
    pub arity: usize,
    /// The program counter (instruction index) pointing to the instruction *after* the block's end.
    pub pc: usize,
    /// The program counter (instruction index) pointing to the continuation of the block (e.g., the `else` part of an `if`, or the start for `loop`).
    pub continuation: usize,
    /// The depth of the value stack when this label was pushed (used for stack cleanup on branch).
    pub stack_depth: usize,
    /// Indicates if this label represents a loop (for `br` targeting).
    pub is_loop: bool,
    /// Indicates if this label represents an if block (for `else` handling).
    pub is_if: bool, // Optional, might help with else logic
}

// Add AsRef<[u8]> implementation for Label to work with BoundedVec
impl AsRef<[u8]> for Label {
    fn as_ref(&self) -> &[u8] {
        // Create a static representation of the label using its critical fields
        // This is a simplification for checksum purposes only
        static mut BUFFER: [u8; 32] = [0; 32];

        unsafe {
            // Pack the critical fields into the buffer
            let arity_bytes = self.arity.to_le_bytes();
            let pc_bytes = self.pc.to_le_bytes();
            let continuation_bytes = self.continuation.to_le_bytes();
            let stack_depth_bytes = self.stack_depth.to_le_bytes();

            // Copy bytes into buffer
            BUFFER[0..8].copy_from_slice(&arity_bytes);
            BUFFER[8..16].copy_from_slice(&pc_bytes);
            BUFFER[16..24].copy_from_slice(&continuation_bytes);
            BUFFER[24..32].copy_from_slice(&stack_depth_bytes);

            // Return a slice to the buffer
            &BUFFER[..]
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

// Static memory type for NullBehavior (no need for once_cell)
static NULL_MEMORY_TYPE: MemoryType = MemoryType {
    limits: Limits { min: 0, max: None },
    shared: false,
};

/// Placeholder implementation for behaviors when no real implementation is needed.
#[derive(Debug)]
pub struct NullBehavior {
    pub locals: Vec<Value>,
    pub pc: usize,
    pub func_idx: u32,
    pub arity: usize,
    pub label_arity: usize,
    pub return_pc: usize,
    pub label_stack: Vec<Label>,
    // Added instance_idx to satisfy FrameBehavior
    pub instance_idx: u32,
}

impl FrameBehavior for NullBehavior {
    fn locals(&mut self) -> &mut Vec<Value> {
        &mut self.locals
    }

    fn get_local(&self, idx: usize) -> Result<Value, Error> {
        Err(Error::new(prelude::ErrorCategory::InvalidLocalIndex, prelude::codes::INVALID_LOCAL_INDEX_ERROR, idx as u32))
    }

    fn as_any(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn set_local(&mut self, idx: usize, _value: Value) -> Result<(), Error> {
        Err(Error::new(prelude::ErrorCategory::InvalidLocalIndex, prelude::codes::INVALID_LOCAL_INDEX_ERROR, idx as u32))
    }

    fn get_global(&self, idx: usize, _engine: &StacklessEngine) -> Result<Arc<Global>, Error> {
        Err(Error::new(prelude::ErrorCategory::InvalidGlobalIndex, prelude::codes::INVALID_GLOBAL_INDEX_ERROR, idx as u32))
    }

    fn set_global(
        &mut self,
        idx: usize,
        _value: Value,
        _engine: &StacklessEngine,
    ) -> Result<(), Error> {
        Err(Error::new(prelude::ErrorCategory::InvalidGlobalIndex, prelude::codes::INVALID_GLOBAL_INDEX_ERROR, idx as u32))
    }

    fn get_memory(&self, idx: usize, _engine: &StacklessEngine) -> Result<Arc<Memory>, Error> {
        Err(Error::new(prelude::ErrorCategory::InvalidMemoryIndex, prelude::codes::INVALID_MEMORY_INDEX_ERROR, idx as u32))
    }

    fn get_memory_mut(
        &mut self,
        idx: usize,
        _engine: &StacklessEngine,
    ) -> Result<Arc<Memory>, Error> {
        Err(Error::new(prelude::ErrorCategory::InvalidMemoryIndex, prelude::codes::INVALID_MEMORY_INDEX_ERROR, idx as u32))
    }

    fn get_table(&self, idx: usize, _engine: &StacklessEngine) -> Result<Arc<Table>, Error> {
        Err(Error::new(prelude::ErrorCategory::NotImplemented, prelude::codes::NOT_IMPLEMENTED_ERROR, format!("NullBehavior::get_table for index: {}", idx)))
    }

    fn get_table_mut(
        &mut self,
        idx: usize,
        _engine: &StacklessEngine,
    ) -> Result<Arc<Table>, Error> {
        Err(Error::new(prelude::ErrorCategory::NotImplemented, prelude::codes::NOT_IMPLEMENTED_ERROR, format!("NullBehavior::get_table_mut for index: {}", idx)))
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
        self.instance_idx
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

    fn set_return_pc(&mut self, _pc: Option<usize>) {}

    fn load_i32(&self, _addr: usize, _align: u32, _engine: &StacklessEngine) -> Result<i32, Error> {
        Err(Error::new(prelude::ErrorCategory::NotImplemented, prelude::codes::NOT_IMPLEMENTED_ERROR, "NullBehavior::load_i32".to_string()))
    }

    fn load_i64(&self, _addr: usize, _align: u32, _engine: &StacklessEngine) -> Result<i64, Error> {
        Err(Error::new(prelude::ErrorCategory::NotImplemented, prelude::codes::NOT_IMPLEMENTED_ERROR, "NullBehavior::load_i64".to_string()))
    }

    fn load_f32(&self, _addr: usize, _align: u32, _engine: &StacklessEngine) -> Result<f32, Error> {
        Err(Error::new(prelude::ErrorCategory::NotImplemented, prelude::codes::NOT_IMPLEMENTED_ERROR, "NullBehavior::load_f32".to_string()))
    }

    fn load_f64(&self, _addr: usize, _align: u32, _engine: &StacklessEngine) -> Result<f64, Error> {
        Err(Error::new(prelude::ErrorCategory::NotImplemented, prelude::codes::NOT_IMPLEMENTED_ERROR, "NullBehavior::load_f64".to_string()))
    }

    fn load_i8(&self, _addr: usize, _align: u32, _engine: &StacklessEngine) -> Result<i8, Error> {
        Err(Error::new(prelude::ErrorCategory::NotImplemented, prelude::codes::NOT_IMPLEMENTED_ERROR, "NullBehavior::load_i8".to_string()))
    }

    fn load_u8(&self, _addr: usize, _align: u32, _engine: &StacklessEngine) -> Result<u8, Error> {
        Err(Error::new(prelude::ErrorCategory::NotImplemented, prelude::codes::NOT_IMPLEMENTED_ERROR, "NullBehavior::load_u8".to_string()))
    }

    fn load_i16(&self, _addr: usize, _align: u32, _engine: &StacklessEngine) -> Result<i16, Error> {
        Err(Error::new(prelude::ErrorCategory::NotImplemented, prelude::codes::NOT_IMPLEMENTED_ERROR, "NullBehavior::load_i16".to_string()))
    }

    fn load_u16(&self, _addr: usize, _align: u32, _engine: &StacklessEngine) -> Result<u16, Error> {
        Err(Error::new(prelude::ErrorCategory::NotImplemented, prelude::codes::NOT_IMPLEMENTED_ERROR, "NullBehavior::load_u16".to_string()))
    }

    fn load_v128(
        &self,
        _addr: usize,
        _align: u32,
        _engine: &StacklessEngine,
    ) -> Result<[u8; 16], Error> {
        Err(Error::new(prelude::ErrorCategory::NotImplemented, prelude::codes::NOT_IMPLEMENTED_ERROR, "NullBehavior::load_v128".to_string()))
    }

    fn store_i32(
        &mut self,
        _addr: usize,
        _align: u32,
        _value: i32,
        _engine: &StacklessEngine,
    ) -> Result<(), Error> {
        Err(Error::new(prelude::ErrorCategory::NotImplemented, prelude::codes::NOT_IMPLEMENTED_ERROR, "NullBehavior::store_i32".to_string()))
    }

    fn store_i64(
        &mut self,
        _addr: usize,
        _align: u32,
        _value: i64,
        _engine: &StacklessEngine,
    ) -> Result<(), Error> {
        Err(Error::new(prelude::ErrorCategory::NotImplemented, prelude::codes::NOT_IMPLEMENTED_ERROR, "NullBehavior::store_i64".to_string()))
    }

    fn store_f32(
        &mut self,
        _addr: usize,
        _align: u32,
        _value: f32,
        _engine: &StacklessEngine,
    ) -> Result<(), Error> {
        Err(Error::new(prelude::ErrorCategory::NotImplemented, prelude::codes::NOT_IMPLEMENTED_ERROR, "NullBehavior::store_f32".to_string()))
    }

    fn store_f64(
        &mut self,
        _addr: usize,
        _align: u32,
        _value: f64,
        _engine: &StacklessEngine,
    ) -> Result<(), Error> {
        Err(Error::new(prelude::ErrorCategory::NotImplemented, prelude::codes::NOT_IMPLEMENTED_ERROR, "NullBehavior::store_f64".to_string()))
    }

    fn store_i8(
        &mut self,
        _addr: usize,
        _align: u32,
        _value: i8,
        _engine: &StacklessEngine,
    ) -> Result<(), Error> {
        Err(Error::new(prelude::ErrorCategory::NotImplemented, prelude::codes::NOT_IMPLEMENTED_ERROR, "NullBehavior::store_i8".to_string()))
    }

    fn store_u8(
        &mut self,
        _addr: usize,
        _align: u32,
        _value: u8,
        _engine: &StacklessEngine,
    ) -> Result<(), Error> {
        Err(Error::new(prelude::ErrorCategory::NotImplemented, prelude::codes::NOT_IMPLEMENTED_ERROR, "NullBehavior::store_u8".to_string()))
    }

    fn store_i16(
        &mut self,
        _addr: usize,
        _align: u32,
        _value: i16,
        _engine: &StacklessEngine,
    ) -> Result<(), Error> {
        Err(Error::new(prelude::ErrorCategory::NotImplemented, prelude::codes::NOT_IMPLEMENTED_ERROR, "NullBehavior::store_i16".to_string()))
    }

    fn store_u16(
        &mut self,
        _addr: usize,
        _align: u32,
        _value: u16,
        _engine: &StacklessEngine,
    ) -> Result<(), Error> {
        Err(Error::new(prelude::ErrorCategory::NotImplemented, prelude::codes::NOT_IMPLEMENTED_ERROR, "NullBehavior::store_u16".to_string()))
    }

    fn store_v128(
        &mut self,
        _addr: usize,
        _align: u32,
        _value: [u8; 16],
        _engine: &StacklessEngine,
    ) -> Result<(), Error> {
        Err(Error::new(prelude::ErrorCategory::NotImplemented, prelude::codes::NOT_IMPLEMENTED_ERROR, "NullBehavior::store_v128".to_string()))
    }

    fn memory_size(&self, _engine: &StacklessEngine) -> Result<u32, Error> {
        Err(Error::new(prelude::ErrorCategory::NotImplemented, prelude::codes::NOT_IMPLEMENTED_ERROR, "NullBehavior::memory_size".to_string()))
    }

    fn memory_grow(&mut self, _pages: u32, _engine: &StacklessEngine) -> Result<u32, Error> {
        Err(Error::new(prelude::ErrorCategory::NotImplemented, prelude::codes::NOT_IMPLEMENTED_ERROR, "NullBehavior::memory_grow".to_string()))
    }

    fn table_get(
        &self,
        _table_idx: u32,
        _idx: u32,
        _engine: &StacklessEngine,
    ) -> Result<Value, Error> {
        Err(Error::new(prelude::ErrorCategory::NotImplemented, prelude::codes::NOT_IMPLEMENTED_ERROR, "NullBehavior::table_get".to_string()))
    }

    fn table_set(
        &mut self,
        _table_idx: u32,
        _idx: u32,
        _value: Value,
        _engine: &StacklessEngine,
    ) -> Result<(), Error> {
        Err(Error::new(prelude::ErrorCategory::NotImplemented, prelude::codes::NOT_IMPLEMENTED_ERROR, "NullBehavior::table_set".to_string()))
    }

    fn table_size(&self, _table_idx: u32, _engine: &StacklessEngine) -> Result<u32, Error> {
        Err(Error::new(prelude::ErrorCategory::NotImplemented, prelude::codes::NOT_IMPLEMENTED_ERROR, "NullBehavior::table_size".to_string()))
    }

    fn table_grow(
        &mut self,
        _table_idx: u32,
        _delta: u32,
        _value: Value,
        _engine: &StacklessEngine,
    ) -> Result<u32, Error> {
        Err(Error::new(prelude::ErrorCategory::NotImplemented, prelude::codes::NOT_IMPLEMENTED_ERROR, "NullBehavior::table_grow".to_string()))
    }

    // Match trait: engine should be &StacklessEngine
    fn table_init(
        &mut self,
        _dst_table_idx: u32,
        _src_elem_idx: u32,
        _dst_idx: u32,
        _src_idx: u32,
        _len: u32,
        _engine: &StacklessEngine,
    ) -> Result<(), Error> {
        Err(Error::new(prelude::ErrorCategory::NotImplemented, prelude::codes::NOT_IMPLEMENTED_ERROR, "NullBehavior::table_init".to_string()))
    }

    // Match trait: engine should be &StacklessEngine
    fn table_copy(
        &mut self,
        _dst_table_idx: u32,
        _src_table_idx: u32,
        _dst_idx: u32,
        _src_idx: u32,
        _len: u32,
        _engine: &StacklessEngine,
    ) -> Result<(), Error> {
        Err(Error::new(prelude::ErrorCategory::NotImplemented, prelude::codes::NOT_IMPLEMENTED_ERROR, "NullBehavior::table_copy".to_string()))
    }

    // Match trait: engine should be &StacklessEngine
    fn elem_drop(&mut self, _elem_idx: u32, _engine: &StacklessEngine) -> Result<(), Error> {
        Err(Error::new(prelude::ErrorCategory::NotImplemented, prelude::codes::NOT_IMPLEMENTED_ERROR, "NullBehavior::elem_drop".to_string()))
    }

    fn table_fill(
        &mut self,
        _table_idx: u32,
        _dst: u32,
        _val: Value,
        _n: u32,
        _engine: &StacklessEngine,
    ) -> Result<(), Error> {
        Err(Error::new(prelude::ErrorCategory::NotImplemented, prelude::codes::NOT_IMPLEMENTED_ERROR, "NullBehavior::table_fill".to_string()))
    }

    fn get_function_type(&self, func_idx: u32) -> Result<FuncType, Error> {
        Err(Error::new(prelude::ErrorCategory::InvalidFunctionIndex, prelude::codes::INVALID_FUNCTION_INDEX_ERROR, func_idx as usize))
    }

    fn get_two_tables_mut(
        &mut self,
        _idx1: u32,
        _idx2: u32,
        _engine: &StacklessEngine,
    ) -> Result<(Arc<Table>, Arc<Table>), Error> {
        Err(Error::new(prelude::ErrorCategory::NotImplemented, prelude::codes::NOT_IMPLEMENTED_ERROR, "NullBehavior::get_two_tables_mut".to_string()))
    }

    fn get_element_segment(
        &self,
        elem_idx: u32,
        _engine: &StacklessEngine,
    ) -> Result<Arc<Element>, Error> {
        Err(Error::new(prelude::ErrorCategory::NotImplemented, prelude::codes::NOT_IMPLEMENTED_ERROR, format!("NullBehavior::get_element_segment for index: {}", elem_idx)))
    }

    fn get_data_segment(
        &self,
        data_idx: u32,
        _engine: &StacklessEngine,
    ) -> Result<Arc<Data>, Error> {
        Err(Error::new(prelude::ErrorCategory::NotImplemented, prelude::codes::NOT_IMPLEMENTED_ERROR, format!("NullBehavior::get_data_segment for index: {}", data_idx)))
    }

    fn drop_data_segment(&mut self, data_idx: u32, _engine: &StacklessEngine) -> Result<(), Error> {
        Err(Error::new(prelude::ErrorCategory::NotImplemented, prelude::codes::NOT_IMPLEMENTED_ERROR, format!("NullBehavior::drop_data_segment for index: {}", data_idx)))
    }

    fn set_data_segment(&mut self, _idx: u32, _segment: Arc<Data>) -> Result<(), Error> {
        Err(Error::new(prelude::ErrorCategory::NotImplemented, prelude::codes::NOT_IMPLEMENTED_ERROR, format!("NullBehavior::set_data_segment for index: {}", _idx)))
    }

    fn pop_bool(&mut self, stack: &mut dyn StackBehavior) -> Result<bool, Error> {
        stack.pop_bool()
    }

    fn pop_i32(&mut self, stack: &mut dyn StackBehavior) -> Result<i32, Error> {
        stack.pop_i32()
    }

    fn push_label(&mut self, label: Label) -> Result<(), Error> {
        Ok(())
    }

    fn pop_label(&mut self) -> Result<Label, Error> {
        Err(Error::new(prelude::ErrorCategory::Execution, prelude::codes::EXECUTION_ERROR, "Cannot pop_label from NullBehavior frame".to_string()))
    }

    fn get_label(&self, depth: usize) -> Option<&Label> {
        None
    }

    fn locals_mut(&mut self) -> &mut [Value] {
        &mut []
    }

    fn get_global_mut(&mut self, _index: usize) -> Result<MutexGuard<Value>, Error> {
        Err(Error::new(prelude::ErrorCategory::Execution, prelude::codes::EXECUTION_ERROR, "Cannot get_global_mut from NullBehavior frame".to_string()))
    }
}

impl ControlFlowBehavior for NullBehavior {
    fn enter_block(&mut self, _ty: BlockType, _stack_len: usize) -> Result<(), Error> {
        Ok(())
    }
    fn enter_loop(&mut self, _ty: BlockType, _stack_len: usize) -> Result<(), Error> {
        Ok(())
    }
    fn enter_if(
        &mut self,
        _ty: BlockType,
        _stack_len: usize,
        _condition: bool,
    ) -> Result<(), Error> {
        Ok(())
    }
    fn enter_else(&mut self, _stack_len: usize) -> Result<(), Error> {
        Ok(())
    }
    fn exit_block(&mut self, _stack: &mut dyn StackBehavior) -> Result<(), Error> {
        Ok(())
    }
    fn branch(&mut self, _depth: u32) -> Result<(usize, usize), Error> {
        Ok((0, 0))
    }
    fn return_(&mut self) -> Result<(usize, usize), Error> {
        Ok((0, 0))
    }
    fn call(&mut self, _func_idx: u32, _stack: &mut dyn StackBehavior) -> Result<(), Error> {
        Ok(())
    }
    fn call_indirect(
        &mut self,
        _type_idx: u32,
        _table_idx: u32,
        _entry: u32,
        _stack: &mut dyn StackBehavior,
    ) -> Result<(), Error> {
        Ok(())
    }
    fn set_label_arity(&mut self, _arity: usize) {}
}

impl StackBehavior for NullBehavior {
    fn push(&mut self, _value: Value) -> Result<(), Error> {
        Ok(())
    }

    fn pop(&mut self) -> Result<Value, Error> {
        Err(Error::new(prelude::ErrorCategory::StackUnderflow, prelude::codes::STACK_UNDERFLOW_ERROR, ""))
    }

    fn peek(&self) -> Result<&Value, Error> {
        Err(Error::new(prelude::ErrorCategory::StackUnderflow, prelude::codes::STACK_UNDERFLOW_ERROR, ""))
    }

    fn peek_mut(&mut self) -> Result<&mut Value, Error> {
        Err(Error::new(prelude::ErrorCategory::StackUnderflow, prelude::codes::STACK_UNDERFLOW_ERROR, ""))
    }

    fn values(&self) -> &[Value] {
        &[]
    }

    fn values_mut(&mut self) -> &mut [Value] {
        &mut []
    }

    fn len(&self) -> usize {
        0
    }

    fn is_empty(&self) -> bool {
        true
    }

    fn push_label(&mut self, _label: Label) -> Result<(), Error> {
        Ok(())
    }

    fn pop_label(&mut self) -> Result<Label, Error> {
        Err(Error::new(prelude::ErrorCategory::Execution, prelude::codes::EXECUTION_ERROR, "Cannot pop_label from NullBehavior stack".to_string()))
    }

    fn get_label(&self, _depth: usize) -> Option<&Label> {
        None
    }

    fn push_n(&mut self, _values: &[Value]) {}

    fn pop_n(&mut self, _n: usize) -> Vec<Value> {
        Vec::new()
    }

    fn pop_frame_label(&mut self) -> Result<Label, Error> {
        Err(Error::new(prelude::ErrorCategory::Execution, prelude::codes::EXECUTION_ERROR, "Cannot pop_frame_label from NullBehavior stack".to_string()))
    }

    fn execute_function_call_direct(
        &mut self,
        _engine: &mut StacklessEngine,
        _caller_instance_idx: u32,
        _func_idx: u32,
        _args: Vec<Value>,
    ) -> Result<Vec<Value>, Error> {
        Err(Error::new(prelude::ErrorCategory::Execution, prelude::codes::EXECUTION_ERROR, "Cannot execute_function_call_direct on NullBehavior".to_string()))
    }

    fn as_any(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

impl InstructionExecutor for NullBehavior {
    fn execute(
        &self,
        _stack: &mut dyn StackBehavior,
        _frame: &mut dyn FrameBehavior,
        _engine: &mut StacklessEngine,
    ) -> Result<ControlFlow, Error> {
        Err(Error::new(prelude::ErrorCategory::NotImplemented, prelude::codes::NOT_IMPLEMENTED_ERROR, "NullBehavior::execute".to_string()))
    }

    fn execute_with_frame_idx(
        &self,
        _stack: &mut dyn StackBehavior,
        _frame_idx: usize,
        _engine: &mut StacklessEngine,
    ) -> Result<ControlFlow, Error> {
        // This implementation is no longer needed as stackless.rs directly uses execute
        // with a cloned frame, avoiding the borrow checker issues
        Err(Error::new(prelude::ErrorCategory::Execution, prelude::codes::EXECUTION_ERROR, "execute_with_frame_idx is deprecated, use execute directly".to_string()))
    }
}

// Explicitly implemented instance_idx at the end of the implementation for NullBehavior
impl NullBehavior {
    // Explicitly implemented to fix compiler error
    pub fn instance_idx(&self) -> u32 {
        self.instance_idx
    }
}

/// Define the EngineBehavior trait for the engine implementation that's used in memory.rs
pub trait EngineBehavior {
    /// Get a memory instance by index
    fn get_memory(&self, memory_idx: usize, instance_idx: usize) -> Result<Arc<Memory>, Error>;

    /// Get a global variable by index
    fn get_global(&self, global_idx: usize, instance_idx: usize) -> Result<Arc<Global>, Error>;

    /// Get a table by index
    fn get_table(&self, table_idx: usize, instance_idx: usize) -> Result<Arc<Table>, Error>;

    /// Get a data segment by index
    fn get_data_segment(&self, data_idx: usize, instance_idx: usize) -> Result<Arc<Data>, Error>;

    /// Get an element segment by index
    fn get_element_segment(
        &self,
        elem_idx: usize,
        instance_idx: usize,
    ) -> Result<Arc<Element>, Error>;
}

// Implementation of EngineBehavior for StacklessEngine will be in stackless.rs

/// Additional behavior trait for module operations
pub trait ModuleBehavior {
    /// Execute instruction by index on behalf of frame
    fn execute_with_idx(
        &mut self,
        stack: &mut dyn StackBehavior,
        frame_idx: usize,
        engine: &mut StacklessEngine,
        instruction: &InstructionType,
    ) -> Result<ControlFlow, Error>;

    /// Get a table element by index
    fn table_elem_get(&self, table_idx: u32, idx: u32) -> Result<Value, Error>;

    /// Push a label onto the context's label stack
    fn push_label(&mut self, label: Label) -> Result<(), Error>;

    /// Get the label at the specified depth
    fn get_label(&self, depth: usize) -> Option<&Label>;

    /// Default implementation for executing instructions (override for custom behavior)
    fn on_instruction(
        &mut self,
        stack: &mut dyn StackBehavior,
        frame_idx: usize,
        engine: &mut StacklessEngine,
        instruction: &InstructionType,
    ) -> Result<ControlFlow, Error>;
}

// Default implementation for the NullBehavior
impl ModuleBehavior for NullBehavior {
    fn execute_with_idx(
        &mut self,
        _stack: &mut dyn StackBehavior,
        _frame_idx: usize,
        _engine: &mut StacklessEngine,
        instruction: &InstructionType,
    ) -> Result<ControlFlow, Error> {
        // Default implementation delegates to the instruction's execute method
        instruction.execute(self, self.func_idx(), _engine)
    }

    fn table_elem_get(&self, _table_idx: u32, _idx: u32) -> Result<Value, Error> {
        Err(Error::new(prelude::ErrorCategory::NotImplemented, prelude::codes::NOT_IMPLEMENTED_ERROR, "table_elem_get not implemented".to_string()))
    }

    fn push_label(&mut self, _label: Label) -> Result<(), Error> {
        Err(Error::new(prelude::ErrorCategory::NotImplemented, prelude::codes::NOT_IMPLEMENTED_ERROR, "push_label not implemented".to_string()))
    }

    fn get_label(&self, _depth: usize) -> Option<&Label> {
        None
    }

    fn on_instruction(
        &mut self,
        _stack: &mut dyn StackBehavior,
        _frame_idx: usize,
        _engine: &mut StacklessEngine,
        instruction: &InstructionType,
    ) -> Result<ControlFlow, Error> {
        instruction.execute(self, _frame_idx, _engine)
    }
}
