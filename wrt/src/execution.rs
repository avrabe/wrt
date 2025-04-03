use crate::{
    behavior::{
        ControlFlowBehavior, FrameBehavior, /*FrameBehavior,*/ Label,
        /* NullBehavior, */ StackBehavior,
    },
    error::{Error, Result},
    global::Global,
    instructions::{Instruction, InstructionExecutor},
    memory::DefaultMemory,
    module::{ExportKind, Function, Module},
    module_instance::ModuleInstance,
    stack::Stack,
    stackless::StacklessStack,
    stackless_frame::StacklessFrame,
    table::Table,
    values::Value,
};
use wast::core::Memory;

#[cfg(feature = "std")]
use std::{option::Option, println, string::ToString, sync::Arc};

#[cfg(not(feature = "std"))]
use alloc::{
    boxed::Box, collections::BTreeMap as HashMap, collections::BTreeSet as HashSet, format,
    string::ToString, sync::Arc, vec, vec::Vec,
};

#[cfg(not(feature = "std"))]
use crate::sync::Mutex;

/// Execution state for WebAssembly engine
#[derive(Debug, PartialEq, Eq)]
pub enum ExecutionState {
    /// Executing instructions normally
    Running,
    /// Paused execution (for bounded fuel)
    Paused {
        /// Instance index
        instance_idx: u32,
        /// Function index
        func_idx: u32,
        /// Program counter
        pc: usize,
        /// Expected results
        expected_results: usize,
    },
    /// Executing a function call
    Calling,
    /// Returning from a function
    Returning,
    /// Branching to a label
    Branching,
    /// Execution completed
    Completed,
    /// Execution finished
    Finished,
    /// Error during execution
    Error,
}

pub struct ExecutionContext {
    pub memory: Vec<u8>,
    pub table: Vec<Function>,
    pub globals: Vec<Value>,
    pub functions: Vec<Function>,
}

/// Execution statistics for monitoring and reporting
#[derive(Debug, Default, Clone)]
pub struct ExecutionStats {
    /// Number of instructions executed
    pub instructions_executed: u64,
    /// Number of function calls
    pub function_calls: u64,
    /// Number of memory operations
    pub memory_operations: u64,
    /// Current memory usage in bytes
    pub current_memory_bytes: u64,
    /// Peak memory usage in bytes
    pub peak_memory_bytes: u64,
    /// Amount of fuel consumed
    pub fuel_consumed: u64,
    /// Time spent in arithmetic operations (µs)
    #[cfg(feature = "std")]
    pub arithmetic_time_us: u64,
    /// Time spent in memory operations (µs)
    #[cfg(feature = "std")]
    pub memory_ops_time_us: u64,
    /// Time spent in function calls (µs)
    #[cfg(feature = "std")]
    pub function_call_time_us: u64,
}

/// The WebAssembly execution engine
#[derive(Debug)]
pub struct Engine<'a> {
    /// The execution stack
    pub stack: Box<dyn Stack>,
    /// The current execution state
    pub state: ExecutionState,
    /// Module instances
    pub instances: Vec<ModuleInstance>,
    /// Tables
    pub tables: Vec<Table>,
    /// Memories
    pub memories: Vec<Memory<'a>>,
    /// Globals
    pub globals: Vec<Global>,
    /// Execution statistics
    pub execution_stats: ExecutionStats,
    /// Remaining fuel for bounded execution
    pub fuel: Option<u64>,
    /// The module being executed
    pub module: Module,
}

/// Represents an execution frame
#[derive(Debug)]
pub struct Frame<'a> {
    /// Local variables
    pub locals: Vec<Value>,
    /// Memory instances
    pub memories: Vec<Memory<'a>>,
    /// Table instances
    pub tables: Vec<Table>,
    /// Global instances
    pub globals: Vec<Global>,
    /// Program counter
    pub pc: usize,
    /// Function index
    pub func_idx: u32,
    /// Instance index
    pub instance_idx: usize,
    /// Label stack
    pub label_stack: Vec<Label>,
    /// Return program counter
    pub return_pc: usize,
    /// Frame arity
    pub arity: usize,
    /// Label arity
    pub label_arity: usize,
}

impl Frame<'_> {
    /// Creates a new frame
    pub fn new() -> Self {
        Self {
            locals: Vec::new(),
            memories: Vec::new(),
            tables: Vec::new(),
            globals: Vec::new(),
            pc: 0,
            func_idx: 0,
            instance_idx: 0,
            label_stack: Vec::new(),
            return_pc: 0,
            arity: 0,
            label_arity: 0,
        }
    }
}

#[derive(Debug)]
pub struct ExecutionStack<'a> {
    pub value_stack: Vec<Value>,
    pub label_stack: Vec<Label>,
    pub frames: Vec<Frame<'a>>,
    pub instruction_count: usize,
}

impl Default for ExecutionStack<'_> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> ExecutionStack<'a> {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            value_stack: Vec::new(),
            label_stack: Vec::new(),
            frames: Vec::new(),
            instruction_count: 0,
        }
    }

    pub fn execute_instruction(
        &mut self,
        _instruction: &Instruction,
        _frame: &mut StacklessFrame,
    ) -> Result<()> {
        // Implementation of instruction execution
        // For now, just return Ok
        Ok(())
    }
}

impl<'a> StackBehavior for ExecutionStack<'a> {
    fn push(&mut self, value: Value) -> Result<()> {
        self.value_stack.push(value);
        Ok(())
    }

    fn pop(&mut self) -> Result<Value> {
        self.value_stack.pop().ok_or(Error::StackUnderflow)
    }

    fn peek(&self) -> Result<&Value> {
        self.value_stack.last().ok_or(Error::StackUnderflow)
    }

    fn peek_mut(&mut self) -> Result<&mut Value> {
        self.value_stack.last_mut().ok_or(Error::StackUnderflow)
    }

    fn len(&self) -> usize {
        self.value_stack.len()
    }

    fn is_empty(&self) -> bool {
        self.value_stack.is_empty()
    }

    fn push_label(&mut self, arity: usize, pc: usize) {
        self.label_stack.push(Label {
            arity,
            pc,
            continuation: 0,
        });
    }

    fn pop_label(&mut self) -> Result<Label> {
        self.label_stack.pop().ok_or(Error::StackUnderflow)
    }

    fn get_label(&self, idx: usize) -> Option<&Label> {
        self.label_stack.get(idx)
    }

    fn values(&self) -> &[Value] {
        &self.value_stack
    }

    fn values_mut(&mut self) -> &mut [Value] {
        &mut self.value_stack
    }
}

impl<'a> ExecutionStack<'a> {
    pub fn push_frame(&mut self, frame: Frame<'a>) {
        self.frames.push(frame);
    }

    pub fn pop_frame(&mut self) -> Result<Frame<'a>> {
        // Implementation of pop_frame
        // For now, just return a default frame
        self.frames.pop().ok_or(Error::StackUnderflow)
    }

    pub fn current_frame(&self) -> Result<&Frame<'a>> {
        // Implementation of current_frame
        // For now, just return an error
        Err(Error::InvalidOperation {
            message: "current_frame not implemented".to_string(),
        })
    }

    pub fn current_frame_mut(&mut self) -> Result<&mut Frame<'a>> {
        // Implementation of current_frame_mut
        // For now, just return an error
        Err(Error::InvalidOperation {
            message: "current_frame_mut not implemented".to_string(),
        })
    }

    pub fn pop_value(&mut self) -> Result<Value> {
        self.value_stack.pop().ok_or(Error::StackUnderflow)
    }
}

impl<'a> Stack for ExecutionStack<'a> {
    fn push_label(&mut self, label: crate::stack::Label) -> Result<()> {
        self.label_stack.push(Label {
            arity: label.arity,
            pc: label.pc,
            continuation: label.continuation,
        });
        Ok(())
    }

    fn pop_label(&mut self) -> Result<crate::stack::Label> {
        let label = self.label_stack.pop().ok_or(Error::StackUnderflow)?;
        Ok(crate::stack::Label {
            arity: label.arity,
            pc: label.pc,
            continuation: label.continuation,
        })
    }

    fn get_label(&self, _idx: usize) -> Result<&crate::stack::Label> {
        Err(Error::InvalidOperation {
            message: "get_label not implemented".to_string(),
        })
    }

    fn get_label_mut(&mut self, _idx: usize) -> Result<&mut crate::stack::Label> {
        Err(Error::InvalidOperation {
            message: "get_label_mut not implemented".to_string(),
        })
    }

    fn labels_len(&self) -> usize {
        self.label_stack.len()
    }
}

impl<'a> Engine<'a> {
    /// Creates a new engine with the given module
    #[must_use]
    pub fn new(module: Module) -> Self {
        let module_arc = Arc::new(module);
        Self {
            stack: Box::new(StacklessStack::new(module_arc.clone(), 0)),
            state: ExecutionState::Running,
            instances: Vec::new(),
            tables: Vec::new(),
            memories: Vec::new(),
            globals: Vec::new(),
            execution_stats: ExecutionStats::default(),
            fuel: None,
            module: Arc::try_unwrap(module_arc).unwrap_or_else(|arc| (*arc).clone()),
        }
    }

    /// Creates a new execution engine
    #[must_use]
    pub fn new_with_module(module: Module) -> Self {
        let module_arc = Arc::new(module);
        Self {
            stack: Box::new(StacklessStack::new(module_arc.clone(), 0)),
            state: ExecutionState::Running,
            instances: vec![],
            tables: vec![],
            memories: vec![],
            globals: vec![],
            execution_stats: ExecutionStats::default(),
            fuel: None,
            module: Arc::try_unwrap(module_arc).unwrap_or_else(|arc| (*arc).clone()),
        }
    }

    /// Creates a new Engine with a Module
    pub fn new_from_result(module_result: Result<Module>) -> Result<Self> {
        let module = module_result?;
        Ok(Self::new(module))
    }

    /// Check if the engine has no instances
    #[must_use]
    pub fn has_no_instances(&self) -> bool {
        self.instances.is_empty()
    }

    /// Get the remaining fuel (None for unlimited)
    #[must_use]
    pub const fn remaining_fuel(&self) -> Option<u64> {
        self.fuel
    }

    /// Gets a module instance by index
    pub fn get_instance(&self, instance_idx: usize) -> Result<&ModuleInstance> {
        self.instances
            .get(instance_idx)
            .ok_or_else(|| Error::Execution(format!("Invalid instance index: {instance_idx}")))
    }

    /// Adds a module instance to the engine
    pub fn add_instance(&mut self, instance: ModuleInstance) -> usize {
        let idx = self.instances.len();
        self.instances.push(instance);
        idx
    }

    /// Instantiates a module
    pub fn instantiate(&mut self, module: Module) -> Result<usize> {
        println!("Instantiating module with {} exports", module.exports.len());
        let instance = ModuleInstance::new(module)?;
        Ok(self.add_instance(instance))
    }

    /// Invokes an exported function
    pub fn invoke_export(&mut self, name: &str, args: &[Value]) -> Result<Vec<Value>> {
        // Find the export in instances
        for instance_idx in 0..self.instances.len() {
            let instance = &self.instances[instance_idx];

            // Look for the export in the module's exports
            for export in &instance.module.exports {
                if export.name == name && export.kind == ExportKind::Function {
                    let func = instance.module.get_function(export.index).ok_or_else(|| {
                        Error::Execution(format!("Function with index {} not found", export.index))
                    })?;

                    // For the remaining functions, execute them normally
                    return self.execute(instance_idx, export.index, args.to_vec());
                }
            }
        }

        Err(Error::ExportNotFound(format!(
            "Function '{}' not found",
            name
        )))
    }

    /// Executes a function
    pub fn execute(
        &mut self,
        instance_idx: usize,
        func_idx: u32,
        args: Vec<Value>,
    ) -> Result<Vec<Value>> {
        let instance = self.get_instance(instance_idx)?;

        // Get the function from the instance's module
        let _func = instance.module.get_function(func_idx).ok_or_else(|| {
            Error::Execution(format!("Function with index {} not found", func_idx))
        })?;

        // Get the export name if available
        let export_name = instance
            .module
            .exports
            .iter()
            .find(|export| export.kind == ExportKind::Function && export.index == func_idx)
            .map(|export| export.name.clone());

        // Execute the function generically by calling execute_export_function
        execute_export_function(&instance.module, instance_idx, export_name.as_deref(), args)
    }
}

pub fn f32_nearest(a: &Value) -> f32 {
    match a {
        Value::F32(a) => {
            if a.is_nan() || a.is_infinite() || *a == 0.0 {
                return *a;
            }

            let int_part = a.floor();
            let fract_part = a.fract().abs();

            if fract_part < 0.5 {
                return int_part;
            } else if fract_part > 0.5 {
                return int_part + 1.0;
            } else {
                if (int_part as i32) % 2 == 0 {
                    return int_part;
                } else {
                    return int_part + 1.0;
                }
            }
        }
        _ => panic!("Expected F32 value"),
    }
}

pub fn f64_nearest(a: &Value) -> f64 {
    match a {
        Value::F64(a) => {
            if a.is_nan() || a.is_infinite() || *a == 0.0 {
                return *a;
            }

            let int_part = a.floor();
            let fract_part = a.fract().abs();

            if fract_part < 0.5 {
                return int_part;
            } else if fract_part > 0.5 {
                return int_part + 1.0;
            } else {
                if (int_part as i64) % 2 == 0 {
                    return int_part;
                } else {
                    return int_part + 1.0;
                }
            }
        }
        _ => panic!("Expected F64 value"),
    }
}

/// Generic function to parse a float value from a string
/// Supports both decimal and hexadecimal formats with optional separators
pub fn parse_float<T: Into<f64> + From<f64>>(value_str: &str) -> Result<T> {
    // Remove underscores (separators)
    let clean_str = value_str.replace("_", "");

    // Check if it's a hexadecimal format
    if clean_str.starts_with("0x") || clean_str.starts_with("-0x") || clean_str.starts_with("+0x") {
        // Parse as hex float
        let parsed = parse_hex_float_internal(&clean_str)?;
        Ok(T::from(parsed))
    } else {
        // Parse as decimal float
        match clean_str.parse::<f64>() {
            Ok(val) => Ok(T::from(val)),
            Err(_) => Err(Error::Parse(format!("Invalid float format: {}", value_str))),
        }
    }
}

/// Internal function to parse hexadecimal float literals
fn parse_hex_float_internal(hex_str: &str) -> Result<f64> {
    // Check if the string starts with 0x or -0x
    let (is_negative, hex_str) = if hex_str.starts_with("-0x") {
        (true, &hex_str[3..])
    } else if hex_str.starts_with("0x") {
        (false, &hex_str[2..])
    } else if hex_str.starts_with("+0x") {
        (false, &hex_str[3..])
    } else {
        return Err(Error::Parse(format!(
            "Invalid hex float format: {}",
            hex_str
        )));
    };

    // Split into integer and fractional parts
    let parts: Vec<&str> = hex_str.split('.').collect();
    if parts.len() > 2 {
        return Err(Error::Parse(format!(
            "Invalid hex float format, multiple decimal points: {}",
            hex_str
        )));
    }

    // Extract exponent if present
    let exponent = if parts.len() == 1 {
        // No decimal point, check for exponent
        if let Some(p_pos) = parts[0].to_lowercase().find('p') {
            let exp_str = &parts[0][p_pos + 1..];
            exp_str
                .parse::<i32>()
                .map_err(|_| Error::Parse(format!("Invalid hex float exponent: {}", exp_str)))?
        } else {
            // No exponent
            0
        }
    } else {
        // Has decimal point, check for exponent in fractional part
        let frac_part = parts[1];
        if let Some(p_pos) = frac_part.to_lowercase().find('p') {
            let exp_str = &frac_part[p_pos + 1..];
            exp_str
                .parse::<i32>()
                .map_err(|_| Error::Parse(format!("Invalid hex float exponent: {}", exp_str)))?
        } else {
            // No exponent
            0
        };
    };

    // Parse the integer part
    let integer_part = if parts.len() > 0 && !parts[0].is_empty() {
        let int_part = if let Some(p_pos) = parts[0].to_lowercase().find('p') {
            &parts[0][..p_pos]
        } else {
            parts[0]
        };

        if !int_part.is_empty() {
            u64::from_str_radix(int_part, 16)
                .map_err(|_| Error::Parse(format!("Invalid hex integer part: {}", int_part)))?
        } else {
            0
        }
    } else {
        0
    };

    // Parse the fractional part if present
    let fractional_contribution = if parts.len() > 1 {
        let frac_part = if let Some(p_pos) = parts[1].to_lowercase().find('p') {
            &parts[1][..p_pos]
        } else {
            parts[1]
        };

        if !frac_part.is_empty() {
            // Convert hex fraction to decimal
            let frac_val = u64::from_str_radix(frac_part, 16)
                .map_err(|_| Error::Parse(format!("Invalid hex fractional part: {}", frac_part)))?;
            let frac_digits = frac_part.len() as u32;
            frac_val as f64 / 16.0f64.powi(frac_digits as i32)
        } else {
            0.0
        }
    } else {
        0.0
    };

    // Combine parts and apply exponent
    let mut value = integer_part as f64 + fractional_contribution;

    // Apply exponent (power of 2)
    if exponent != 0 {
        value *= 2.0f64.powi(exponent);
    }

    // Apply sign
    if is_negative {
        value = -value;
    }

    Ok(value)
}

/// Execute an export function by name from an instance
pub fn execute_export_function(
    module: &Module,
    instance_idx: usize,
    export_name: Option<&str>,
    args: Vec<Value>,
) -> Result<Vec<Value>> {
    println!("Execute export function: {:?}", export_name);
    println!("Arguments: {:?}", args);

    let exports = &module.exports;
    let export = exports
        .iter()
        .find(|export| export.name == export_name.unwrap())
        .ok_or_else(|| Error::ExportNotFound(export_name.unwrap().to_string()))?;

    if export.kind == ExportKind::Function {
        let func_idx = export.index;
        let func_type = module.get_function_type(func_idx).unwrap();
        println!("Function type: {:?}", func_type);
        println!("Expected result count: {}", func_type.results.len());

        let module_arc = Arc::new(module.clone());
        let mut stack = StacklessStack::new(module_arc.clone(), instance_idx);

        // Create the initial frame using from_function to handle both args and locals
        let mut frame = StacklessFrame::from_function(
            module_arc.clone(), // Pass Arc<Module>
            func_idx,
            &args, // Pass args as slice
            0,     // instance_idx, assuming 0 for now
        )?;

        // Define func_code needed for label stack push below
        // func_code is already retrieved within from_function, maybe refactor later
        let func = module.get_function(func_idx).unwrap(); // Need func for code length
        let func_code = &func.code;

        // Push the implicit function block label
        let function_return_arity = func_type.results.len();
        frame.label_stack.push(Label {
            arity: function_return_arity,
            // PC/Continuation for the function block itself.
            // Using MAX for continuation seems reasonable to signify function end.
            pc: func_code.len(), // Points just after the last instruction
            continuation: usize::MAX,
        });

        println!(
            "DEBUG: execute_export_function - Initial Frame: {:?}",
            frame
        );

        // Execution loop using while and pc
        while frame.pc() < func_code.len() {
            let current_pc = frame.pc();
            // Check for return signal
            if frame.return_pc() == usize::MAX {
                println!("DEBUG: execute_export_function - Detected return signal. Exiting loop.");
                break; // Exit loop if return was signaled
            }

            let instruction = &func_code[current_pc];
            println!(
                "DEBUG: execute_export_function - PC: {}, Executing: {:?}, Stack: {:?}",
                current_pc,
                instruction,
                stack.values()
            );

            execute_instruction(instruction, &mut stack, &mut frame)?;

            // Only increment PC if the instruction didn't modify it (e.g., not a branch or return)
            if frame.pc() == current_pc {
                frame.set_pc(current_pc + 1);
            }
            println!(
                "DEBUG: execute_export_function - PC after instr: {}, Return PC: {}",
                frame.pc(),
                frame.return_pc()
            );
        }

        /*
        // Manual execution for debugging (REMOVE)
        println!("Manual Execution Start");
        execute_instruction(&Instruction::LocalGet(0), &mut stack, &mut frame)?;
        println!("Stack after LocalGet(0): {:?}", stack.values());
        execute_instruction(&Instruction::LocalGet(1), &mut stack, &mut frame)?;
        println!("Stack after LocalGet(1): {:?}", stack.values());
        execute_instruction(&Instruction::I32And, &mut stack, &mut frame)?;
        println!("Stack after I32And: {:?}", stack.values());
        println!("Manual Execution End");
        */

        // println!("DEBUG: execute_export_function - Loop finished.");
        // println!("Addr of stack AFTER loop: {:p}", &stack);
        // println!("DEBUG: execute_export_function - Stack state BEFORE result retrieval: {:?}", stack.values());

        // Return results in the correct order
        let results_count = func_type.results.len();
        let stack_values = stack.values().to_vec();
        // println!("DEBUG: execute_export_function - stack.values().to_vec() resulted in: {:?}", stack_values);

        let results = if results_count > 0 {
            let stack_len = stack_values.len();
            if stack_len >= results_count {
                stack_values[stack_len - results_count..].to_vec()
            } else {
                return Err(Error::StackUnderflow);
            }
        } else {
            Vec::new()
        };

        println!("Final results: {:?}", results);
        Ok(results)
    } else {
        Err(Error::InvalidExport)
    }
}

pub fn execute_instruction(
    instruction: &Instruction,
    stack: &mut impl Stack,
    frame: &mut dyn FrameBehavior,
) -> Result<()> {
    // For debugging:
    //println!("Addr of stack INSIDE execute_instruction: {:p}", stack);
    //println!("In execute_instruction: {:?}", instruction);

    // Use the instruction's execute method
    instruction.execute(stack, frame)
}
