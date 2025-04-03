//! Stackless function activation frame
//!
//! This module defines the `StacklessFrame` struct and its associated implementations,
//! representing the state of a single function activation in the stackless WRT engine.

use core::panic;
use std::{any::Any, sync::Arc};

use crate::{
    instructions::Instruction,
    module::{Function, Module},
};
use parking_lot::MutexGuard;
use wasmparser::{BlockType as WasmBlockType, FuncType as WasmFuncType, ValType};

use crate::{
    crate::module_instance::ModuleInstance,
    behavior::{self, ControlFlowBehavior, FrameBehavior, Label, StackBehavior},
    error::{Error, Result},
    global::{self, Global},
    memory::{DefaultMemory, MemoryBehavior},
    stack::{self, Stack},
    table::Table,
    types::{BlockType, FuncType, ValueType},
    values::Value,
};

/// Represents a function activation frame in the stackless engine.
#[derive(Debug, Clone)]
pub struct StacklessFrame {
    /// The module associated with this frame.
    pub module: Arc<Module>,
    /// The index of the function being executed in this frame.
    pub func_idx: u32,
    /// The program counter, indicating the next instruction to execute within the function's code.
    pub pc: usize,
    /// The local variables for this frame, including function arguments.
    /// Note: In some contexts within the stackless engine, this might also temporarily hold operand stack values.
    pub locals: Vec<Value>,
    /// The index of the module instance this frame belongs to.
    pub instance_idx: u32,
    /// The number of return values expected by the caller of this function frame.
    pub arity: usize,
    /// The arity (number of stack values expected) of the current control flow block (block, loop, if).
    pub label_arity: usize,
    /// The stack of active control flow labels (blocks, loops, ifs) within this frame.
    pub label_stack: Vec<Label>,
    /// The program counter in the *caller's* frame to return to after this frame finishes.
    pub return_pc: usize,
}

impl StacklessFrame {
    /// Creates a new stackless frame for a function call (internal helper).
    /// Validates argument count and types against the function signature.
    fn new_internal(
        module: Arc<Module>,
        instance_idx: u32,
        func_idx: u32,
        args: Vec<Value>,
    ) -> Result<Self> {
        let module_clone = module.clone(); // Clone Arc before borrowing
        let func_type = module_clone.get_function_type(func_idx)?;

        if args.len() != func_type.params.len() {
            return Err(Error::InvalidFunctionType(format!(
                "Function {func_idx}: Expected {} arguments, got {}",
                func_type.params.len(),
                args.len()
            )));
        }

        for (i, (arg, param_type)) in args.iter().zip(func_type.params.iter()).enumerate() {
            if !arg.matches_type(param_type) {
                return Err(Error::InvalidType(format!(
                    "Function {func_idx}: Argument {} type mismatch: expected {:?}, got {:?}",
                    i,
                    param_type,
                    arg.get_type()
                )));
            }
        }

        Ok(Self {
            module: module_clone,
            func_idx,
            pc: 0,        // Start at the beginning of the function code
            locals: args, // Arguments become the initial part of locals
            instance_idx,
            arity: func_type.results.len(), // Frame arity is the function's RETURN arity
            label_arity: func_type.params.len(), // Initial label arity matches function INPUT arity
            label_stack: Vec::new(),
            return_pc: 0, // Will be set by the caller
        })
    }

    /// Creates a new stackless frame prepared for executing a specific function.
    /// Initializes locals with arguments and default values for declared local variables.
    pub fn new(
        module: Arc<Module>,
        func_idx: u32,
        args: &[Value],
        instance_idx: u32,
    ) -> Result<Self> {
        let func = module
            .get_function(func_idx)
            .ok_or(Error::FunctionNotFound(func_idx))?;

        // Use internal helper to validate args and create basic frame
        let mut frame = Self::new_internal(module.clone(), instance_idx, func_idx, args.to_vec())?;

        // Initialize declared local variables with their default values
        for local_type in &func.locals {
            frame.locals.push(Value::default_for_type(local_type));
        }

        Ok(frame)
    }

    /// Gets the function definition associated with this frame.
    pub fn get_function(&self) -> Result<&Function> {
        self.module
            .get_function(self.func_idx)
            .ok_or_else(|| Error::FunctionNotFound(self.func_idx))
    }

    /// Gets the function type associated with this frame.
    pub fn get_function_type(&self) -> Result<&FuncType> {
        self.module.get_function_type(self.func_idx).ok_or_else(|| {
            Error::InvalidFunctionType(format!(
                "Function type not found for index: {}",
                self.func_idx
            ))
        })
    }

    /// Finds the program counter (PC) of the matching `Else` or `End` instruction
    /// for the `If` block starting *after* the current frame PC.
    /// Used when the `If` condition is false.
    pub fn find_matching_else_or_end(&self) -> Result<usize> {
        let func = self.get_function()?;
        let code = &func.code;
        let mut depth = 1; // Start inside the If block
        let mut pc = self.pc + 1; // Start searching after the If instruction

        while pc < code.len() {
            match &code[pc] {
                Instruction::If(..) => depth += 1,
                Instruction::Else if depth == 1 => {
                    // Found the matching Else for the initial If
                    return Ok(pc);
                }
                Instruction::End => {
                    depth -= 1;
                    if depth == 0 {
                        // Found the matching End for the initial If (no Else)
                        return Ok(pc);
                    }
                }
                _ => {}
            }
            pc += 1;
        }

        Err(Error::Execution(format!(
            "Unmatched If at PC {} in function {}",
            self.pc, self.func_idx
        )))
    }

    /// Finds the program counter (PC) of the matching `End` instruction
    /// for the block (`Block`, `Loop`, `If`) starting *after* the current frame PC.
    /// Used primarily for skipping the `Else` block.
    pub fn find_matching_end(&self) -> Result<usize> {
        let func = self.get_function()?;
        let code = &func.code;
        let mut depth = 1; // Start inside the block needing an End
        let mut pc = self.pc + 1; // Start searching after the instruction starting the block (e.g., Else)

        while pc < code.len() {
            match &code[pc] {
                Instruction::Block(..) | Instruction::Loop(..) | Instruction::If(..) => depth += 1,
                Instruction::End => {
                    depth -= 1;
                    if depth == 0 {
                        // Found the matching End
                        return Ok(pc);
                    }
                }
                _ => {}
            }
            pc += 1;
        }
        Err(Error::Execution(format!(
            "Unmatched block starting near PC {} in function {}",
            self.pc, self.func_idx
        )))
    }
}

// Implement the behavior traits

impl StackBehavior for StacklessFrame {
    // NOTE: StackBehavior for StacklessFrame often manipulates `locals` directly
    // when used within the engine's step function, as there isn't a separate operand stack.
    // Be cautious when interpreting these methods outside that context.

    fn push(&mut self, value: Value) -> Result<()> {
        self.locals.push(value);
        Ok(())
    }

    fn pop(&mut self) -> Result<Value> {
        self.locals.pop().ok_or(Error::StackUnderflow)
    }

    fn peek(&self) -> Result<&Value> {
        self.locals.last().ok_or(Error::StackUnderflow)
    }

    fn peek_mut(&mut self) -> Result<&mut Value> {
        self.locals.last_mut().ok_or(Error::StackUnderflow)
    }

    fn values(&self) -> &[Value] {
        &self.locals
    }

    fn values_mut(&mut self) -> &mut [Value] {
        &mut self.locals
    }

    fn len(&self) -> usize {
        self.locals.len()
    }

    fn is_empty(&self) -> bool {
        self.locals.is_empty()
    }

    // Label stack operations are delegated to the frame's label_stack
    fn push_label(&mut self, arity: usize, pc: usize) {
        // Note: Continuation is often updated later based on block type
        self.label_stack.push(Label {
            arity,
            pc,
            continuation: pc, // Default continuation
        });
    }

    fn pop_label(&mut self) -> Result<Label> {
        self.label_stack
            .pop()
            .ok_or(Error::Execution("Label stack empty".to_string()))
    }

    fn get_label(&self, index: usize) -> Option<&Label> {
        // Access label stack relative to the end (top)
        let stack_len = self.label_stack.len();
        if index < stack_len {
            Some(&self.label_stack[stack_len - 1 - index])
        } else {
            None
        }
    }
}

impl ControlFlowBehavior for StacklessFrame {
    fn enter_block(&mut self, ty: BlockType, stack_len: usize) -> Result<()> {
        // Need to resolve function type index if BlockType::FuncType(idx)
        // let arity = self.module.resolve_block_type_arity(&ty)?; // Method not found on Arc<Module>
        let arity = 0; // Placeholder
        self.label_stack.push(Label {
            arity,
            pc: self.pc,                    // PC points *after* the block instruction
            continuation: self.pc,          // Default continuation is next instruction
            stack_depth: stack_len - arity, // Record stack depth before block params
            is_loop: false,
            is_if: false,
        });
        self.set_label_arity(arity);
        println!(
            "DEBUG: enter_block - Pushed Label: {:?}, Arity: {}, New Label Arity: {}",
            self.label_stack.last().unwrap(),
            arity,
            self.label_arity()
        );
        Ok(())
    }

    fn enter_loop(&mut self, ty: BlockType, stack_len: usize) -> Result<()> {
        // let arity = self.module.resolve_block_type_arity(&ty)?; // Method not found on Arc<Module>
        let arity = 0; // Placeholder
        self.label_stack.push(Label {
            arity,
            pc: self.pc,               // PC points *after* the loop instruction
            continuation: self.pc - 1, // Loop continues at the loop instruction itself
            stack_depth: stack_len - arity,
            is_loop: true,
            is_if: false,
        });
        self.set_label_arity(arity);
        println!(
            "DEBUG: enter_loop - Pushed Label: {:?}, Arity: {}, New Label Arity: {}",
            self.label_stack.last().unwrap(),
            arity,
            self.label_arity()
        );
        Ok(())
    }

    fn enter_if(&mut self, ty: BlockType, stack_len: usize, condition: bool) -> Result<()> {
        // let arity = self.module.resolve_block_type_arity(&ty)?; // Method not found on Arc<Module>
        let arity = 0; // Placeholder
        self.label_stack.push(Label {
            arity,
            pc: self.pc,
            continuation: self.pc, // Updated when 'End' or 'Else' is found
            stack_depth: stack_len - arity,
            is_loop: false,
            is_if: true,
        });
        // If condition is false, we need to jump *past* the `if` block
        // But we don't know where that is yet. This needs engine support or block parsing.
        // For now, assume we execute the 'if' block and handle jump at 'End' or 'Else'.
        if !condition {
            // Mark the block to be skipped until Else/End?
            // Or find the jump target now?
            // Requires more sophisticated control flow handling
            println!(
                "WARN: `if false` condition encountered, requires jump handling (unimplemented)"
            );
        }
        Ok(())
    }

    fn enter_else(&mut self, _stack_len: usize) -> Result<()> {
        // This instruction is encountered when executing the 'then' block of an 'if'.
        // We need to jump directly to the 'end' instruction matching the 'if'.
        // The label for the 'if' block is currently on top of the label stack.

        // Find the 'end' associated with the 'if' block *containing* this 'else'.
        // The `find_matching_end` helper assumes the block starts *after* the current PC.
        // We need a way to find the end corresponding to the label on the stack.

        // For now, let's search forward from the 'else' PC.
        let target_pc = self.find_matching_end()?;
        println!(
            "DEBUG: enter_else - Jumping from PC {} to {}",
            self.pc, target_pc
        );
        self.set_pc(target_pc);
        // The 'End' instruction will handle popping the label and values.
        Ok(())
    }

    fn exit_block(&mut self, stack: &mut dyn Stack) -> Result<()> {
        println!(
            "DEBUG: exit_block - START - PC: {}, Stack: {:?}, Label Stack: {:?}",
            self.pc,
            stack.values(), // Assuming stack provides a way to view values
            self.label_stack
        );

        // 1. Pop the current label
        let label = self.label_stack.pop().ok_or(Error::Execution(
            "Label stack empty in exit_block (End instruction)".into(),
        ))?;
        println!("DEBUG: exit_block - Popped Label: {:?}", label);

        // 2. Get the expected number of results for this block
        let arity = label.arity;
        println!("DEBUG: exit_block - Label arity: {}", arity);

        // 3. Assert stack has at least n values
        if stack.len() < arity {
            println!(
                "DEBUG: exit_block - ERROR: Stack underflow. Need {}, have {}",
                arity,
                stack.len()
            );
            return Err(Error::StackUnderflow);
        }

        // 4. Pop n values
        let mut results = Vec::with_capacity(arity);
        for _ in 0..arity {
            // Popping from the *operand* stack provided
            results.push(stack.pop()?);
        }
        // Popped values are in reverse order [val_n-1, ..., val_0]
        println!("DEBUG: exit_block - Popped results: {:?}", results);

        // 5. Push results back onto stack (in correct order [val_0, ..., val_n-1])
        results.reverse(); // Now results are [val_0, ..., val_n-1]
        println!("DEBUG: exit_block - Pushing results back: {:?}", results);
        for result in results {
            // Pushing back onto the *operand* stack provided
            stack.push(result)?;
        }

        // 6. Restore the outer block's arity to the frame's label_arity
        if let Some(outer_label) = self.label_stack.last() {
            println!("DEBUG: exit_block - Found outer label: {:?}", outer_label);
            let outer_arity = outer_label.arity;
            println!(
                "DEBUG: exit_block - Restoring label_arity to: {}",
                outer_arity
            );
            self.set_label_arity(outer_arity);
        } else {
            // If no outer label, we are exiting the function frame itself.
            // Restore arity to the function's return arity.
            let func_type = self.get_function_type()?;
            let func_arity = func_type.params.len();
            println!(
                "DEBUG: exit_block - Exiting function frame. Restoring label_arity to func return arity: {}",
                func_arity
            );
            self.set_label_arity(func_arity);
        }

        // 7. PC is handled by the main loop (advances past the 'End')
        println!(
            "DEBUG: exit_block - END - PC: {}, Stack: {:?}, Label Stack: {:?}, New Label Arity: {}",
            self.pc,
            stack.values(), // Assuming stack provides a way to view values
            self.label_stack,
            self.label_arity()
        );
        Ok(())
    }

    fn branch(&mut self, depth: u32, stack: &mut dyn Stack) -> Result<()> {
        println!(
            "DEBUG: branch - START - Depth: {}, PC: {}, Stack: {:?}, Label Stack: {:?}",
            depth,
            self.pc,
            stack.values(), // Assuming stack provides a way to view values
            self.label_stack
        );

        let label_stack_len = self.label_stack.len();
        if depth as usize >= label_stack_len {
            return Err(Error::Execution(format!(
                "Branch depth {} out of bounds (label stack len = {})",
                depth, label_stack_len
            )));
        }

        // 1. Get the target label (relative to the top of the stack)
        let target_label_index = label_stack_len - 1 - (depth as usize);
        // Clone needed because we modify label_stack later
        let target_label = self.label_stack[target_label_index].clone();
        let target_arity = target_label.arity;
        println!(
            "DEBUG: branch - Target Label (idx {}): {:?}, Arity: {}",
            target_label_index, target_label, target_arity
        );

        // 2. Pop the m result values expected by the target label's block.
        if stack.len() < target_arity {
            println!(
                "DEBUG: branch - ERROR: Stack underflow for branch results. Need {}, have {}",
                target_arity,
                stack.len()
            );
            return Err(Error::StackUnderflow);
        }
        let mut results = Vec::with_capacity(target_arity);
        for _ in 0..target_arity {
            results.push(stack.pop()?);
        }
        results.reverse(); // Keep results in stack order [res_0, ..., res_m-1]
        println!("DEBUG: branch - Popped results for target: {:?}", results);

        // 3. Pop labels from the label stack up to and including the target label.
        for d in 0..=depth {
            let popped_label = self.label_stack.pop().unwrap(); // Safe due to depth check
            println!(
                "DEBUG: branch - Popped label (depth {}): {:?}",
                depth - d,
                popped_label
            );
        }

        // 4. Push the result values back onto the stack.
        println!("DEBUG: branch - Pushing results back: {:?}", results);
        for result in results {
            stack.push(result)?;
        }

        // 5. Set the program counter to the target label's continuation point.
        println!(
            "DEBUG: branch - Setting PC to label continuation: {}",
            target_label.continuation
        );
        self.set_pc(target_label.continuation);

        // 6. Restore the arity of the new top label (if any)
        let new_label_arity = self.label_stack.last().map(|l| l.arity);
        if let Some(arity) = new_label_arity {
            self.set_label_arity(arity);
            // println!("DEBUG: branch - Restored label_arity to: {}", arity); // Removed to potentially fix borrow issue
        } else {
            // If branching out of the function entirely, restore function return arity
            let func_type = self.get_function_type()?;
            let func_arity = func_type.params.len();
            self.set_label_arity(func_arity);
            println!(
                "DEBUG: branch - Restored label_arity to func arity: {}",
                func_arity
            );
        }

        // 7. Pop labels up to the target
        self.label_stack.truncate(target_label_index + 1);

        println!(
            "DEBUG: branch - END - PC: {}, Stack: {:?}, Label Stack: {:?}, New Label Arity: {}",
            self.pc,
            stack.values(), // Assuming stack provides a way to view values
            self.label_stack,
            self.label_arity()
        );
        Ok(())
    }

    fn return_(&mut self, stack: &mut dyn Stack) -> Result<()> {
        // 1. Get the function's return arity
        let func_type = self.get_function_type()?;
        let return_arity = func_type.params.len();
        println!("DEBUG: return_ - Func Arity: {}", return_arity);

        // 2. Pop the return values from the stack
        if stack.len() < return_arity {
            println!(
                "DEBUG: return_ - ERROR: Stack underflow for return values. Need {}, have {}",
                return_arity,
                stack.len()
            );
            return Err(Error::StackUnderflow);
        }
        let mut return_values = Vec::with_capacity(return_arity);
        for _ in 0..return_arity {
            return_values.push(stack.pop()?);
        }
        return_values.reverse(); // Put them in order [val_0, ..., val_n-1]
        println!("DEBUG: return_ - Popped return values: {:?}", return_values);

        // 3. Clear the label stack for this frame (as we are leaving it)
        self.label_stack.clear();
        println!("DEBUG: return_ - Cleared label stack");

        // 4. Push return values back (caller will expect them)
        println!(
            "DEBUG: return_ - Pushing return values back: {:?}",
            return_values
        );
        for value in return_values {
            stack.push(value)?;
        }

        // 5. Set PC to the stored return address
        println!(
            "DEBUG: return_ - Setting PC to return_pc: {}",
            self.return_pc
        );
        self.set_pc(self.return_pc);

        // The actual frame pop happens in the engine loop
        Ok(())
    }

    // `call` and `call_indirect` are handled by the engine, not directly by frame behavior.
    // The engine pushes a new frame.
    fn call(&mut self, _func_idx: u32, _stack: &mut dyn Stack) -> Result<()> {
        Err(Error::Unimplemented(
            "call should be handled by StacklessEngine, not StacklessFrame".to_string(),
        ))
    }

    fn call_indirect(
        &mut self,
        _type_idx: u32,
        _table_idx: u32,
        _entry_idx: u32,
        _stack: &mut dyn Stack,
    ) -> Result<()> {
        Err(Error::Unimplemented(
            "call_indirect should be handled by StacklessEngine, not StacklessFrame".to_string(),
        ))
    }

    // This is primarily managed by enter/exit block/loop/if
    fn set_label_arity(&mut self, arity: usize) {
        self.label_arity = arity;
    }
}

// Implement FrameBehavior trait for StacklessFrame
impl FrameBehavior for StacklessFrame {
    fn locals(&mut self) -> &mut Vec<Value> {
        &mut self.locals
    }

    fn get_local(&self, idx: usize) -> Result<Value> {
        self.locals.get(idx).cloned().ok_or_else(|| {
            Error::InvalidLocal(format!(
                "Local index out of bounds: {} (locals len: {})",
                idx,
                self.locals.len()
            ))
        })
    }

    fn set_local(&mut self, idx: usize, value: Value) -> Result<()> {
        if idx < self.locals.len() {
            // TODO: Type check? Should be guaranteed by validation?
            self.locals[idx] = value;
            Ok(())
        } else {
            Err(Error::InvalidLocal(format!(
                "Local index out of bounds: {} (locals len: {})",
                idx,
                self.locals.len()
            )))
        }
    }

    fn get_global(&self, idx: usize) -> Result<Arc<Global>> {
        // Reverted: Accessing globals via frame/module directly is problematic.
        // This should likely be handled via ModuleInstance or Engine context.
        // Err(Error::Unimplemented("get_global needs instance context".to_string()))
        // TEMP FIX: Delegate to module instance (assuming we can get it - THIS IS STILL WRONG)
        // Let's just return an error for now.
        Err(Error::InvalidGlobalIndex(idx)) // Or Unimplemented
    }

    fn set_global(&mut self, idx: usize, value: Value) -> Result<()> {
        // Reverted: Accessing globals via frame/module directly is problematic.
        // Err(Error::Unimplemented("set_global needs instance context".to_string()))
        Err(Error::InvalidGlobalIndex(idx)) // Or Unimplemented
    }

    // Memory operations delegate to the module's memory instance(s)
    fn get_memory(&self, idx: usize) -> Result<Arc<dyn MemoryBehavior>> {
        Ok(self.module.get_memory(idx)? as Arc<dyn MemoryBehavior>)
    }

    fn get_memory_mut(&mut self, idx: usize) -> Result<Arc<dyn MemoryBehavior>> {
        // MemoryBehavior uses interior mutability (&self), so get_memory is sufficient.
        self.get_memory(idx)
    }

    // Table operations delegate to the module's table instance(s)
    fn get_table(&self, idx: usize) -> Result<Arc<Table>> {
        self.module
            .tables
            .read()
            .map_err(|_| Error::PoisonedLock)? // Handle potential lock poisoning
            .get(idx)
            .cloned()
            .ok_or(Error::InvalidTableIndex(idx))
    }

    fn get_table_mut(&mut self, idx: usize) -> Result<Arc<Table>> {
        // Table uses interior mutability (RwLock), so get_table is sufficient.
        self.get_table(idx)
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
        self.arity // Function return arity
    }

    fn set_arity(&mut self, arity: usize) {
        // This usually shouldn't be set directly on the frame after creation.
        // Maybe log a warning?
        // self.arity = arity;
        println!("WARN: Attempted to set frame arity directly to {}", arity);
    }

    fn label_arity(&self) -> usize {
        self.label_arity // Current block's expected stack arity
    }

    fn return_pc(&self) -> usize {
        self.return_pc
    }

    fn set_return_pc(&mut self, pc: usize) {
        self.return_pc = pc;
    }

    fn as_any(&mut self) -> &mut dyn std::any::Any {
        self
    }

    // --- Memory access methods delegate to MemoryBehavior ---

    fn load_i32(&self, addr: usize, _align: u32) -> Result<i32> {
        let memory = self.get_memory(0)?; // Assuming memory index 0
        memory.read_i32(addr as u32)
    }

    fn load_i64(&self, addr: usize, _align: u32) -> Result<i64> {
        let memory = self.get_memory(0)?;
        memory.read_i64(addr as u32)
    }

    fn load_f32(&self, addr: usize, _align: u32) -> Result<f32> {
        let memory = self.get_memory(0)?;
        memory.read_f32(addr as u32)
    }

    fn load_f64(&self, addr: usize, _align: u32) -> Result<f64> {
        let memory = self.get_memory(0)?;
        memory.read_f64(addr as u32)
    }

    fn load_i8(&self, addr: usize, _align: u32) -> Result<i8> {
        let memory = self.get_memory(0)?;
        Ok(memory.read_byte(addr as u32)? as i8)
    }

    fn load_u8(&self, addr: usize, _align: u32) -> Result<u8> {
        let memory = self.get_memory(0)?;
        memory.read_byte(addr as u32)
    }

    fn load_i16(&self, addr: usize, _align: u32) -> Result<i16> {
        let memory = self.get_memory(0)?;
        Ok(memory.read_u16(addr as u32)? as i16)
    }

    fn load_u16(&self, addr: usize, _align: u32) -> Result<u16> {
        let memory = self.get_memory(0)?;
        memory.read_u16(addr as u32)
    }

    fn store_i32(&mut self, addr: usize, _align: u32, value: i32) -> Result<()> {
        let memory = self.get_memory_mut(0)?;
        memory.write_i32(addr as u32, value)
    }

    fn store_i64(&mut self, addr: usize, _align: u32, value: i64) -> Result<()> {
        let memory = self.get_memory_mut(0)?;
        memory.write_i64(addr as u32, value)
    }

    fn store_f32(&mut self, addr: usize, _align: u32, value: f32) -> Result<()> {
        let memory = self.get_memory_mut(0)?;
        memory.write_f32(addr as u32, value)
    }

    fn store_f64(&mut self, addr: usize, _align: u32, value: f64) -> Result<()> {
        let memory = self.get_memory_mut(0)?;
        memory.write_f64(addr as u32, value)
    }

    fn store_i8(&mut self, addr: usize, _align: u32, value: i8) -> Result<()> {
        let memory = self.get_memory_mut(0)?;
        memory.write_byte(addr as u32, value as u8)
    }

    fn store_u8(&mut self, addr: usize, _align: u32, value: u8) -> Result<()> {
        let memory = self.get_memory_mut(0)?;
        memory.write_byte(addr as u32, value)
    }

    fn store_i16(&mut self, addr: usize, _align: u32, value: i16) -> Result<()> {
        let memory = self.get_memory_mut(0)?;
        memory.write_u16(addr as u32, value as u16)
    }

    fn store_u16(&mut self, addr: usize, _align: u32, value: u16) -> Result<()> {
        let memory = self.get_memory_mut(0)?;
        memory.write_u16(addr as u32, value)
    }

    fn store_v128(&mut self, addr: usize, _align: u32, value: [u8; 16]) -> Result<()> {
        let memory = self.get_memory_mut(0)?;
        memory.write_v128(addr as u32, value)
    }

    fn get_function_type(&self, func_idx: u32) -> Result<FuncType> {
        todo!() // Placeholder
    }

    fn load_v128(&self, addr: usize, _align: u32) -> Result<[u8; 16]> {
        let memory = self.get_memory(0)?;
        memory.read_v128(addr as u32)
    }

    fn memory_size(&self) -> Result<u32> {
        todo!() // Placeholder
    }

    fn memory_grow(&mut self, pages: u32) -> Result<u32> {
        todo!() // Placeholder
    }

    fn table_get(&self, table_idx: u32, idx: u32) -> Result<Value> {
        todo!() // Placeholder
    }

    fn table_set(&mut self, table_idx: u32, idx: u32, value: Value) -> Result<()> {
        todo!() // Placeholder
    }

    fn table_size(&self, table_idx: u32) -> Result<u32> {
        todo!() // Placeholder
    }

    fn table_grow(&mut self, table_idx: u32, delta: u32, value: Value) -> Result<u32> {
        todo!() // Placeholder
    }

    fn table_init(
        &mut self,
        table_idx: u32,
        elem_idx: u32,
        dst: u32,
        src: u32,
        n: u32,
    ) -> Result<()> {
        todo!() // Placeholder
    }

    fn table_copy(
        &mut self,
        dst_table: u32,
        src_table: u32,
        dst: u32,
        src: u32,
        n: u32,
    ) -> Result<()> {
        todo!() // Placeholder
    }

    fn elem_drop(&mut self, elem_idx: u32) -> Result<()> {
        todo!() // Placeholder
    }

    fn table_fill(&mut self, table_idx: u32, dst: u32, val: Value, n: u32) -> Result<()> {
        todo!() // Placeholder
    }

    fn pop_bool(&mut self, stack: &mut dyn Stack) -> Result<bool> {
        todo!() // Placeholder
    }

    fn pop_i32(&mut self, stack: &mut dyn Stack) -> Result<i32> {
        todo!() // Placeholder
    }

    fn get_two_tables_mut(
        &mut self,
        idx1: u32,
        idx2: u32,
    ) -> Result<(std::sync::MutexGuard<Table>, std::sync::MutexGuard<Table>)> {
        todo!() // Placeholder
    }
}
