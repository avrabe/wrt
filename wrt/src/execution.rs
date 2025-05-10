use crate::stackless::StacklessEngine;
use crate::{
    behavior::{
        ControlFlow, FrameBehavior, InstructionExecutor, Label,
        /* NullBehavior, */ StackBehavior,
    },
    error::{kinds, Error, Result},
    instructions::Instruction,
    module::{ExportKind, Function, Module},
    prelude::TypesValue as Value,
    stackless::StacklessStack,
    stackless_frame::StacklessFrame,
};
use wrt_runtime::{GlobalType, Memory, Table};
use wrt_types::values::Value as RuntimeValue;

#[cfg(feature = "std")]
use std::{option::Option, string::ToString, sync::Arc};

#[cfg(not(feature = "std"))]
use alloc::{
    boxed::Box, collections::BTreeMap as HashMap, collections::BTreeSet as HashSet, format,
    string::ToString, sync::Arc, vec, vec::Vec,
};

#[cfg(not(feature = "std"))]
use crate::sync::Mutex;

use log::trace;

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

#[derive(Debug)]
pub struct ExecutionContext {
    pub memories: Vec<Arc<Memory>>,
    pub tables: Vec<Arc<Table>>,
    pub globals: Vec<RuntimeValue>,
    pub functions: Vec<Function>,
}

/// Execution statistics for WebAssembly runtime
#[derive(Debug, Clone, Default)]
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
    /// Count of fuel exhausted events
    pub fuel_exhausted_count: u64,
    /// Time spent in arithmetic operations (µs)
    #[cfg(feature = "std")]
    pub arithmetic_time_us: u64,
    /// Time spent in memory operations (µs)
    #[cfg(feature = "std")]
    pub memory_ops_time_us: u64,
    /// Time spent in function calls (µs)
    #[cfg(feature = "std")]
    pub function_call_time_us: u64,
    /// Memory read operations
    pub memory_reads: u64,
    /// Memory write operations
    pub memory_writes: u64,
    /// Memory grow operations
    pub memory_grows: u64,
    /// Collection push operations
    pub collection_pushes: u64,
    /// Collection pop operations
    pub collection_pops: u64,
    /// Collection lookup operations
    pub collection_lookups: u64,
    /// Collection insert operations
    pub collection_inserts: u64,
    /// Collection remove operations
    pub collection_removes: u64,
    /// Collection validate operations
    pub collection_validates: u64,
    /// Checksum calculations
    pub checksum_calculations: u64,
    /// Control flow operations
    pub control_flows: u64,
    /// Arithmetic operations
    pub arithmetic_ops: u64,
    /// Other operations
    pub other_ops: u64,
}

impl ExecutionStats {
    /// Create new execution stats
    pub fn new() -> Self {
        Self::default()
    }

    /// Update stats from operation summary
    pub fn update_from_operations(&mut self, ops: wrt_types::OperationSummary) {
        self.memory_reads = ops.memory_reads;
        self.memory_writes = ops.memory_writes;
        self.memory_grows = ops.memory_grows;
        self.collection_pushes = ops.collection_pushes;
        self.collection_pops = ops.collection_pops;
        self.collection_lookups = ops.collection_lookups;
        self.collection_inserts = ops.collection_inserts;
        self.collection_removes = ops.collection_removes;
        self.collection_validates = ops.collection_validates;
        self.checksum_calculations = ops.checksum_calculations;
        self.function_calls += ops.function_calls;
        self.control_flows = ops.control_flows;
        self.arithmetic_ops = ops.arithmetic_ops;
        self.other_ops = ops.other_ops;

        // Update aggregate stats
        self.memory_operations = ops.memory_reads + ops.memory_writes + ops.memory_grows;
        self.fuel_consumed += ops.fuel_consumed;
    }

    /// Reset all operation statistics
    pub fn reset_operations(&mut self) {
        self.memory_reads = 0;
        self.memory_writes = 0;
        self.memory_grows = 0;
        self.collection_pushes = 0;
        self.collection_pops = 0;
        self.collection_lookups = 0;
        self.collection_inserts = 0;
        self.collection_removes = 0;
        self.collection_validates = 0;
        self.checksum_calculations = 0;
        self.control_flows = 0;
        self.arithmetic_ops = 0;
        self.other_ops = 0;
    }

    /// Format execution statistics as a human-readable string
    #[cfg(feature = "std")]
    pub fn formatted(&self) -> String {
        use std::fmt::Write;
        let mut output = String::new();

        writeln!(&mut output, "Execution Statistics:").unwrap();
        writeln!(&mut output, "-----------------------").unwrap();
        writeln!(
            &mut output,
            "Instructions executed: {}",
            self.instructions_executed
        )
        .unwrap();
        writeln!(&mut output, "Function calls: {}", self.function_calls).unwrap();
        writeln!(&mut output, "Fuel consumed: {}", self.fuel_consumed).unwrap();
        if self.fuel_exhausted_count > 0 {
            writeln!(
                &mut output,
                "Fuel exhausted events: {}",
                self.fuel_exhausted_count
            )
            .unwrap();
        }

        writeln!(&mut output, "\nMemory Operations:").unwrap();
        writeln!(&mut output, "  - Read operations: {}", self.memory_reads).unwrap();
        writeln!(&mut output, "  - Write operations: {}", self.memory_writes).unwrap();
        writeln!(&mut output, "  - Grow operations: {}", self.memory_grows).unwrap();
        writeln!(
            &mut output,
            "  - Current memory: {} bytes",
            self.current_memory_bytes
        )
        .unwrap();
        writeln!(
            &mut output,
            "  - Peak memory: {} bytes",
            self.peak_memory_bytes
        )
        .unwrap();

        writeln!(&mut output, "\nCollection Operations:").unwrap();
        writeln!(
            &mut output,
            "  - Push operations: {}",
            self.collection_pushes
        )
        .unwrap();
        writeln!(&mut output, "  - Pop operations: {}", self.collection_pops).unwrap();
        writeln!(
            &mut output,
            "  - Lookup operations: {}",
            self.collection_lookups
        )
        .unwrap();
        writeln!(
            &mut output,
            "  - Insert operations: {}",
            self.collection_inserts
        )
        .unwrap();
        writeln!(
            &mut output,
            "  - Remove operations: {}",
            self.collection_removes
        )
        .unwrap();
        writeln!(
            &mut output,
            "  - Validate operations: {}",
            self.collection_validates
        )
        .unwrap();

        writeln!(&mut output, "\nVerification:").unwrap();
        writeln!(
            &mut output,
            "  - Checksum calculations: {}",
            self.checksum_calculations
        )
        .unwrap();

        #[cfg(feature = "std")]
        {
            writeln!(&mut output, "\nTiming:").unwrap();
            writeln!(
                &mut output,
                "  - Arithmetic operations: {}µs",
                self.arithmetic_time_us
            )
            .unwrap();
            writeln!(
                &mut output,
                "  - Memory operations: {}µs",
                self.memory_ops_time_us
            )
            .unwrap();
            writeln!(
                &mut output,
                "  - Function calls: {}µs",
                self.function_call_time_us
            )
            .unwrap();
        }

        output
    }
}

/// WebAssembly execution engine
#[derive(Debug)]
pub struct Engine {
    /// The modules loaded in the engine
    pub module: Module,
    /// The module instances active in the engine
    pub instances: Vec<ExecutionContext>,
    /// Remaining fuel for bounded execution (None means unlimited)
    pub fuel: Option<u64>,
    /// Execution statistics
    pub stats: ExecutionStats,
}

impl Engine {
    /// Create a new execution engine with the given module
    pub fn new(module: Module) -> Self {
        Self {
            module,
            instances: Vec::new(),
            fuel: None,
            stats: ExecutionStats::default(),
        }
    }

    /// Create a new engine from a module result
    pub fn new_from_result(module_result: Result<Module>) -> Result<Self> {
        module_result.map(|module| Self::new(module))
    }

    /// Instantiate a module, creating a new instance context
    pub fn instantiate(&mut self) -> Result<usize> {
        let context = ExecutionContext {
            memories: self.module.memories.clone(),
            tables: self.module.tables.clone(),
            globals: self
                .module
                .globals
                .iter()
                .map(|g| g.value.clone().into())
                .collect(),
            functions: self.module.functions.clone(),
        };

        self.instances.push(context);
        Ok(self.instances.len() - 1)
    }

    /// Get a memory instance from the specified instance
    pub fn get_memory(&self, instance_idx: usize, memory_idx: usize) -> Result<Arc<Memory>> {
        let instance = self
            .instances
            .get(instance_idx)
            .ok_or_else(|| Error::new(kinds::InvalidInstanceIndexError(instance_idx as u32)))?;

        instance
            .memories
            .get(memory_idx)
            .cloned()
            .ok_or_else(|| Error::new(kinds::InvalidMemoryIndexError(memory_idx as u32)))
    }

    /// Get a table instance from the specified instance
    pub fn get_table(&self, instance_idx: usize, table_idx: usize) -> Result<Arc<Table>> {
        let instance = self
            .instances
            .get(instance_idx)
            .ok_or_else(|| Error::new(kinds::InvalidInstanceIndexError(instance_idx as u32)))?;

        instance
            .tables
            .get(table_idx)
            .cloned()
            .ok_or_else(|| Error::new(kinds::InvalidTableIndexError(table_idx as u32)))
    }

    /// Execute a function in the specified instance
    pub fn execute(
        &mut self,
        instance_idx: usize,
        func_idx: usize,
        args: Vec<Value>,
    ) -> Result<Vec<Value>> {
        // Check if the instance exists
        if instance_idx >= self.instances.len() {
            return Err(Error::new(kinds::InvalidInstanceIndexError(
                instance_idx as u32,
            )));
        }

        // Check if the function exists
        let instance = &self.instances[instance_idx];
        if func_idx >= instance.functions.len() {
            return Err(Error::new(kinds::InvalidFunctionIndexError(
                func_idx as u32,
            )));
        }

        // This is where we would execute the function
        // For now just return an empty vector
        Ok(Vec::new())
    }
}

pub fn f32_nearest(a: &Value) -> f32 {
    /// Performs the nearest rounding operation on an f32 value.
    ///
    /// This implements the WebAssembly nearest rounding mode for f32 values,
    /// rounding to the nearest integer, with ties rounded to the nearest even integer.
    ///
    /// # Panics
    ///
    /// This function will panic if the provided value is not an F32 value.
    /// Safety impact: [LOW|MEDIUM|HIGH] - [Brief explanation of the safety implication]
    /// Tracking: WRTQ-XXX (qualification requirement tracking ID).
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
    /// Performs the nearest rounding operation on an f64 value.
    ///
    /// This implements the WebAssembly nearest rounding mode for f64 values,
    /// rounding to the nearest integer, with ties rounded to the nearest even integer.
    ///
    /// # Panics
    ///
    /// Safety impact: [LOW|MEDIUM|HIGH] - [Brief explanation of the safety implication]
    /// Tracking: WRTQ-XXX (qualification requirement tracking ID).
    /// This function will panic if the provided value is not an F64 value.
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

/// Internal function to parse floats from strings
pub fn parse_float<T: Into<f64> + From<f64>>(value_str: &str) -> Result<T> {
    let clean_str = value_str.trim();

    // Check for hex format
    if clean_str.starts_with("0x") || clean_str.starts_with("-0x") || clean_str.starts_with("+0x") {
        let parsed = parse_hex_float_internal(clean_str)?;
        Ok(T::from(parsed))
    } else {
        // Parse as decimal float
        match clean_str.parse::<f64>() {
            Ok(val) => Ok(T::from(val)),
            Err(_) => Err(Error::new(kinds::ParseError(format!(
                "Invalid float format: {}",
                value_str
            )))),
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
        return Err(Error::new(kinds::ParseError(format!(
            "Invalid hex float format: {}",
            hex_str
        ))));
    };

    // Split into integer and fractional parts
    let parts: Vec<&str> = hex_str.split('.').collect();
    if parts.len() > 2 {
        return Err(Error::new(kinds::ParseError(format!(
            "Invalid hex float format, multiple decimal points: {}",
            hex_str
        ))));
    };

    // Extract exponent if present
    let exponent = if parts.len() == 1 {
        // No decimal point, check for exponent
        if let Some(p_pos) = parts[0].to_lowercase().find('p') {
            let exp_str = &parts[0][p_pos + 1..];
            exp_str
                .parse::<i32>()
                .unwrap_or_else(|_| panic!("Invalid exponent: {}", exp_str))
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
                .unwrap_or_else(|_| panic!("Invalid exponent: {}", exp_str))
        } else {
            0
        }
    };

    // Parse the integer part
    let integer_part = if parts.len() > 0 && !parts[0].is_empty() {
        let int_part = if let Some(p_pos) = parts[0].to_lowercase().find('p') {
            &parts[0][..p_pos]
        } else {
            parts[0]
        };

        if !int_part.is_empty() {
            u64::from_str_radix(int_part, 16).map_err(|_| {
                Error::new(kinds::ParseError(format!(
                    "Invalid hex integer part: {}",
                    int_part
                )))
            })?
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
            let frac_val = u64::from_str_radix(frac_part, 16).map_err(|_| {
                Error::new(kinds::ParseError(format!(
                    "Invalid hex fractional part: {}",
                    frac_part
                )))
            })?;
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
    trace!("Execute export function: {:?}", export_name);
    trace!("Arguments: {:?}", args);

    let exports = &module.exports;
    let export = exports
        .iter()
        .find(|export| export.name == export_name.unwrap())
        .ok_or_else(|| {
            Error::new(
                kinds::EXPORT_NOT_FOUND_ERROR,
                export_name.unwrap().to_string(),
            )
        })?;

    if export.kind == ExportKind::Function {
        let func_idx = export.index;
        let func_type = module.get_function_type(func_idx).unwrap();
        trace!("Function type: {:?}", func_type);
        trace!("Expected result count: {}", func_type.results.len());

        let module_arc = Arc::new(module.clone());
        let mut stack = StacklessStack::new(module_arc.clone(), instance_idx);

        // Create the initial frame using from_function to handle both args and locals
        let mut frame = StacklessFrame::new(
            module_arc.clone(),
            func_idx,
            args.as_slice(),
            instance_idx.try_into().unwrap(),
        )
        .map_err(|e| Error::new(e))?;

        // Define func_code needed for label stack push below
        // func_code is already retrieved within from_function, maybe refactor later
        let func = module.get_function(func_idx).unwrap(); // Need func for code length
        let func_code = &func.code;

        // Push the implicit function block label
        let function_return_arity = func_type.results.len();
        frame.label_stack.push(Label {
            arity: 0,
            pc: 0, // Needs to be set after finding end instruction
            continuation: 0,
            stack_depth: 0,
            is_if: false,
            is_loop: false,
        });

        trace!(
            "DEBUG: execute_export_function - Initial Frame: {:?}",
            frame
        );

        // Execution loop using while and pc
        while frame.pc() < func_code.len() {
            let current_pc = frame.pc();
            // Check for return signal
            if frame.return_pc() == usize::MAX {
                trace!("DEBUG: execute_export_function - Detected return signal. Exiting loop.");
                break; // Exit loop if return was signaled
            }

            let instruction = &func_code[current_pc];
            trace!(
                "DEBUG: execute_export_function - PC: {}, Executing: {:?}, Stack: {:?}",
                current_pc,
                instruction,
                stack.values()
            );

            // Execute the instruction and handle control flow
            match execute_instruction(
                instruction,
                &mut stack,
                &mut frame,
                &mut StacklessEngine::new(),
            )? {
                ControlFlow::Continue => {
                    // Only increment PC if the instruction didn't modify it (e.g., not a branch or return)
                    if frame.pc() == current_pc {
                        frame.set_pc(current_pc + 1);
                    }
                }
                ControlFlow::Trap(err) => {
                    // Propagate trap errors
                    return Err(err);
                }
                // Other control flow types are unexpected in this simplified execution context
                // Branching, Returning, Calling should be handled within the instruction executor
                // or by the main engine loop, not this function-level execution.
                ControlFlow::Branch { .. } => {
                    // The instruction executor should have updated the PC directly.
                    // If we reach here, it might indicate an issue, but we assume the PC is correct.
                    // No explicit PC increment needed here.
                }
                ControlFlow::Return { .. } => {
                    // The return instruction should have set the frame's return_pc or signaled.
                    // Break the loop to handle result processing.
                    break;
                }
                ControlFlow::Call { .. } => {
                    return Err(Error::new(
                        kinds::EXECUTION_ERROR,
                        "Unexpected ControlFlow::Call in execute_export_function".to_string(),
                    ));
                }
            }

            trace!(
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

        // trace!("DEBUG: execute_export_function - Loop finished.");
        // trace!("Addr of stack AFTER loop: {:p}", &stack);
        // trace!("DEBUG: execute_export_function - Stack state BEFORE result retrieval: {:?}", stack.values());

        // Return results in the correct order
        let results_count = func_type.results.len();
        let stack_values = stack.values().to_vec();
        // trace!("DEBUG: execute_export_function - stack.values().to_vec() resulted in: {:?}", stack_values);

        let results = if results_count > 0 {
            let stack_len = stack_values.len();
            if stack_len >= results_count {
                stack_values[stack_len - results_count..].to_vec()
            } else {
                return Err(Error::new(
                    kinds::STACK_UNDERFLOW,
                    "Stack underflow during result extraction".to_string(),
                ));
            }
        } else {
            Vec::new()
        };

        trace!("Final results: {:?}", results);
        Ok(results)
    } else {
        Err(Error::new(
            kinds::EXECUTION_ERROR,
            "Invalid export kind".to_string(),
        ))
    }
}

pub fn execute_instruction(
    instruction: &Instruction,
    stack: &mut dyn StackBehavior,
    frame: &mut dyn FrameBehavior,
    engine: &mut StacklessEngine,
) -> Result<ControlFlow> {
    // Delegate execution to the instruction itself via the trait
    instruction.execute(stack, frame, engine)
}
