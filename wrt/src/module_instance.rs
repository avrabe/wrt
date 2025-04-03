use crate::{
    error::{Error, Result},
    global::Global,
    instructions::Instruction,
    memory::DefaultMemory as Memory,
    module::{Module, OtherExport},
    table::Table,
    types::FuncType,
};

/// Represents a module instance during execution
#[derive(Debug, Clone)]
pub struct ModuleInstance {
    /// Module index in the engine instances array
    pub module_idx: u32,
    /// Module definition
    pub module: Module,
    /// Function addresses
    pub func_addrs: Vec<FunctionAddr>,
    /// Table addresses
    pub table_addrs: Vec<TableAddr>,
    /// Memory addresses
    pub memory_addrs: Vec<MemoryAddr>,
    /// Global addresses
    pub global_addrs: Vec<GlobalAddr>,
    /// Actual memory instances with data buffers
    pub memories: Vec<Memory>,
    /// Actual table instances
    pub tables: Vec<Table>,
    /// Actual global instances
    pub globals: Vec<Global>,
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
    pub const fn new(module: Module) -> Result<Self> {
        Ok(Self {
            module,
            module_idx: 0,
            func_addrs: Vec::new(),
            table_addrs: Vec::new(),
            memory_addrs: Vec::new(),
            global_addrs: Vec::new(),
            memories: Vec::new(),
            tables: Vec::new(),
            globals: Vec::new(),
        })
    }

    /// Finds an export by name
    #[must_use]
    pub fn find_export(&self, name: &str) -> Option<&OtherExport> {
        self.module.exports.iter().find(|e| e.name == name)
    }

    /// Gets an export by name
    pub fn get_export(&self, name: &str) -> Result<&OtherExport> {
        self.find_export(name)
            .ok_or_else(|| Error::ExportNotFound(name.to_string()))
    }

    /// Gets a function type by index
    pub fn get_function_type(&self, func_idx: u32) -> Result<&FuncType> {
        self.module
            .types
            .get(func_idx as usize)
            .ok_or_else(|| Error::Execution(format!("Invalid function type index: {func_idx}")))
    }

    /// Gets a function instruction by index and program counter
    pub fn get_function_instruction(&self, func_idx: u32, pc: usize) -> Result<&Instruction> {
        let func = self
            .module
            .functions
            .get(func_idx as usize)
            .ok_or_else(|| Error::Execution(format!("Function index {func_idx} out of bounds")))?;

        func.code
            .get(pc)
            .ok_or_else(|| Error::Execution(format!("Instruction index {pc} out of bounds")))
    }
}
