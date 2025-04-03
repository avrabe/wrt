use crate::{
    behavior::{self, ControlFlowBehavior, FrameBehavior},
    error::{Error, Result},
    global::Global,
    instructions::{types::BlockType, Instruction},
    memory::{DefaultMemory, MemoryBehavior},
    stack::Stack,
    table::Table,
    types::{ExternType, GlobalType, MemoryType, TableType},
    types::{FuncType, ValueType},
    values::Value,
    String, Vec,
};

use std::sync::{Arc, RwLock};

#[cfg(not(feature = "std"))]
use core::fmt;

// Use debug_println macro as println for no_std environment
#[cfg(not(feature = "std"))]
use crate::debug_println as println;

#[cfg(not(feature = "std"))]
use alloc::vec;

#[cfg(feature = "std")]
use std::string::ToString;

#[cfg(not(feature = "std"))]
use alloc::string::ToString;

/// Represents the address of a table within a module instance.
/// Used for indirect function calls.
#[derive(Debug, Clone, Copy)]
pub struct TableAddr {
    /// The index of the table within the module's tables section.
    pub table_idx: u32,
}

/// Represents a WebAssembly module
#[derive(Debug, Clone)]
pub struct Module {
    /// Module types (function signatures)
    pub types: Vec<FuncType>,
    /// Imported functions, tables, memories, and globals
    pub imports: Vec<Import>,
    /// Function definitions
    pub functions: Vec<Function>,
    /// Table definitions
    pub tables: Arc<RwLock<Vec<Arc<Table>>>>,
    /// Memory definitions
    pub memories: Arc<RwLock<Vec<Arc<DefaultMemory>>>>,
    /// Global variable definitions
    pub globals: Arc<RwLock<Vec<Arc<Global>>>>,
    /// Element segments for tables
    pub elements: Vec<Element>,
    /// Data segments for memories
    pub data: Vec<Data>,
    /// Start function index
    pub start: Option<u32>,
    /// Custom sections
    pub custom_sections: Vec<CustomSection>,
    /// Exports (functions, tables, memories, and globals)
    pub exports: Vec<OtherExport>,
    /// Optional name for the module, often derived from the custom "name" section.
    pub name: Option<String>,
    /// Original binary (if available)
    pub binary: Option<Vec<u8>>,
    /// Table addresses for indirect function calls
    pub table_addrs: Vec<TableAddr>,
    /// Local variables for the module
    pub locals: Vec<Value>,
    /// Label arity for the module
    pub label_arity: usize,
}

impl Default for Module {
    fn default() -> Self {
        Self::new().expect("Failed to create default module")
    }
}

/// Represents an import in a WebAssembly module
#[derive(Debug, Clone)]
pub struct Import {
    /// Module name
    pub module: String,
    /// Import name
    pub name: String,
    /// Import type
    pub ty: ExternType,
}

/// Represents a WebAssembly function
#[derive(Debug, Clone)]
pub struct Function {
    /// The type index of the function
    pub type_idx: u32,
    /// The local variables of the function
    pub locals: Vec<ValueType>,
    /// The instructions that make up the function body
    pub code: Vec<Instruction>,
}

impl Function {
    /// Creates a new function with the given type index, locals, and code
    #[must_use]
    pub const fn new(type_idx: u32, locals: Vec<ValueType>, code: Vec<Instruction>) -> Self {
        Self {
            type_idx,
            locals,
            code,
        }
    }
}

/// Represents an element segment for tables
#[derive(Debug, Clone)]
pub struct Element {
    /// Table index
    pub table_idx: u32,
    /// Offset expression
    pub offset: Vec<Instruction>,
    /// Function indices
    pub init: Vec<u32>,
}

/// Represents a data segment for memories
#[derive(Debug, Clone)]
pub struct Data {
    /// Memory index
    pub memory_idx: u32,
    /// Offset expression
    pub offset: Vec<Instruction>,
    /// Initial data
    pub init: Vec<u8>,
}

/// Represents a custom section in a WebAssembly module
#[derive(Debug, Clone)]
pub struct CustomSection {
    /// Section name
    pub name: String,
    /// Section data
    pub data: Vec<u8>,
}

/// Export kind
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExportKind {
    /// Function export
    Function,
    /// Table export
    Table,
    /// Memory export
    Memory,
    /// Global export
    Global,
}

/// Represents an export in a WebAssembly module
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OtherExport {
    /// Export name
    pub name: String,
    /// Export kind
    pub kind: ExportKind,
    /// Export index
    pub index: u32,
}

/// Represents the value of an export
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExportValue {
    /// A function with the specified index
    Function(u32),
    /// A table with the specified index
    Table(u32),
    /// A memory with the specified index
    Memory(u32),
    /// A global with the specified index
    Global(u32),
}

/// Represents an index into one of the module's sections
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum ExportItem {
    /// A function with the specified index
    Function(u32),
    /// A table with the specified index
    Table(u32),
    /// A memory with the specified index
    Memory(u32),
    /// A global with the specified index
    Global(u32),
}

/// Represents a WebAssembly code section entry
#[derive(Debug, Clone)]
pub struct Code {
    /// The size of the code section entry
    pub size: u32,
    /// The local declarations
    pub locals: Vec<(u32, ValueType)>,
    /// The function body (instructions)
    pub expr: Vec<Instruction>,
}

impl Code {
    /// Creates a new code section entry with the given size, locals, and expression
    #[must_use]
    pub const fn new(size: u32, locals: Vec<(u32, ValueType)>, expr: Vec<Instruction>) -> Self {
        Self { size, locals, expr }
    }
}

impl Module {
    /// Creates a new empty module
    pub fn new() -> Result<Self> {
        Ok(Self {
            types: Vec::new(),
            imports: Vec::new(),
            functions: Vec::new(),
            tables: Arc::new(RwLock::new(Vec::new())),
            memories: Arc::new(RwLock::new(Vec::new())),
            globals: Arc::new(RwLock::new(Vec::new())),
            elements: Vec::new(),
            data: Vec::new(),
            start: None,
            exports: Vec::new(),
            custom_sections: Vec::new(),
            name: None,
            binary: None,
            table_addrs: Vec::new(),
            locals: Vec::new(),
            label_arity: 0,
        })
    }

    /// Loads a WebAssembly binary and creates a Module.
    ///
    /// This method validates the binary format and returns a parsed Module.
    pub fn load_from_binary(&mut self, bytes: &[u8]) -> Result<Self> {
        if bytes.len() < 8 {
            return Err(Error::Parse("Binary too short".into()));
        }

        // Check magic number and version
        if bytes[0..8] == [0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00] {
            self.load_wasm_binary(bytes)
        } else if bytes[0..8] == [0x00, 0x61, 0x73, 0x6D, 0x0D, 0x00, 0x01, 0x00] {
            self.load_component_binary(bytes)
        } else {
            Err(Error::Parse("Invalid binary format".into()))
        }
    }

    /// Load a WebAssembly module binary
    fn load_wasm_binary(&self, bytes: &[u8]) -> Result<Self> {
        let mut module = self.clone();

        // Clear existing definitions
        module.memories.write().unwrap().clear();
        module.functions.clear();
        module.imports.clear();
        module.exports.clear();
        module.globals.write().unwrap().clear();
        module.data.clear();
        module.elements.clear();
        module.tables.write().unwrap().clear();
        module.types.clear();
        module.custom_sections.clear();

        // Initialize module from binary
        parse_module(&mut module, bytes)?;

        Ok(module)
    }

    /// Load a WebAssembly component binary
    fn load_component_binary(&self, bytes: &[u8]) -> Result<Self> {
        let mut module = self.clone();

        // Clear existing definitions
        module.memories.write().unwrap().clear();
        module.functions.clear();
        module.imports.clear();
        module.exports.clear();
        module.globals.write().unwrap().clear();
        module.data.clear();
        module.elements.clear();
        module.tables.write().unwrap().clear();
        module.types.clear();
        module.custom_sections.clear();

        // Parse the component binary
        parse_component(&mut module, bytes)?;

        Ok(module)
    }

    /// Validates the module
    ///
    /// This function checks that the module is valid according to the WebAssembly spec
    pub fn validate(&self) -> Result<()> {
        // Validate function types
        for func in &self.functions {
            if func.type_idx as usize >= self.types.len() {
                return Err(Error::Parse(format!(
                    "Invalid function type index: {}",
                    func.type_idx
                )));
            }
        }

        // Validate exports
        for export in &self.exports {
            match export.kind {
                ExportKind::Function => {
                    if export.index as usize >= self.functions.len() {
                        return Err(Error::Parse(format!(
                            "Invalid function export index: {}",
                            export.index
                        )));
                    }
                }
                ExportKind::Table => {
                    if export.index as usize >= self.tables.read().unwrap().len() {
                        return Err(Error::Parse(format!(
                            "Invalid table export index: {}",
                            export.index
                        )));
                    }
                }
                ExportKind::Memory => {
                    if export.index as usize >= self.memories.read().unwrap().len() {
                        return Err(Error::Parse(format!(
                            "Invalid memory export index: {}",
                            export.index
                        )));
                    }
                }
                ExportKind::Global => {
                    if export.index as usize >= self.globals.read().unwrap().len() {
                        return Err(Error::Parse(format!(
                            "Invalid global export index: {}",
                            export.index
                        )));
                    }
                }
            }
        }

        // All good
        Ok(())
    }

    #[cfg(feature = "serialization")]
    /// Serialize the module to a binary format
    pub fn to_binary(&self) -> crate::error::Result<Vec<u8>> {
        use crate::error::Error;

        // For now, we'll use the original binary if available,
        // otherwise recreate from the parsed module
        if let Some(binary) = &self.binary {
            Ok(binary.clone())
        } else {
            // In a real implementation, regenerate the binary from the module
            // For now, return an error as this is not yet implemented
            Err(Error::Validation(
                "Serializing a module without original binary is not yet supported".into(),
            ))
        }
    }

    /// Creates a Module from WebAssembly binary bytes
    ///
    /// # Parameters
    ///
    /// * `bytes` - The WebAssembly binary bytes
    ///
    /// # Returns
    ///
    /// The parsed module, or an error if the binary is invalid
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        Self::new()?.load_from_binary(bytes)
    }

    /// Creates an empty module
    #[must_use]
    pub fn empty() -> Self {
        Self {
            types: Vec::new(),
            imports: Vec::new(),
            functions: Vec::new(),
            tables: Arc::new(RwLock::new(Vec::new())),
            memories: Arc::new(RwLock::new(Vec::new())),
            globals: Arc::new(RwLock::new(Vec::new())),
            exports: Vec::new(),
            elements: Vec::new(),
            data: Vec::new(),
            custom_sections: Vec::new(),
            binary: None,
            start: None,
            name: None,
            table_addrs: Vec::new(),
            locals: Vec::new(),
            label_arity: 0,
        }
    }

    /// Creates a Module from a WebAssembly Text Format (WAT) string.
    ///
    /// # Arguments
    ///
    /// * `wat`: A string slice containing the WAT representation of the module.
    ///
    /// # Returns
    ///
    /// A `Result` containing the parsed `Module` on success, or an `Error` if parsing fails.
    #[cfg(feature = "serialization")]
    pub fn from_wat(wat: &str) -> Result<Self> {
        let wasm = wat::parse_str(wat)?;
        Self::from_bytes(&wasm)
    }

    /// Gets an export by name
    #[must_use]
    pub fn get_export(&self, name: &str) -> Option<&OtherExport> {
        self.exports.iter().find(|e| e.name == name)
    }

    /// Adds a function export
    pub fn add_function_export(&mut self, name: String, index: u32) {
        self.exports.push(OtherExport {
            name,
            kind: ExportKind::Function,
            index,
        });
    }

    /// Adds a table export
    pub fn add_table_export(&mut self, name: String, index: u32) {
        self.exports.push(OtherExport {
            name,
            kind: ExportKind::Table,
            index,
        });
    }

    /// Adds a memory export
    pub fn add_memory_export(&mut self, name: String, index: u32) {
        self.exports.push(OtherExport {
            name,
            kind: ExportKind::Memory,
            index,
        });
    }

    /// Adds a global export
    pub fn add_global_export(&mut self, name: String, index: u32) {
        self.exports.push(OtherExport {
            name,
            kind: ExportKind::Global,
            index,
        });
    }

    /// Gets a reference to a function definition by its index.
    ///
    /// # Arguments
    ///
    /// * `idx`: The index of the function.
    ///
    /// # Returns
    ///
    /// An `Option` containing a reference to the `Function` if found, otherwise `None`.
    #[must_use]
    pub fn get_function(&self, idx: u32) -> Option<&Function> {
        self.functions.get(idx as usize)
    }

    /// Gets a reference to the function type (signature) for a given function index.
    ///
    /// # Arguments
    ///
    /// * `idx`: The index of the function.
    ///
    /// # Returns
    ///
    /// An `Option` containing a reference to the `FuncType` if the function and its type exist,
    /// otherwise `None`.
    #[must_use]
    pub fn get_function_type(&self, idx: u32) -> Option<&FuncType> {
        self.types
            .get(self.functions.get(idx as usize)?.type_idx as usize)
    }

    /// Gets a global by index
    pub fn get_global(&self, idx: usize) -> Result<Arc<Global>> {
        let globals = self.globals.read().map_err(|_| Error::PoisonedLock)?;
        globals
            .get(idx)
            .cloned()
            .ok_or_else(|| Error::InvalidGlobalIndex(idx))
    }

    /// Gets a memory by index
    pub fn get_memory(&self, idx: usize) -> Result<Arc<DefaultMemory>> {
        self.memories
            .read()
            .unwrap()
            .get(idx)
            .cloned()
            .ok_or_else(|| Error::InvalidMemoryIndex(idx))
    }

    /// Returns the number of memory definitions in the module.
    pub fn memories_len(&self) -> usize {
        self.memories.read().map_or(0, |memories| memories.len())
    }

    /// Returns the number of table definitions in the module.
    pub fn tables_len(&self) -> usize {
        self.tables.read().map_or(0, |tables| tables.len())
    }

    /// Returns the number of global definitions in the module.
    pub fn globals_len(&self) -> usize {
        self.globals.read().map_or(0, |globals| globals.len())
    }

    /// Evaluates a constant expression (used for global/element/data offsets).
    ///
    /// Note: This requires an initial `stack` usually containing globals.
    /// For simple const exprs like `i32.const`, the stack can be empty.
    fn evaluate_const_expr(
        &self,
        expr: &[Instruction],
        // Initial stack, typically containing imported globals if needed by expr.
        initial_stack: &mut Vec<Value>,
    ) -> Result<Value> {
        let mut stack = initial_stack.clone(); // Use a local stack
        for instr in expr {
            match instr {
                Instruction::I32Const(val) => stack.push(Value::I32(*val)),
                Instruction::I64Const(val) => stack.push(Value::I64(*val)),
                Instruction::F32Const(val) => stack.push(Value::F32(*val)),
                Instruction::F64Const(val) => stack.push(Value::F64(*val)),
                Instruction::GlobalGet(idx) => {
                    // Attempt to get from initial stack first (could be imported global)
                    if (*idx as usize) < stack.len() {
                        // Assume initial_stack holds globals in order. This might be fragile.
                        // A better approach might involve passing global values explicitly.
                        stack.push(stack[*idx as usize].clone());
                    } else {
                        // Fallback to module globals (should only happen if not imported)
                        let globals = self.globals.read().map_err(|_| Error::PoisonedLock)?;
                        let global = globals
                            .get(*idx as usize)
                            .ok_or(Error::InvalidGlobalIndex(*idx as usize))?;
                        stack.push(global.get_value()?);
                    }
                }
                // Other const instructions like ref.null, ref.func could be added here if needed
                _ => {
                    return Err(Error::InvalidConstant(
                        "Unsupported instruction in const expression".to_string(),
                    ))
                }
            }
        }
        stack.pop().ok_or(Error::InvalidConstant(
            "Const expression evaluation resulted in empty stack".to_string(),
        ))
    }

    /// Creates a global variable instance based on its definition and initial value expression.
    pub fn create_global(
        &mut self,
        global_type: GlobalType,
        init_expr: Vec<Instruction>,
    ) -> Result<Arc<Global>> {
        // For constant expressions, the initial stack is usually empty,
        // unless the expression relies on imported globals (which is complex).
        // We assume simple const exprs here.
        let mut initial_stack = Vec::new();
        let initial_value = self.evaluate_const_expr(&init_expr, &mut initial_stack)?;
        let global = Arc::new(Global::new(global_type, initial_value)?);
        self.globals
            .write()
            .map_err(|_| Error::PoisonedLock)?
            .push(global.clone());
        Ok(global)
    }
}

/// Returns the decoded value and the number of bytes read
fn read_leb128_u32(bytes: &[u8]) -> Result<(u32, usize)> {
    let mut result = 0u32;
    let mut shift = 0;
    let mut bytes_read = 0;
    let mut byte;

    loop {
        byte = bytes.get(bytes_read).ok_or(Error::UnexpectedEof)?;
        bytes_read += 1;

        result |= u32::from(byte & 0x7f) << shift;
        shift += 7;

        if (byte & 0x80) == 0 {
            break;
        }

        if shift >= 32 {
            return Err(Error::InvalidLeb128("LEB128 value too large".to_string()));
        }
    }

    Ok((result, bytes_read))
}

/// Returns the decoded signed value and the number of bytes read
fn read_leb128_i32(bytes: &[u8]) -> Result<(i32, usize)> {
    let mut result = 0i32;
    let mut shift = 0;
    let mut bytes_read = 0;
    let mut byte;
    let mut sign_bit_set = false;

    loop {
        byte = bytes.get(bytes_read).ok_or(Error::UnexpectedEof)?;
        bytes_read += 1;

        // Apply the 7 bits to the result
        result |= (i32::from(byte & 0x7f)) << shift;
        shift += 7;

        // Check if we're done
        if (byte & 0x80) == 0 {
            // Check if the sign bit (bit 6 in the last byte) is set
            sign_bit_set = (byte & 0x40) != 0;
            break;
        }

        if shift >= 32 {
            return Err(Error::InvalidLeb128("LEB128 value too large".to_string()));
        }
    }

    // Sign extend if necessary
    if sign_bit_set && shift < 32 {
        // Fill in the sign extension bits
        result |= !0 << shift;
    }

    Ok((result, bytes_read))
}

/// Returns the decoded signed value and the number of bytes read
fn read_leb128_i64(bytes: &[u8]) -> Result<(i64, usize)> {
    let mut result = 0i64;
    let mut shift = 0;
    let mut bytes_read = 0;
    let mut byte;
    let mut sign_bit_set = false;

    loop {
        byte = bytes.get(bytes_read).ok_or(Error::UnexpectedEof)?;
        bytes_read += 1;

        // Apply the 7 bits to the result
        result |= (i64::from(byte & 0x7f)) << shift;
        shift += 7;

        // Check if we're done
        if (byte & 0x80) == 0 {
            // Check if the sign bit (bit 6 in the last byte) is set
            sign_bit_set = (byte & 0x40) != 0;
            break;
        }

        if shift >= 64 {
            return Err(Error::InvalidLeb128("LEB128 value too large".to_string()));
        }
    }

    // Sign extend if necessary
    if sign_bit_set && shift < 64 {
        // Fill in the sign extension bits
        result |= !0 << shift;
    }

    Ok((result, bytes_read))
}

fn parse_module(module: &mut Module, bytes: &[u8]) -> Result<()> {
    // Parse module header
    if bytes.len() < 8 {
        return Err(Error::InvalidModule("Invalid module".to_string()));
    }

    // Check magic number
    if bytes[0..4] != [0x00, 0x61, 0x73, 0x6D] {
        return Err(Error::InvalidModule("Invalid module".to_string()));
    }

    // Check version
    if bytes[4..8] != [0x01, 0x00, 0x00, 0x00] {
        return Err(Error::InvalidModule("Invalid module".to_string()));
    }

    // Parse sections
    let mut offset = 8;
    while offset < bytes.len() {
        let section_id = bytes[offset];
        offset += 1;

        let (size, bytes_read) = read_leb128_u32(&bytes[offset..])?;
        offset += bytes_read;
        let section_content_start_offset = offset; // Start of the section's content
        let expected_section_end_offset = section_content_start_offset + size as usize; // Expected end

        // Ensure the declared section size doesn't exceed remaining bytes
        if expected_section_end_offset > bytes.len() {
            return Err(Error::Parse(format!(
                "Section size {} for section ID {} exceeds remaining bytes {}",
                size,
                section_id,
                bytes.len() - section_content_start_offset
            )));
        }

        match section_id {
            // Type section
            0x01 => {
                let (count, bytes_read) = read_leb128_u32(&bytes[offset..])?;
                offset += bytes_read;

                for _ in 0..count {
                    let (func_type, bytes_read) = read_func_type(&bytes[offset..])?;
                    offset += bytes_read;
                    module.types.push(func_type);
                }
            }
            // Import section
            0x02 => {
                let (count, bytes_read) = read_leb128_u32(&bytes[offset..])?;
                offset += bytes_read;

                for _ in 0..count {
                    let (import, bytes_read) = read_import(&bytes[offset..])?;
                    offset += bytes_read;
                    module.imports.push(import);
                }
            }
            // Function section
            0x03 => {
                let (count, bytes_read) = read_leb128_u32(&bytes[offset..])?;
                offset += bytes_read;

                for _ in 0..count {
                    let (type_idx, bytes_read) = read_leb128_u32(&bytes[offset..])?;
                    offset += bytes_read;
                    module
                        .functions
                        .push(Function::new(type_idx, Vec::new(), Vec::new()));
                }
            }
            // Table section
            0x04 => {
                let (count, bytes_read) = read_leb128_u32(&bytes[offset..])?;
                offset += bytes_read;

                for _ in 0..count {
                    let (table, bytes_read) = read_table(&bytes[offset..])?;
                    offset += bytes_read;
                    module.tables.write().unwrap().push(Arc::clone(&table));
                }
            }
            // Memory section
            0x05 => {
                let (count, bytes_read) = read_leb128_u32(&bytes[offset..])?;
                offset += bytes_read;

                for _ in 0..count {
                    let (memory, bytes_read) = read_memory(&bytes[offset..])?;
                    offset += bytes_read;
                    module.memories.write().unwrap().push(Arc::clone(&memory));
                }
            }
            // Global section
            0x06 => {
                let (count, bytes_read) = read_leb128_u32(&bytes[offset..])?;
                offset += bytes_read;

                for _ in 0..count {
                    let (global, bytes_read) = read_global(&bytes[offset..])?;
                    offset += bytes_read;
                    module.globals.write().unwrap().push(Arc::clone(&global));
                }
            }
            // Export section
            0x07 => {
                let (count, bytes_read) = read_leb128_u32(&bytes[offset..])?;
                offset += bytes_read;

                for _ in 0..count {
                    let (export, bytes_read) = read_export(&bytes[offset..])?;
                    offset += bytes_read;
                    module.exports.push(export);
                }
            }
            // Start section
            0x08 => {
                let (start_func, bytes_read) = read_leb128_u32(&bytes[offset..])?;
                offset += bytes_read;
                module.start = Some(start_func);
            }
            // Element section
            0x09 => {
                let (count, bytes_read) = read_leb128_u32(&bytes[offset..])?;
                offset += bytes_read;

                for _ in 0..count {
                    let (element, bytes_read) = read_element(&bytes[offset..])?;
                    offset += bytes_read;
                    module.elements.push(element);
                }
            }
            // Code section
            0x0A => {
                let (count, bytes_read) = read_leb128_u32(&bytes[offset..])?;
                offset += bytes_read;

                // Ensure function section has been processed and matches code count
                if module.functions.len() < count as usize {
                    return Err(Error::InvalidModule(
                        "Code section count exceeds function section count".to_string(),
                    ));
                }

                for idx in 0..count as usize {
                    // Use index
                    let (code, bytes_read) = read_code(&bytes[offset..])?;
                    offset += bytes_read;
                    // Assign code and locals to the correct function index
                    module.functions[idx].code = code.expr;
                    module.functions[idx].locals = code
                        .locals
                        .iter()
                        .map(|&(_count, val_type)| val_type)
                        .collect(); // Extract ValueType
                }
            }
            // Data section
            0x0B => {
                let (count, bytes_read) = read_leb128_u32(&bytes[offset..])?;
                offset += bytes_read;

                for _ in 0..count {
                    let (data, bytes_read) = read_data(&bytes[offset..])?;
                    offset += bytes_read;
                    module.data.push(data);
                }
            }
            // Custom section
            0x00 => {
                // Skip custom section
                offset += size as usize;
            }
            // Unknown section
            _ => {
                return Err(Error::InvalidModule("Invalid module".to_string()));
            }
        }

        // Validate that the number of bytes consumed matches the section size
        if offset != expected_section_end_offset {
            return Err(Error::Parse(format!(
                "Section size mismatch for section ID {}: expected end offset {}, but got {}",
                section_id, expected_section_end_offset, offset
            )));
        }
    }

    Ok(())
}

fn parse_component(module: &mut Module, bytes: &[u8]) -> Result<()> {
    // Store the original binary
    module.binary = Some(bytes.to_vec());

    // Create a custom section to indicate this is a component
    module.custom_sections.push(CustomSection {
        name: "component-model-info".to_string(),
        data: vec![0x01], // 0x01 indicates this is a component
    });

    // Simple validation of the header
    if bytes.len() < 8 {
        return Err(Error::InvalidModule("Component binary too short".into()));
    }

    // Check magic number
    if &bytes[0..4] != [0x00, 0x61, 0x73, 0x6D] {
        return Err(Error::InvalidModule(
            "Invalid component magic number".into(),
        ));
    }

    // Check version
    if &bytes[4..8] != [0x0D, 0x00, 0x01, 0x00] {
        return Err(Error::InvalidModule("Invalid component version".into()));
    }

    // For now, just validate that there's at least the required type section
    if bytes.len() <= 8 {
        return Err(Error::InvalidModule(
            "Missing required component sections".into(),
        ));
    }

    Ok(())
}

fn read_func_type(bytes: &[u8]) -> Result<(FuncType, usize)> {
    if bytes.is_empty() {
        return Err(Error::Parse("Empty function type section".into()));
    }

    // The first byte should be 0x60 for function type
    if bytes[0] != 0x60 {
        return Err(Error::Parse(format!(
            "Invalid function type tag: 0x{:02x}, expected 0x60",
            bytes[0]
        )));
    }

    let mut offset = 1;

    // Read parameter count (leb128 encoded)
    let (param_count, param_bytes_read) = read_leb128_u32(&bytes[offset..])?;
    offset += param_bytes_read;

    // Read parameters
    let mut params = Vec::with_capacity(param_count as usize);
    for _ in 0..param_count {
        if offset >= bytes.len() {
            return Err(Error::Parse("Unexpected end of function type bytes".into()));
        }

        let value_type = match bytes[offset] {
            0x7F => ValueType::I32,
            0x7E => ValueType::I64,
            0x7D => ValueType::F32,
            0x7C => ValueType::F64,
            0x70 => ValueType::FuncRef,
            0x6F => ValueType::ExternRef,
            0x7B => ValueType::V128,
            _ => {
                return Err(Error::Parse(format!(
                    "Invalid value type: 0x{:02x}",
                    bytes[offset]
                )));
            }
        };
        params.push(value_type);
        offset += 1;
    }

    // Read result count (leb128 encoded)
    let (result_count, result_bytes_read) = read_leb128_u32(&bytes[offset..])?;
    offset += result_bytes_read;

    // Read results
    let mut results = Vec::with_capacity(result_count as usize);
    for _ in 0..result_count {
        if offset >= bytes.len() {
            return Err(Error::Parse("Unexpected end of function type bytes".into()));
        }

        let value_type = match bytes[offset] {
            0x7F => ValueType::I32,
            0x7E => ValueType::I64,
            0x7D => ValueType::F32,
            0x7C => ValueType::F64,
            0x70 => ValueType::FuncRef,
            0x6F => ValueType::ExternRef,
            0x7B => ValueType::V128,
            _ => {
                return Err(Error::Parse(format!(
                    "Invalid value type: 0x{:02x}",
                    bytes[offset]
                )));
            }
        };
        results.push(value_type);
        offset += 1;
    }

    Ok((FuncType { params, results }, offset))
}

fn read_import(bytes: &[u8]) -> Result<(Import, usize)> {
    // TODO: Implement import reading
    Err(Error::InvalidImport(
        "Import reading not implemented".into(),
    ))
}

fn read_table(bytes: &[u8]) -> Result<(Arc<Table>, usize)> {
    let table_type = TableType {
        element_type: ValueType::FuncRef,
        min: 0,
        max: None,
    };
    let table = Table::new(table_type);
    Ok((Arc::new(table), 0))
}

fn read_memory(bytes: &[u8]) -> Result<(Arc<DefaultMemory>, usize)> {
    let mut offset = 0;

    // Read flags
    if offset >= bytes.len() {
        return Err(Error::Parse("Unexpected end of memory type bytes".into()));
    }
    let flags = bytes[offset];
    offset += 1;

    // Read min limit (LEB128)
    let (min, bytes_read) = read_leb128_u32(&bytes[offset..])?;
    offset += bytes_read;

    // Read max limit if flags indicate it's present
    let max = if flags & 0x01 != 0 {
        let (max_val, bytes_read) = read_leb128_u32(&bytes[offset..])?;
        offset += bytes_read;
        Some(max_val)
    } else {
        None
    };

    let memory_type = MemoryType { min, max };
    let memory = DefaultMemory::new(memory_type);
    Ok((Arc::new(memory), offset))
}

fn read_global(bytes: &[u8]) -> Result<(Arc<Global>, usize)> {
    let global_type = GlobalType {
        content_type: ValueType::I32,
        mutable: false,
    };
    let global = Global::new(global_type, Value::I32(0))?;
    Ok((Arc::new(global), 0))
}

fn read_export(bytes: &[u8]) -> Result<(OtherExport, usize)> {
    if bytes.is_empty() {
        return Err(Error::Parse("Empty export section".into()));
    }

    let mut offset = 0;

    // Read export name length (LEB128)
    let (name_len, name_len_bytes) = read_leb128_u32(&bytes[offset..])?;
    offset += name_len_bytes;

    if offset + name_len as usize > bytes.len() {
        return Err(Error::Parse("Export name exceeds available bytes".into()));
    }

    // Read export name
    let name_bytes = &bytes[offset..offset + name_len as usize];
    let name = match std::str::from_utf8(name_bytes) {
        Ok(s) => s.to_string(),
        Err(_) => return Err(Error::Parse("Invalid UTF-8 sequence in export name".into())),
    };
    offset += name_len as usize;

    // Read export kind
    if offset >= bytes.len() {
        return Err(Error::Parse("Unexpected end of export bytes".into()));
    }

    let kind = match bytes[offset] {
        0x00 => ExportKind::Function,
        0x01 => ExportKind::Table,
        0x02 => ExportKind::Memory,
        0x03 => ExportKind::Global,
        _ => {
            return Err(Error::Parse(format!(
                "Invalid export kind: 0x{:02x}",
                bytes[offset]
            )))
        }
    };
    offset += 1;

    // Read export index (LEB128)
    let (index, index_bytes) = read_leb128_u32(&bytes[offset..])?;
    offset += index_bytes;

    Ok((OtherExport { name, kind, index }, offset))
}

fn read_element(bytes: &[u8]) -> Result<(Element, usize)> {
    // TODO: Implement element reading
    Err(Error::InvalidElement(
        "Element reading not implemented".into(),
    ))
}

fn read_code(bytes: &[u8]) -> Result<(Code, usize)> {
    if bytes.is_empty() {
        return Err(Error::Parse("Empty code section".into()));
    }

    let initial_offset = 0; // Assuming bytes slice starts exactly at the code entry
    let mut offset = initial_offset;

    // Read the size of the code section entry
    let (size, size_bytes_read) = read_leb128_u32(&bytes[offset..])?;
    offset += size_bytes_read;
    let expected_end_offset = offset + size as usize; // Calculate expected end based on declared size

    // Read local declarations
    let (local_count, local_count_bytes) = read_leb128_u32(&bytes[offset..])?;
    offset += local_count_bytes;

    let mut locals = Vec::with_capacity(local_count as usize);
    // let mut total_bytes_read = size_bytes_read + local_count_bytes; // No longer needed

    // Read local entries
    for _ in 0..local_count {
        if offset >= bytes.len() {
            return Err(Error::Parse(
                "Unexpected end of code section while reading locals".into(),
            ));
        }

        // Read local count
        let (count, count_bytes) = read_leb128_u32(&bytes[offset..])?;
        offset += count_bytes;
        // total_bytes_read += count_bytes;

        // Read local type
        if offset >= bytes.len() {
            return Err(Error::Parse(
                "Unexpected end of code section while reading local type".into(),
            ));
        }

        let value_type = match bytes[offset] {
            0x7F => ValueType::I32,
            0x7E => ValueType::I64,
            0x7D => ValueType::F32,
            0x7C => ValueType::F64,
            0x70 => ValueType::FuncRef,
            0x6F => ValueType::ExternRef,
            0x7B => ValueType::V128,
            _ => {
                return Err(Error::Parse(format!(
                    "Invalid value type: 0x{:02x}",
                    bytes[offset]
                )));
            }
        };
        offset += 1;
        // total_bytes_read += 1;

        locals.push((count, value_type));
    }

    // Read expression (instructions)
    let mut expr = Vec::new();
    let function_body_start = offset;
    // let function_body_size = (size as usize).saturating_sub(total_bytes_read - size_bytes_read); // No longer needed

    // Decode instructions until the expected end offset is reached
    // let mut i = 0; // Remove i counter
    while offset < expected_end_offset {
        if offset >= bytes.len() {
            // Check bounds before reading opcode
            return Err(Error::Parse("Unexpected end of code section body".into()));
        }
        let opcode = bytes[offset];
        offset += 1;
        // i += 1; // Remove i increment

        let instruction = match opcode {
            0x00 => Instruction::Unreachable,
            0x01 => Instruction::Nop,
            0x02 => {
                // Block
                if offset >= expected_end_offset {
                    return Err(Error::Parse("Unexpected end for Block type".into()));
                }
                let block_type = match bytes[offset] {
                    0x40 => BlockType::Empty,
                    0x7F => BlockType::Value(ValueType::I32),
                    0x7E => BlockType::Value(ValueType::I64),
                    0x7D => BlockType::Value(ValueType::F32),
                    0x7C => BlockType::Value(ValueType::F64),
                    _ => BlockType::Empty,
                };
                offset += 1;
                // i += 1; // Remove i increment
                Instruction::Block(block_type)
            }
            0x03 => {
                // Loop
                if offset >= expected_end_offset {
                    return Err(Error::Parse("Unexpected end for Loop type".into()));
                }
                let block_type = match bytes[offset] {
                    0x40 => BlockType::Empty,
                    0x7F => BlockType::Value(ValueType::I32),
                    0x7E => BlockType::Value(ValueType::I64),
                    0x7D => BlockType::Value(ValueType::F32),
                    0x7C => BlockType::Value(ValueType::F64),
                    _ => BlockType::Empty,
                };
                offset += 1;
                // i += 1; // Remove i increment
                Instruction::Loop(block_type)
            }
            0x04 => {
                // If
                if offset >= expected_end_offset {
                    return Err(Error::Parse("Unexpected end for If type".into()));
                }
                let block_type = match bytes[offset] {
                    0x40 => BlockType::Empty,
                    0x7F => BlockType::Value(ValueType::I32),
                    0x7E => BlockType::Value(ValueType::I64),
                    0x7D => BlockType::Value(ValueType::F32),
                    0x7C => BlockType::Value(ValueType::F64),
                    _ => BlockType::Empty,
                };
                offset += 1;
                // i += 1; // Remove i increment
                Instruction::If(block_type)
            }
            0x05 => Instruction::Else,
            0x06..=0x0A => Instruction::Nop,
            0x0B => Instruction::End,
            0x0C => {
                // br
                let (label_idx, bytes_read) = read_leb128_u32(&bytes[offset..])?;
                if offset + bytes_read > expected_end_offset {
                    return Err(Error::Parse("Immediate overrun for br".into()));
                }
                offset += bytes_read;
                // i += bytes_read; // Remove i increment
                Instruction::Br(label_idx)
            }
            0x0D => {
                // br_if
                let (label_idx, bytes_read) = read_leb128_u32(&bytes[offset..])?;
                if offset + bytes_read > expected_end_offset {
                    return Err(Error::Parse("Immediate overrun for br_if".into()));
                }
                offset += bytes_read;
                // i += bytes_read; // Remove i increment
                Instruction::BrIf(label_idx)
            }
            0x10 => {
                // call
                let (func_idx, bytes_read) = read_leb128_u32(&bytes[offset..])?;
                if offset + bytes_read > expected_end_offset {
                    return Err(Error::Parse("Immediate overrun for call".into()));
                }
                offset += bytes_read;
                // i += bytes_read; // Remove i increment
                Instruction::Call(func_idx)
            }
            0x14 => {
                // table.size
                let (table_idx, bytes_read) = read_leb128_u32(&bytes[offset..])?;
                if offset + bytes_read > expected_end_offset {
                    return Err(Error::Parse("Immediate overrun for table.size".into()));
                }
                offset += bytes_read;
                // i += bytes_read; // Remove i increment
                Instruction::TableSize(table_idx)
            }
            0x1A => Instruction::Drop,
            0x1e => {
                // i32.load8_s
                let (align, align_bytes_read) = read_leb128_u32(&bytes[offset..])?;
                if offset + align_bytes_read > expected_end_offset {
                    return Err(Error::Parse(
                        "Immediate overrun for i32.load8_s align".into(),
                    ));
                }
                offset += align_bytes_read;
                // i += align_bytes_read; // Remove i increment
                let (mem_offset, offset_bytes_read) = read_leb128_u32(&bytes[offset..])?;
                if offset + offset_bytes_read > expected_end_offset {
                    return Err(Error::Parse(
                        "Immediate overrun for i32.load8_s offset".into(),
                    ));
                }
                offset += offset_bytes_read;
                // i += offset_bytes_read; // Remove i increment
                Instruction::I32Load8S(align, mem_offset)
            }
            0x20 => {
                // local.get
                let (local_idx, bytes_read) = read_leb128_u32(&bytes[offset..])?;
                if offset + bytes_read > expected_end_offset {
                    return Err(Error::Parse("Immediate overrun for local.get".into()));
                }
                offset += bytes_read;
                // i += bytes_read; // Remove i increment
                Instruction::LocalGet(local_idx)
            }
            0x21 => {
                // local.set
                let (local_idx, bytes_read) = read_leb128_u32(&bytes[offset..])?;
                if offset + bytes_read > expected_end_offset {
                    return Err(Error::Parse("Immediate overrun for local.set".into()));
                }
                offset += bytes_read;
                // i += bytes_read; // Remove i increment
                Instruction::LocalSet(local_idx)
            }
            0x22 => {
                // local.tee
                let (local_idx, bytes_read) = read_leb128_u32(&bytes[offset..])?;
                if offset + bytes_read > expected_end_offset {
                    return Err(Error::Parse("Immediate overrun for local.tee".into()));
                }
                offset += bytes_read;
                // i += bytes_read; // Remove i increment
                Instruction::LocalTee(local_idx)
            }
            0x23 => {
                // global.get
                let (global_idx, bytes_read) = read_leb128_u32(&bytes[offset..])?;
                if offset + bytes_read > expected_end_offset {
                    return Err(Error::Parse("Immediate overrun for global.get".into()));
                }
                offset += bytes_read;
                // i += bytes_read; // Remove i increment
                Instruction::GlobalGet(global_idx)
            }
            0x24 => {
                // global.set
                let (global_idx, bytes_read) = read_leb128_u32(&bytes[offset..])?;
                if offset + bytes_read > expected_end_offset {
                    return Err(Error::Parse("Immediate overrun for global.set".into()));
                }
                offset += bytes_read;
                // i += bytes_read; // Remove i increment
                Instruction::GlobalSet(global_idx)
            }
            0x28 => {
                // i32.load
                let (align, align_bytes_read) = read_leb128_u32(&bytes[offset..])?;
                if offset + align_bytes_read > expected_end_offset {
                    return Err(Error::Parse("Immediate overrun for i32.load align".into()));
                }
                offset += align_bytes_read;
                // i += align_bytes_read;
                let (mem_offset, offset_bytes_read) = read_leb128_u32(&bytes[offset..])?;
                if offset + offset_bytes_read > expected_end_offset {
                    return Err(Error::Parse("Immediate overrun for i32.load offset".into()));
                }
                offset += offset_bytes_read;
                // i += offset_bytes_read;
                Instruction::I32Load(align, mem_offset)
            }
            0x36 => {
                // i32.store
                let (align, align_bytes_read) = read_leb128_u32(&bytes[offset..])?;
                if offset + align_bytes_read > expected_end_offset {
                    return Err(Error::Parse("Immediate overrun for i32.store align".into()));
                }
                offset += align_bytes_read;
                let (mem_offset, offset_bytes_read) = read_leb128_u32(&bytes[offset..])?;
                if offset + offset_bytes_read > expected_end_offset {
                    return Err(Error::Parse(
                        "Immediate overrun for i32.store offset".into(),
                    ));
                }
                offset += offset_bytes_read;
                Instruction::I32Store(align, mem_offset)
            }
            0x41 => {
                // i32.const
                let (val, bytes_read) = read_leb128_i32(&bytes[offset..])?;
                if offset + bytes_read > expected_end_offset {
                    return Err(Error::Parse("Immediate overrun for i32.const".into()));
                }
                offset += bytes_read;
                // i += bytes_read;
                Instruction::I32Const(val)
            }
            0x42 => {
                // i64.const
                let (val, bytes_read) = read_leb128_i64(&bytes[offset..])?;
                if offset + bytes_read > expected_end_offset {
                    return Err(Error::Parse("Immediate overrun for i64.const".into()));
                }
                offset += bytes_read;
                // i += bytes_read;
                Instruction::I64Const(val)
            }
            0x43 => {
                // f32.const
                if offset + 4 > expected_end_offset {
                    return Err(Error::Parse("Immediate overrun for f32.const".into()));
                }
                let val = f32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap());
                offset += 4;
                // i += 4;
                Instruction::F32Const(val)
            }
            0x44 => {
                // f64.const
                if offset + 8 > expected_end_offset {
                    return Err(Error::Parse("Immediate overrun for f64.const".into()));
                }
                let val = f64::from_le_bytes(bytes[offset..offset + 8].try_into().unwrap());
                offset += 8;
                // i += 8;
                Instruction::F64Const(val)
            }
            0x45..=0x69 => Instruction::Nop, // Placeholder for simple numeric ops
            0x6a => Instruction::I32Add,
            0x6b => Instruction::I32Sub,
            0x6c => Instruction::I32Mul,
            0x6d => Instruction::I32DivS,
            0x6e => Instruction::I32DivU,
            0x6f => Instruction::I32RemS,
            0x70 => Instruction::I32RemU,
            0x71 => Instruction::I32And,
            0x72 => Instruction::I32Or,
            0x73 => Instruction::I32Xor,
            0x74 => Instruction::I32Shl,
            0x75 => Instruction::I32ShrS,
            0x76 => Instruction::I32ShrU,
            0x77 => Instruction::I32Rotl,
            0x78 => Instruction::I32Rotr,
            0x7c => Instruction::F64Add,
            0x7d => Instruction::F64Sub,
            0x7e => Instruction::F64Mul,
            0x7f => Instruction::F64Div,
            0x8b => Instruction::F32Abs,
            0x8c => Instruction::F32Neg,
            0x8d => Instruction::F32Ceil,
            0x8e => Instruction::F32Floor,
            0x8f => Instruction::F32Trunc,
            0x90 => Instruction::F32Nearest,
            0x91 => Instruction::F32Sqrt,
            0x92 => Instruction::F32Add,
            0x93 => Instruction::F32Sub,
            0x94 => Instruction::F32Mul,
            0x95 => Instruction::F32Div,
            0x96 => Instruction::F32Min,
            0x97 => Instruction::F32Max,
            0x98 => Instruction::F32Copysign,
            0xfd => {
                if offset >= expected_end_offset {
                    return Err(Error::Parse("Unexpected end after 0xfd prefix".into()));
                }
                let simd_opcode = bytes[offset];
                offset += 1;
                // i += 1; // Remove i increment

                match simd_opcode {
                    0x00 => {
                        // v128.load
                        let (align, align_bytes) = read_leb128_u32(&bytes[offset..])?;
                        if offset + align_bytes > expected_end_offset {
                            return Err(Error::Parse(
                                "Immediate overrun for v128.load align".into(),
                            ));
                        }
                        offset += align_bytes;
                        // i += align_bytes;
                        let (mem_offset, offset_bytes) = read_leb128_u32(&bytes[offset..])?;
                        if offset + offset_bytes > expected_end_offset {
                            return Err(Error::Parse(
                                "Immediate overrun for v128.load offset".into(),
                            ));
                        }
                        offset += offset_bytes;
                        // i += offset_bytes;
                        Instruction::V128Load(align, mem_offset)
                    }
                    0x0B => {
                        // v128.store
                        let (align, align_bytes) = read_leb128_u32(&bytes[offset..])?;
                        if offset + align_bytes > expected_end_offset {
                            return Err(Error::Parse(
                                "Immediate overrun for v128.store align".into(),
                            ));
                        }
                        offset += align_bytes;
                        // i += align_bytes;
                        let (mem_offset, offset_bytes) = read_leb128_u32(&bytes[offset..])?;
                        if offset + offset_bytes > expected_end_offset {
                            return Err(Error::Parse(
                                "Immediate overrun for v128.store offset".into(),
                            ));
                        }
                        offset += offset_bytes;
                        // i += offset_bytes;
                        Instruction::V128Store(align, mem_offset)
                    }
                    0x0C => {
                        // v128.const
                        if offset + 16 > expected_end_offset {
                            return Err(Error::Parse("Immediate overrun for v128.const".into()));
                        }
                        let mut const_bytes = [0u8; 16];
                        const_bytes.copy_from_slice(&bytes[offset..offset + 16]);
                        offset += 16;
                        // i += 16; // Remove i increment
                        Instruction::V128Const(const_bytes)
                    }
                    0x0D => {
                        // v128.shuffle
                        if offset + 16 > expected_end_offset {
                            return Err(Error::Parse("Immediate overrun for v128.shuffle".into()));
                        }
                        let mut lane_bytes = [0u8; 16];
                        lane_bytes.copy_from_slice(&bytes[offset..offset + 16]);
                        offset += 16;
                        // i += 16; // Remove i increment
                        Instruction::V128Shuffle(lane_bytes)
                    }
                    0x0F => Instruction::V128SplatI8x16,
                    0x10 => Instruction::V128SplatI16x8,
                    0x11 => Instruction::V128SplatI32x4,
                    0x12 => Instruction::V128SplatI64x2,
                    0x13 => Instruction::F32x4Splat,
                    0x14 => Instruction::F64x2Splat,
                    0x7E => Instruction::I32x4ExtAddPairwiseI16x8S,
                    0x7F => Instruction::I32x4ExtAddPairwiseI16x8U,
                    0xAE => Instruction::SimdOpAE,
                    0xAF => {
                        return Err(Error::Parse(
                            "Unimplemented SIMD opcode 0xaf (i32x4.sub_sat_s)".into(),
                        ))
                    }
                    0xB0 => {
                        return Err(Error::Parse(
                            "Unimplemented SIMD opcode 0xb0 (i32x4.sub_sat_u)".into(),
                        ))
                    }
                    0xB1 => Instruction::SimdOpB1,
                    0xB2 => {
                        return Err(Error::Parse(
                            "Unimplemented SIMD opcode 0xb2 (i64x2.abs)".into(),
                        ))
                    }
                    0xB3 => {
                        return Err(Error::Parse(
                            "Unimplemented SIMD opcode 0xb3 (i64x2.neg)".into(),
                        ))
                    }
                    0xB4 => {
                        return Err(Error::Parse(
                            "Unimplemented SIMD opcode 0xb4 (i64x2.all_true)".into(),
                        ))
                    }
                    0xB5 => Instruction::SimdOpB5,
                    0xB6 => Instruction::I32x4DotI16x8S,
                    0xB7 => {
                        return Err(Error::Parse(
                            "Unimplemented SIMD opcode 0xb7 (i32x4.extmul_low_i16x8_s)".into(),
                        ))
                    }
                    0xB8 => {
                        return Err(Error::Parse(
                            "Unimplemented SIMD opcode 0xb8 (i32x4.extmul_high_i16x8_s)".into(),
                        ))
                    }
                    0xB9 => {
                        return Err(Error::Parse(
                            "Unimplemented SIMD opcode 0xb9 (i32x4.extmul_low_i16x8_u)".into(),
                        ))
                    }
                    _ => {
                        return Err(Error::InvalidModule(format!(
                            "Unknown SIMD opcode: 0xfd 0x{:02x}",
                            simd_opcode
                        )));
                    }
                }
            }
            _ => {
                return Err(Error::InvalidModule(format!(
                    "Unknown opcode: 0x{opcode:02x} at offset {offset}",
                    offset = function_body_start + (offset - function_body_start) // Calculate offset within body for error
                )));
            }
        };

        // Check if we read past the expected end BEFORE pushing
        if offset > expected_end_offset {
            return Err(Error::Parse(format!(
                 "Read past end of function body. Expected end: {}, Actual offset: {}. Last opcode: 0x{:02x}",
                 expected_end_offset, offset, opcode
             )));
        }

        expr.push(instruction);

        // Break loop specifically on End opcode AFTER processing it
        if opcode == 0x0B {
            break;
        }
    }

    // After loop, verify the offset matches exactly
    if offset != expected_end_offset {
        return Err(Error::Parse(format!(
            "Function body size mismatch. Expected end offset: {}, Actual end offset: {}. Declared size: {}, Locals size: {}",
            expected_end_offset,
            offset,
            size,
            function_body_start - size_bytes_read // Calculate actual locals size read
        )));
    }

    Ok((Code { size, locals, expr }, offset - initial_offset)) // Return total bytes read for this code entry
}

fn read_data(bytes: &[u8]) -> Result<(Data, usize)> {
    // TODO: Implement data reading
    Err(Error::InvalidData("Data reading not implemented".into()))
}
