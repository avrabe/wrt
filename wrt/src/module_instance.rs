use crate::{
    error::{self, kinds},
    error::{Error, Result},
    global::Global,
    instructions::Instruction,
    memory,
    module::Element,
    module::{Data, Module, OtherExport},
    prelude::TypesValue as Value,
    prelude::FuncType,
    prelude::*,
};
use std::sync::Arc;
use wrt_runtime::{Memory, Table};

/// Represents a module instance during execution
#[derive(Debug, Clone)]
pub struct ModuleInstance {
    /// Module index in the engine instances array
    pub module_idx: u32,
    /// Module definition (Now an Arc)
    pub module: Arc<Module>,
    /// Function addresses
    pub func_addrs: Vec<FunctionAddr>,
    /// Table addresses
    pub table_addrs: Vec<TableAddr>,
    /// Memory addresses
    pub memory_addrs: Vec<MemoryAddr>,
    /// Global addresses
    pub global_addrs: Vec<GlobalAddr>,
    /// Actual memory instances
    pub memories: Vec<Arc<Memory>>,
    /// Actual table instances
    pub tables: Vec<Arc<Table>>,
    /// Actual global instances
    pub globals: Vec<Arc<Global>>,
}

/// Represents a function address
#[derive(Debug, Clone)]
pub struct FunctionAddr {
    /// Module instance index
    pub instance_idx: u32,
    /// Function index
    pub func_idx: u32,
}

/// Represents a table address
#[derive(Debug, Clone, Copy)]
pub struct TableAddr {
    /// Module instance index
    pub instance_idx: u32,
    /// Table index
    pub table_idx: u32,
}

/// Represents a memory address
#[derive(Debug, Clone)]
pub struct MemoryAddr {
    /// Module instance index
    pub instance_idx: u32,
    /// Memory index
    pub memory_idx: u32,
}

/// Represents a global address
#[derive(Debug, Clone)]
pub struct GlobalAddr {
    /// Module instance index
    pub instance_idx: u32,
    /// Global index
    pub global_idx: u32,
}

impl ModuleInstance {
    /// Creates a new module instance
    pub fn new(module: Arc<Module>) -> Result<Self> {
        // Initialize tables, memories, and globals from the module
        let tables = module.tables.clone();
        let memories = module.memories.clone();
        let globals = module.globals.clone();

        Ok(Self {
            module,
            module_idx: 0,            // Placeholder, should be set by engine
            func_addrs: Vec::new(),   // Placeholder
            table_addrs: Vec::new(),  // Placeholder
            memory_addrs: Vec::new(), // Placeholder
            global_addrs: Vec::new(), // Placeholder
            memories,                 // Initialized from module
            tables,                   // Initialized from module
            globals,                  // Initialized from module
        })
    }

    /// Finds an export by name
    #[must_use]
    pub fn find_export(&self, name: &str) -> Option<&OtherExport> {
        self.module.exports.get(name)
    }

    /// Gets an export by name
    pub fn get_export(&self, name: &str) -> Result<&OtherExport> {
        self.find_export(name)
            .ok_or_else(|| Error::new(
                ErrorCategory::Validation,
                codes::VALIDATION_EXPORT_NOT_FOUND,
                kinds::ExportNotFoundError(name.to_string())
            ))
    }

    /// Gets a function type by index
    pub fn get_function_type(&self, func_idx: u32) -> Result<&FuncType> {
        self.module.types.get(func_idx as usize).ok_or_else(|| {
            Error::new(
                ErrorCategory::Validation,
                codes::VALIDATION_INVALID_FUNCTION_TYPE,
                kinds::InvalidFunctionTypeError(format!(
                    "Invalid function type index: {func_idx}"
                ))
            )
        })
    }

    /// Gets a function instruction by index and program counter
    pub fn get_function_instruction(&self, func_idx: u32, pc: usize) -> Result<&Instruction> {
        let func = self
            .module
            .functions
            .get(func_idx as usize)
            .ok_or_else(|| {
                Error::new(
                    ErrorCategory::Validation,
                    codes::VALIDATION_FUNCTION_NOT_FOUND,
                    kinds::FunctionNotFoundError(format!(
                        "Function index {func_idx} out of bounds"
                    ))
                )
            })?;

        func.code.get(pc).ok_or_else(|| {
            Error::new(
                ErrorCategory::Execution,
                codes::EXECUTION_INSTRUCTION_INDEX_OUT_OF_BOUNDS,
                kinds::ExecutionError(format!(
                    "Instruction index {pc} out of bounds"
                ))
            )
        })
    }

    /// Gets a table by index (returns Arc)
    pub fn get_table(&self, idx: usize) -> Result<Arc<Table>> {
        self.tables
            .get(idx)
            .cloned()
            .ok_or_else(|| Error::new(
                ErrorCategory::Validation,
                codes::VALIDATION_INVALID_TABLE_INDEX,
                kinds::InvalidTableIndexError(idx as u32)
            ))
    }

    /// Gets a table by index
    pub fn get_table_mut(&mut self, idx: usize) -> Result<Arc<Table>> {
        self.tables
            .get_mut(idx)
            .cloned()
            .ok_or_else(|| Error::new(
                ErrorCategory::Validation,
                codes::VALIDATION_INVALID_TABLE_INDEX,
                kinds::InvalidTableIndexError(idx as u32)
            ))
    }

    /// Set a value in the table at the specified index
    pub fn table_set(
        &mut self,
        table_idx: usize,
        elem_idx: u32,
        value: Option<Value>,
    ) -> Result<()> {
        // Get a reference to the table
        let table = self.get_table_mut(table_idx)?;

        // Since we can't mutate the Arc<Table> directly, we need to use a workaround
        // Create a mutable clone of the table
        let mut table_clone = (*table).clone();

        // Perform the operation on the clone
        let result = table_clone.set(elem_idx, value);

        // Replace the original table with the modified clone
        if result.is_ok() {
            if let Some(table_ref) = self.tables.get_mut(table_idx) {
                *table_ref = Arc::new(table_clone);
            }
        }

        result
    }

    /// Grow the table by the specified amount
    pub fn table_grow(&mut self, table_idx: usize, delta: u32, init_value: Value) -> Result<u32> {
        // Get a reference to the table
        let table = self.get_table_mut(table_idx)?;

        // Since we can't mutate the Arc<Table> directly, we need to use a workaround
        // Create a mutable clone of the table
        let mut table_clone = (*table).clone();

        // Perform the operation on the clone
        let result = table_clone.grow(delta, init_value);

        // Replace the original table with the modified clone
        if let Ok(_) = result {
            if let Some(table_ref) = self.tables.get_mut(table_idx) {
                *table_ref = Arc::new(table_clone);
            }
        }

        result
    }

    /// Sets a global value by index (modifies Arc<Global>)
    pub fn set_global(&mut self, idx: usize, value: Value) -> Result<()> {
        if let Some(global_arc) = self.globals.get(idx) {
            // Global::set takes &self due to interior mutability (Mutex/Atomic)
            global_arc.set(&value)
        } else {
            Err(Error::new(
                ErrorCategory::Validation,
                codes::VALIDATION_INVALID_GLOBAL_INDEX,
                kinds::InvalidGlobalIndexError(idx as u32)
            ))
        }
    }

    /// Gets an element segment by index
    pub fn get_element_segment(&self, idx: u32) -> Result<&Element> {
        self.module
            .elements
            .get(idx as usize)
            .ok_or_else(|| Error::new(
                ErrorCategory::Validation,
                codes::VALIDATION_INVALID_ELEMENT_INDEX,
                kinds::InvalidElementIndexError(idx)
            ))
    }

    /// Gets a memory by index
    pub fn get_memory(&self, idx: usize) -> Result<Arc<Memory>> {
        self.memories
            .get(idx)
            .cloned()
            .ok_or_else(|| Error::new(
                ErrorCategory::Validation,
                codes::VALIDATION_INVALID_MEMORY_INDEX,
                kinds::InvalidMemoryIndexError(idx as u32)
            ))
    }

    /// Gets a memory by index (for mutable access)
    pub fn get_memory_mut(&mut self, idx: usize) -> Result<Arc<Memory>> {
        self.memories
            .get(idx)
            .cloned()
            .ok_or_else(|| Error::new(
                ErrorCategory::Validation,
                codes::VALIDATION_INVALID_MEMORY_INDEX,
                kinds::InvalidMemoryIndexError(idx as u32)
            ))
    }

    /// Gets a global by index
    pub fn get_global(&self, idx: usize) -> Result<Arc<Global>> {
        self.globals
            .get(idx)
            .cloned()
            .ok_or_else(|| Error::new(
                ErrorCategory::Validation,
                codes::VALIDATION_INVALID_GLOBAL_INDEX,
                kinds::InvalidGlobalIndexError(idx as u32)
            ))
    }

    /// Gets a data segment by index
    pub fn get_data(&self, idx: u32) -> Result<&Data> {
        self.module
            .data
            .get(idx as usize)
            .ok_or_else(|| Error::new(
                ErrorCategory::Validation,
                codes::VALIDATION_INVALID_DATA_SEGMENT_INDEX,
                kinds::InvalidDataSegmentIndexError(idx)
            ))
    }

    /// Drops a data segment by index
    pub fn drop_data_segment(&mut self, _idx: u32) -> Result<()> {
        // In the original implementation, this would set data[idx] to None
        // But since we're using Vec<Data>, not Vec<Option<Data>>, we should
        // implement a mechanism to track dropped data segments - for now just a stub
        Ok(())
    }

    /// Sets a data segment by index
    pub fn set_data_segment(&mut self, _idx: u32, _segment: Arc<Data>) -> Result<()> {
        // This would update data[idx], but again we need a proper implementation
        // if we're using Vec<Data> instead of Vec<Option<Arc<Data>>>
        Ok(())
    }

    /// Gets two tables for mutable access, ensuring they're different
    pub fn get_two_tables_mut(&mut self, idx1: u32, idx2: u32) -> Result<(Arc<Table>, Arc<Table>)> {
        if idx1 == idx2 {
            return Err(Error::new(
                ErrorCategory::Validation,
                codes::VALIDATION_DUPLICATE_TABLE_REFERENCE,
                kinds::ValidationError(format!(
                    "Cannot get mutable references to the same table: {idx1} == {idx2}"
                ))
            ));
        }
        let table1 = self.get_table(idx1 as usize)?;
        let table2 = self.get_table(idx2 as usize)?;
        Ok((table1, table2))
    }

    /// Drops an element segment by index
    pub fn elem_drop(&mut self, elem_idx: u32) -> Result<()> {
        // Get the element segment to verify it exists
        self.get_element_segment(elem_idx)?;

        // As with data segments, we'd need a mechanism to track dropped elements
        // For now, just return Ok
        Ok(())
    }

    /// Gets a function address by index
    pub fn get_func_addr(&self, func_idx: u32) -> Result<FunctionAddr> {
        if func_idx as usize >= self.func_addrs.len() {
            return Err(Error::new(
                ErrorCategory::Validation,
                codes::VALIDATION_INVALID_FUNCTION_INDEX,
                kinds::InvalidFunctionIndexError(func_idx)
            ));
        }
        Ok(self.func_addrs[func_idx as usize].clone())
    }
}
