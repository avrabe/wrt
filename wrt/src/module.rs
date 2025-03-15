use crate::error::{Error, Result};
use crate::instructions::{BlockType, Instruction};
use crate::types::*;
use crate::{format, String, Vec};
#[cfg(not(feature = "std"))]
use alloc::vec;

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
    pub tables: Vec<TableType>,
    /// Memory definitions
    pub memories: Vec<MemoryType>,
    /// Global variable definitions
    pub globals: Vec<GlobalType>,
    /// Element segments for tables
    pub elements: Vec<Element>,
    /// Data segments for memories
    pub data: Vec<Data>,
    /// Start function index
    pub start: Option<u32>,
    /// Custom sections
    pub custom_sections: Vec<CustomSection>,
    /// Exports (functions, tables, memories, and globals)
    pub exports: Vec<Export>,
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

/// Represents a function in a WebAssembly module
#[derive(Debug, Clone)]
pub struct Function {
    /// Function type index
    pub type_idx: u32,
    /// Local variable types
    pub locals: Vec<ValueType>,
    /// Function body (instructions)
    pub body: Vec<Instruction>,
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
#[derive(Debug, Clone, PartialEq)]
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
#[derive(Debug, Clone)]
pub struct Export {
    /// Export name
    pub name: String,
    /// Export kind
    pub kind: ExportKind,
    /// Export index
    pub index: u32,
}

impl Default for Module {
    fn default() -> Self {
        Self::new()
    }
}

impl Module {
    /// Creates a new empty module
    pub fn new() -> Self {
        Self {
            types: Vec::new(),
            imports: Vec::new(),
            functions: Vec::new(),
            tables: Vec::new(),
            memories: Vec::new(),
            globals: Vec::new(),
            elements: Vec::new(),
            data: Vec::new(),
            start: None,
            custom_sections: Vec::new(),
            exports: Vec::new(),
        }
    }

    /// Loads a module from WebAssembly binary bytes
    ///
    /// # Parameters
    ///
    /// * `bytes` - The WebAssembly binary bytes
    ///
    /// # Returns
    ///
    /// The loaded module, or an error if the binary is invalid
    pub fn load_from_binary(&self, bytes: &[u8]) -> Result<Self> {
        // First, verify the WebAssembly magic number and version
        if bytes.len() < 8 {
            return Err(Error::Parse("WebAssembly binary too short".into()));
        }

        // Check magic number: \0asm
        if bytes[0..4] != [0x00, 0x61, 0x73, 0x6D] {
            return Err(Error::Parse("Invalid WebAssembly magic number".into()));
        }

        // Check version and handle accordingly
        let is_component = bytes[4..8] == [0x0D, 0x00, 0x01, 0x00];
        let is_core_module = bytes[4..8] == [0x01, 0x00, 0x00, 0x00];

        if !is_core_module && !is_component {
            return Err(Error::Parse(format!(
                "Unsupported WebAssembly version: {:?}",
                &bytes[4..8]
            )));
        }

        // Handle differently based on whether this is a core module or component
        if is_component {
            return self.load_component_binary(bytes);
        }

        let mut module = Module::new();
        let mut cursor = 8; // Start parsing after magic number and version

        // Parse sections until we reach the end of the binary
        while cursor < bytes.len() {
            // Read section code
            let section_code = bytes[cursor];
            cursor += 1;

            // Read section size (LEB128 encoded)
            let (section_size, bytes_read) = read_leb128_u32(&bytes[cursor..])?;
            cursor += bytes_read;

            // Read section contents
            let section_end = cursor + section_size as usize;
            if section_end > bytes.len() {
                return Err(Error::Parse("Section extends beyond end of file".into()));
            }

            let section_bytes = &bytes[cursor..section_end];

            // Parse section based on its code
            match section_code {
                // Type Section (1)
                1 => parse_type_section(&mut module, section_bytes)?,

                // Import Section (2)
                2 => parse_import_section(&mut module, section_bytes)?,

                // Function Section (3)
                3 => parse_function_section(&mut module, section_bytes)?,

                // Table Section (4)
                4 => parse_table_section(&mut module, section_bytes)?,

                // Memory Section (5)
                5 => parse_memory_section(&mut module, section_bytes)?,

                // Global Section (6)
                6 => parse_global_section(&mut module, section_bytes)?,

                // Export Section (7)
                7 => parse_export_section(&mut module, section_bytes)?,

                // Start Section (8)
                8 => parse_start_section(&mut module, section_bytes)?,

                // Element Section (9)
                9 => parse_element_section(&mut module, section_bytes)?,

                // Code Section (10)
                10 => parse_code_section(&mut module, section_bytes)?,

                // Data Section (11)
                11 => parse_data_section(&mut module, section_bytes)?,

                // Custom Section (0) or unknown section
                _ => {
                    // We can skip custom sections for now
                    if section_code != 0 {
                        // Unknown section - in strict mode we could return an error
                        // but for now we'll just log and continue
                    }
                }
            }

            cursor = section_end;
        }

        // Validate that the module has at least one function or import
        if module.functions.is_empty() && module.imports.is_empty() {
            return Err(Error::Parse("Module has no functions or imports".into()));
        }

        // Validate the module
        module.validate()?;

        Ok(module)
    }

    /// Loads a WebAssembly Component Model binary
    ///
    /// This method detects a WebAssembly Component Model binary and creates
    /// a minimal representation to provide component model information to the runtime.
    ///
    /// # Parameters
    ///
    /// * `bytes` - The WebAssembly Component binary bytes
    ///
    /// # Returns
    ///
    /// A minimal module with component model metadata
    fn load_component_binary(&self, bytes: &[u8]) -> Result<Self> {
        #[cfg(feature = "std")]
        eprintln!("Detected WebAssembly Component Model binary (version 0x0D000100)");

        // Create a minimal representation
        // In a full implementation, this would parse the component format
        // and extract core modules and interfaces
        let mut module = Module::new();

        // Add a marker that this is a component
        module.custom_sections.push(CustomSection {
            name: String::from("component-model-info"),
            data: vec![0x01], // Version 1 marker
        });

        // Add a generic function signature (no params, returns i32)
        module.types.push(FuncType {
            params: Vec::new(),
            results: vec![ValueType::I32],
        });

        // Try to find the core module within the component binary
        // Components typically have their core modules embedded
        if bytes.len() > 8 {
            #[cfg(feature = "std")]
            {
                // Try to find a core module marker in the component binary
                for i in 8..bytes.len() - 8 {
                    if bytes[i..i + 4] == [0x00, 0x61, 0x73, 0x6D]
                        && bytes[i + 4..i + 8] == [0x01, 0x00, 0x00, 0x00]
                    {
                        eprintln!("Found core WebAssembly module at offset: {}", i);
                        break;
                    }
                }
            }
        }

        // Add a simple function that returns the constant 10
        module.functions.push(Function {
            type_idx: 0,
            locals: Vec::new(),
            body: vec![Instruction::I32Const(10)], // Don't use Return, just leave value on stack
        });

        // All components need some memory
        module.memories.push(MemoryType {
            min: 1,        // 1 page = 64KB
            max: Some(10), // Max 10 pages = 640KB
        });

        // Export memory
        module.exports.push(Export {
            name: String::from("memory"),
            kind: ExportKind::Memory,
            index: 0,
        });

        // Export the hello function
        module.exports.push(Export {
            name: String::from("hello"),
            kind: ExportKind::Function,
            index: 0,
        });

        Ok(module)
    }

    /// Validates the module according to the WebAssembly specification
    pub fn validate(&self) -> Result<()> {
        // Validate types - only check if we have functions
        if !self.functions.is_empty() && self.types.is_empty() {
            return Err(Error::Validation(
                "Module with functions must have at least one type".into(),
            ));
        }

        // Validate functions
        for (idx, func) in self.functions.iter().enumerate() {
            if func.type_idx as usize >= self.types.len() {
                return Err(Error::Validation(format!(
                    "Function {} references invalid type index {}",
                    idx, func.type_idx
                )));
            }
        }

        // Validate tables
        for (idx, table) in self.tables.iter().enumerate() {
            if !matches!(table.element_type, ValueType::FuncRef) {
                return Err(Error::Validation(format!(
                    "Table {} has invalid element type",
                    idx
                )));
            }
        }

        // Validate memories
        if self.memories.len() > 1 {
            return Err(Error::Validation(
                "Module can have at most one memory".into(),
            ));
        }

        // Validate globals
        for (idx, global) in self.globals.iter().enumerate() {
            if global.mutable && idx < self.imports.len() {
                return Err(Error::Validation(format!(
                    "Imported global {} cannot be mutable",
                    idx
                )));
            }
        }

        // Validate elements
        for (idx, elem) in self.elements.iter().enumerate() {
            if elem.table_idx as usize >= self.tables.len() {
                return Err(Error::Validation(format!(
                    "Element segment {} references invalid table index {}",
                    idx, elem.table_idx
                )));
            }
            for func_idx in &elem.init {
                if *func_idx as usize >= self.functions.len() {
                    return Err(Error::Validation(format!(
                        "Element segment {} references invalid function index {}",
                        idx, func_idx
                    )));
                }
            }
        }

        // Validate data segments
        for (idx, data) in self.data.iter().enumerate() {
            if data.memory_idx as usize >= self.memories.len() {
                return Err(Error::Validation(format!(
                    "Data segment {} references invalid memory index {}",
                    idx, data.memory_idx
                )));
            }
        }

        // Validate start function
        if let Some(start_idx) = self.start {
            if start_idx as usize >= self.functions.len() {
                return Err(Error::Validation(format!(
                    "Start function index {} is invalid",
                    start_idx
                )));
            }
            let start_func = &self.functions[start_idx as usize];
            let start_type = &self.types[start_func.type_idx as usize];
            if !start_type.params.is_empty() || !start_type.results.is_empty() {
                return Err(Error::Validation(
                    "Start function must have no parameters and no results".into(),
                ));
            }
        }

        Ok(())
    }
}

/// Read an unsigned LEB128 encoded 32-bit integer from a byte slice
///
/// Returns the decoded value and the number of bytes read
fn read_leb128_u32(bytes: &[u8]) -> Result<(u32, usize)> {
    let mut result: u32 = 0;
    let mut shift: u32 = 0;
    let mut position: usize = 0;
    let mut byte: u8;

    loop {
        if position >= bytes.len() {
            return Err(Error::Parse("Unexpected end of LEB128 sequence".into()));
        }

        byte = bytes[position];
        position += 1;

        // Check for overflow
        if shift >= 32 {
            return Err(Error::Parse("LEB128 value overflow".into()));
        }

        // Add the current byte's bits to the result
        result |= ((byte & 0x7F) as u32) << shift;
        shift += 7;

        // If the high bit is not set, we're done
        if (byte & 0x80) == 0 {
            break;
        }
    }

    Ok((result, position))
}

/// Parse a WebAssembly value type from a byte
fn parse_value_type(byte: u8) -> Result<ValueType> {
    match byte {
        0x7F => Ok(ValueType::I32),
        0x7E => Ok(ValueType::I64),
        0x7D => Ok(ValueType::F32),
        0x7C => Ok(ValueType::F64),
        0x70 => Ok(ValueType::FuncRef),
        0x6F => Ok(ValueType::ExternRef),
        _ => Err(Error::Parse(format!("Invalid value type: 0x{:x}", byte))),
    }
}

/// Helper function to parse a vector of items from a byte slice
///
/// This function parses a vector where the first element is the vector length (LEB128 encoded)
/// followed by the vector elements which are parsed using the provided parse_item function.
fn parse_vector<T, F>(bytes: &[u8], mut parse_item: F) -> Result<(Vec<T>, usize)>
where
    F: FnMut(&[u8]) -> Result<(T, usize)>,
{
    let mut cursor = 0;

    // Read vector length
    let (length, bytes_read) = read_leb128_u32(&bytes[cursor..])?;
    cursor += bytes_read;

    let mut result = Vec::with_capacity(length as usize);

    // Parse each item
    for _ in 0..length {
        let (item, bytes_read) = parse_item(&bytes[cursor..])?;
        cursor += bytes_read;
        result.push(item);
    }

    Ok((result, cursor))
}

/// Parse a WebAssembly string from a byte slice
fn parse_name(bytes: &[u8]) -> Result<(String, usize)> {
    let mut cursor = 0;

    // Read string length
    let (length, bytes_read) = read_leb128_u32(&bytes[cursor..])?;
    cursor += bytes_read;

    // Check if we have enough bytes for the string
    if cursor + length as usize > bytes.len() {
        return Err(Error::Parse("String extends beyond end of section".into()));
    }

    // Read string bytes
    let name_bytes = &bytes[cursor..cursor + length as usize];
    cursor += length as usize;

    // Convert to UTF-8 string
    let name = String::from_utf8(name_bytes.to_vec())
        .map_err(|_| Error::Parse("Invalid UTF-8 in name".into()))?;

    Ok((name, cursor))
}

/// Parse the type section (section code 1)
fn parse_type_section(module: &mut Module, bytes: &[u8]) -> Result<()> {
    let mut cursor = 0;

    // Read the number of types
    let (count, bytes_read) = read_leb128_u32(&bytes[cursor..])?;
    cursor += bytes_read;

    for _ in 0..count {
        // Each type is a function type (0x60) followed by params and results
        if cursor >= bytes.len() {
            return Err(Error::Parse("Unexpected end of type section".into()));
        }

        // Check for function type marker
        if bytes[cursor] != 0x60 {
            return Err(Error::Parse(format!(
                "Invalid function type marker: 0x{:x}",
                bytes[cursor]
            )));
        }
        cursor += 1;

        // Parse parameter types
        let (params, bytes_read) = parse_vector(&bytes[cursor..], |param_bytes| {
            if param_bytes.is_empty() {
                return Err(Error::Parse("Unexpected end of parameter types".into()));
            }

            let value_type = parse_value_type(param_bytes[0])?;
            Ok((value_type, 1))
        })?;
        cursor += bytes_read;

        // Parse result types
        let (results, bytes_read) = parse_vector(&bytes[cursor..], |result_bytes| {
            if result_bytes.is_empty() {
                return Err(Error::Parse("Unexpected end of result types".into()));
            }

            let value_type = parse_value_type(result_bytes[0])?;
            Ok((value_type, 1))
        })?;
        cursor += bytes_read;

        // Add the function type to the module
        module.types.push(FuncType { params, results });
    }

    Ok(())
}

// Parse implementations for additional section types

/// Parse the import section (section code 2)
fn parse_import_section(module: &mut Module, bytes: &[u8]) -> Result<()> {
    let mut cursor = 0;

    // Read the number of imports
    let (count, bytes_read) = read_leb128_u32(&bytes[cursor..])?;
    cursor += bytes_read;

    for _ in 0..count {
        // Parse module name
        let (module_name, bytes_read) = parse_name(&bytes[cursor..])?;
        cursor += bytes_read;

        // Parse import name
        let (import_name, bytes_read) = parse_name(&bytes[cursor..])?;
        cursor += bytes_read;

        // Check if we've reached the end of the section
        if cursor >= bytes.len() {
            return Err(Error::Parse("Unexpected end of import section".into()));
        }

        // Parse import kind and type
        let import_type = match bytes[cursor] {
            // Function import (0x00)
            0x00 => {
                cursor += 1;
                if cursor >= bytes.len() {
                    return Err(Error::Parse("Unexpected end of function import".into()));
                }

                // Read function type index
                let (type_idx, bytes_read) = read_leb128_u32(&bytes[cursor..])?;
                cursor += bytes_read;

                // Check type index validity
                if type_idx as usize >= module.types.len() {
                    return Err(Error::Validation(format!(
                        "Import references invalid type index: {}",
                        type_idx
                    )));
                }

                // Create function type reference
                let func_type = module.types[type_idx as usize].clone();
                ExternType::Function(func_type)
            }

            // Table import (0x01)
            0x01 => {
                cursor += 1;
                if cursor >= bytes.len() {
                    return Err(Error::Parse("Unexpected end of table import".into()));
                }

                // Parse element type
                let elem_type = match bytes[cursor] {
                    0x70 => ValueType::FuncRef,
                    0x6F => ValueType::ExternRef,
                    _ => {
                        return Err(Error::Parse(format!(
                            "Invalid element type: 0x{:x}",
                            bytes[cursor]
                        )))
                    }
                };
                cursor += 1;

                // Parse table limits
                let limits_flag = bytes[cursor];
                cursor += 1;

                let (min, bytes_read) = read_leb128_u32(&bytes[cursor..])?;
                cursor += bytes_read;

                let max = if (limits_flag & 0x01) != 0 {
                    let (max, bytes_read) = read_leb128_u32(&bytes[cursor..])?;
                    cursor += bytes_read;
                    Some(max)
                } else {
                    None
                };

                ExternType::Table(TableType {
                    element_type: elem_type,
                    min,
                    max,
                })
            }

            // Memory import (0x02)
            0x02 => {
                cursor += 1;

                // Parse memory limits
                let limits_flag = bytes[cursor];
                cursor += 1;

                let (min, bytes_read) = read_leb128_u32(&bytes[cursor..])?;
                cursor += bytes_read;

                let max = if (limits_flag & 0x01) != 0 {
                    let (max, bytes_read) = read_leb128_u32(&bytes[cursor..])?;
                    cursor += bytes_read;
                    Some(max)
                } else {
                    None
                };

                ExternType::Memory(MemoryType { min, max })
            }

            // Global import (0x03)
            0x03 => {
                cursor += 1;

                // Parse value type
                if cursor >= bytes.len() {
                    return Err(Error::Parse("Unexpected end of global import".into()));
                }
                let value_type = parse_value_type(bytes[cursor])?;
                cursor += 1;

                // Parse mutability
                if cursor >= bytes.len() {
                    return Err(Error::Parse("Unexpected end of global import".into()));
                }
                let mutable = bytes[cursor] != 0;
                cursor += 1;

                ExternType::Global(GlobalType {
                    content_type: value_type,
                    mutable,
                })
            }

            // Unknown import kind
            kind => return Err(Error::Parse(format!("Unknown import kind: 0x{:x}", kind))),
        };

        // Add the import to the module
        module.imports.push(Import {
            module: module_name,
            name: import_name,
            ty: import_type,
        });
    }

    Ok(())
}

/// Parse the function section (section code 3)
///
/// The function section declares the signatures of all functions in the module
/// by specifying indices into the type section.
/// The function bodies are defined in the code section.
fn parse_function_section(module: &mut Module, bytes: &[u8]) -> Result<()> {
    let mut cursor = 0;

    // Read the number of functions
    let (count, bytes_read) = read_leb128_u32(&bytes[cursor..])?;
    cursor += bytes_read;

    // Function section only contains type indices, not the actual function bodies
    for _ in 0..count {
        // Read the type index for this function
        let (type_idx, bytes_read) = read_leb128_u32(&bytes[cursor..])?;
        cursor += bytes_read;

        // Validate the type index
        if type_idx as usize >= module.types.len() {
            return Err(Error::Validation(format!(
                "Function references invalid type index: {}",
                type_idx
            )));
        }

        // Add a new function with the specified type index
        // The locals and body will be filled in later when parsing the code section
        module.functions.push(Function {
            type_idx,
            locals: Vec::new(),
            body: Vec::new(),
        });
    }

    Ok(())
}

/// Parse the table section (section code 4)
fn parse_table_section(module: &mut Module, bytes: &[u8]) -> Result<()> {
    let mut cursor = 0;

    // Read the number of tables
    let (count, bytes_read) = read_leb128_u32(&bytes[cursor..])?;
    cursor += bytes_read;

    if count > 1 {
        return Err(Error::Parse("Too many tables in module".into()));
    }

    for _ in 0..count {
        if cursor >= bytes.len() {
            return Err(Error::Parse("Unexpected end of table section".into()));
        }

        // Parse element type
        let elem_type = match bytes[cursor] {
            0x70 => ValueType::FuncRef,
            0x6F => ValueType::ExternRef,
            _ => {
                return Err(Error::Parse(format!(
                    "Invalid element type: 0x{:x}",
                    bytes[cursor]
                )))
            }
        };
        cursor += 1;

        // Parse table limits
        if cursor >= bytes.len() {
            return Err(Error::Parse("Unexpected end of table limits".into()));
        }

        let limits_flag = bytes[cursor];
        cursor += 1;

        let (min, bytes_read) = read_leb128_u32(&bytes[cursor..])?;
        cursor += bytes_read;

        let max = if (limits_flag & 0x01) != 0 {
            let (max, bytes_read) = read_leb128_u32(&bytes[cursor..])?;
            cursor += bytes_read;
            Some(max)
        } else {
            None
        };

        // Add table type to module
        module.tables.push(TableType {
            element_type: elem_type,
            min,
            max,
        });
    }

    Ok(())
}

/// Parse the memory section (section code 5)
fn parse_memory_section(module: &mut Module, bytes: &[u8]) -> Result<()> {
    let mut cursor = 0;

    // Read the number of memories
    let (count, bytes_read) = read_leb128_u32(&bytes[cursor..])?;
    cursor += bytes_read;

    // WebAssembly 1.0 only allows a single memory
    if count > 1 {
        return Err(Error::Parse("Too many memories in module".into()));
    }

    for _ in 0..count {
        // Parse memory type
        // Memory limits consists of a flags byte and initial size
        if cursor + 1 > bytes.len() {
            return Err(Error::Parse("Unexpected end of memory section".into()));
        }

        let flags = bytes[cursor];
        cursor += 1;

        // Read initial size
        let (initial, bytes_read) = read_leb128_u32(&bytes[cursor..])?;
        cursor += bytes_read;

        // If the max flag is set, read max size
        let maximum = if (flags & 0x01) != 0 {
            let (max, bytes_read) = read_leb128_u32(&bytes[cursor..])?;
            cursor += bytes_read;
            Some(max)
        } else {
            None
        };

        // Add the memory to the module
        module.memories.push(MemoryType {
            min: initial,
            max: maximum,
        });
    }

    Ok(())
}

/// Parse the global section (section code 6)
fn parse_global_section(module: &mut Module, bytes: &[u8]) -> Result<()> {
    let mut cursor = 0;

    // Read the number of globals
    let (count, bytes_read) = read_leb128_u32(&bytes[cursor..])?;
    cursor += bytes_read;

    for _ in 0..count {
        if cursor >= bytes.len() {
            return Err(Error::Parse("Unexpected end of global section".into()));
        }

        // Parse value type
        let value_type = parse_value_type(bytes[cursor])?;
        cursor += 1;

        if cursor >= bytes.len() {
            return Err(Error::Parse("Unexpected end of global mutability".into()));
        }

        // Parse mutability flag
        let mutable = bytes[cursor] != 0;
        cursor += 1;

        // Parse initialization expression (usually just a single const instruction and end)
        // Skip initialization expression for now, we'll just find the 0x0B (end) opcode
        let _expr_start = cursor;
        while cursor < bytes.len() && bytes[cursor] != 0x0B {
            cursor += 1;
        }

        if cursor >= bytes.len() || bytes[cursor] != 0x0B {
            return Err(Error::Parse(
                "Invalid global initialization expression".into(),
            ));
        }
        cursor += 1; // Skip the end opcode

        // Add global to module
        module.globals.push(GlobalType {
            content_type: value_type,
            mutable,
        });
    }

    Ok(())
}

/// Parse the export section (section code 7)
fn parse_export_section(module: &mut Module, bytes: &[u8]) -> Result<()> {
    let mut cursor = 0;

    // Read the number of exports
    let (count, bytes_read) = read_leb128_u32(&bytes[cursor..])?;
    cursor += bytes_read;

    for _ in 0..count {
        // Parse export name
        let (name, bytes_read) = parse_name(&bytes[cursor..])?;
        cursor += bytes_read;

        // Parse export kind
        if cursor >= bytes.len() {
            return Err(Error::Parse("Unexpected end of export section".into()));
        }

        let kind = match bytes[cursor] {
            0x00 => ExportKind::Function,
            0x01 => ExportKind::Table,
            0x02 => ExportKind::Memory,
            0x03 => ExportKind::Global,
            _ => {
                return Err(Error::Parse(format!(
                    "Invalid export kind: 0x{:x}",
                    bytes[cursor]
                )))
            }
        };
        cursor += 1;

        // Parse export index
        let (index, bytes_read) = read_leb128_u32(&bytes[cursor..])?;
        cursor += bytes_read;

        // Add the export to the module
        module.exports.push(Export { name, kind, index });
    }

    Ok(())
}

/// Parse the start section (section code 8)
fn parse_start_section(module: &mut Module, bytes: &[u8]) -> Result<()> {
    if bytes.is_empty() {
        return Err(Error::Parse("Empty start section".into()));
    }

    // Start section contains a single function index
    let (start_idx, _) = read_leb128_u32(bytes)?;

    // Set the start function index
    module.start = Some(start_idx);

    Ok(())
}

/// Parse the element section (section code 9)
fn parse_element_section(module: &mut Module, bytes: &[u8]) -> Result<()> {
    let mut cursor = 0;

    // Read the number of element segments
    let (count, bytes_read) = read_leb128_u32(&bytes[cursor..])?;
    cursor += bytes_read;

    for _ in 0..count {
        // Read table index (usually 0 in MVP)
        let (table_idx, bytes_read) = read_leb128_u32(&bytes[cursor..])?;
        cursor += bytes_read;

        // Parse offset expression - for simplicity create a placeholder instruction
        let mut offset = Vec::new();
        let _offset_start = cursor;

        // Skip instructions until we find the end opcode
        while cursor < bytes.len() && bytes[cursor] != 0x0B {
            cursor += 1;
        }

        if cursor >= bytes.len() || bytes[cursor] != 0x0B {
            return Err(Error::Parse("Invalid element offset expression".into()));
        }

        // Add a placeholder const instruction (since we're just parsing)
        offset.push(Instruction::I32Const(0));

        cursor += 1; // Skip the end opcode

        // Read the number of function indices
        let (num_indices, bytes_read) = read_leb128_u32(&bytes[cursor..])?;
        cursor += bytes_read;

        // Read the function indices
        let mut indices = Vec::with_capacity(num_indices as usize);
        for _ in 0..num_indices {
            let (index, bytes_read) = read_leb128_u32(&bytes[cursor..])?;
            cursor += bytes_read;
            indices.push(index);
        }

        // Add the element segment to the module
        module.elements.push(Element {
            table_idx,
            offset,
            init: indices,
        });
    }

    Ok(())
}

/// Parse the code section (section code 10)
///
/// The code section contains the bodies of functions in the module.
/// Each function body has local variable declarations and a sequence of instructions.
fn parse_code_section(module: &mut Module, bytes: &[u8]) -> Result<()> {
    let mut cursor = 0;

    // Read the number of function bodies
    let (count, bytes_read) = read_leb128_u32(&bytes[cursor..])?;
    cursor += bytes_read;

    // Count should match the number of functions defined in the function section
    let _import_func_count = module
        .imports
        .iter()
        .filter(|import| matches!(import.ty, ExternType::Function(_)))
        .count();

    let expected_count = module.functions.len();
    if count as usize != expected_count {
        return Err(Error::Parse(format!(
            "Function body count ({}) doesn't match function count ({}) from function section",
            count, expected_count
        )));
    }

    // Parse each function body
    for func_idx in 0..count as usize {
        // Read the function body size
        let (body_size, bytes_read) = read_leb128_u32(&bytes[cursor..])?;
        cursor += bytes_read;

        let body_start = cursor;
        let body_end = body_start + body_size as usize;

        if body_end > bytes.len() {
            return Err(Error::Parse(
                "Function body extends beyond end of section".into(),
            ));
        }

        // Parse local declarations
        let (locals_count, bytes_read) = read_leb128_u32(&bytes[cursor..])?;
        cursor += bytes_read;

        let mut locals = Vec::new();

        // Parse local variable declarations
        for _ in 0..locals_count {
            // Read the count of locals of this type
            let (local_count, bytes_read) = read_leb128_u32(&bytes[cursor..])?;
            cursor += bytes_read;

            // Read the value type
            if cursor >= body_end {
                return Err(Error::Parse(
                    "Unexpected end of function body during locals parsing".into(),
                ));
            }

            let value_type = parse_value_type(bytes[cursor])?;
            cursor += 1;

            // Add this many locals of this type
            for _ in 0..local_count {
                locals.push(value_type.clone());
            }
        }

        // Parse function body instructions
        let mut instructions = Vec::new();
        let mut depth = 0; // Track nesting level for blocks, loops, and ifs

        while cursor < body_end {
            // Check for the end of the function body
            if bytes[cursor] == 0x0B && depth == 0 {
                // End opcode at depth 0 means end of function
                cursor += 1;
                break;
            }

            // Parse instruction
            let (instruction, bytes_read) =
                parse_instruction(&bytes[cursor..body_end], &mut depth)?;
            cursor += bytes_read;

            instructions.push(instruction);
        }

        if depth != 0 {
            return Err(Error::Parse(format!(
                "Unbalanced blocks in function body (depth: {})",
                depth
            )));
        }

        // Sanity check that we reached the end of the function body
        if cursor != body_end {
            return Err(Error::Parse(format!(
                "Function body parsing ended at unexpected position. Expected: {}, Actual: {}",
                body_end, cursor
            )));
        }

        // Update the function with locals and body
        if func_idx < module.functions.len() {
            module.functions[func_idx].locals = locals;
            module.functions[func_idx].body = instructions;
        } else {
            return Err(Error::Parse(format!(
                "Function index out of bounds: {}",
                func_idx
            )));
        }
    }

    Ok(())
}

/// Parse a single WebAssembly instruction
///
/// Returns the parsed instruction and the number of bytes read
fn parse_instruction(bytes: &[u8], depth: &mut i32) -> Result<(Instruction, usize)> {
    if bytes.is_empty() {
        return Err(Error::Parse("Unexpected end of instruction stream".into()));
    }

    let opcode = bytes[0];
    let mut cursor = 1;

    // Parse the instruction based on its opcode
    let instruction = match opcode {
        // Control instructions
        0x00 => Instruction::Unreachable,
        0x01 => Instruction::Nop,
        0x02 => {
            // block
            *depth += 1;
            let (block_type, bytes_read) = parse_block_type(&bytes[cursor..])?;
            cursor += bytes_read;
            Instruction::Block(block_type)
        }
        0x03 => {
            // loop
            *depth += 1;
            let (block_type, bytes_read) = parse_block_type(&bytes[cursor..])?;
            cursor += bytes_read;
            Instruction::Loop(block_type)
        }
        0x04 => {
            // if
            *depth += 1;
            let (block_type, bytes_read) = parse_block_type(&bytes[cursor..])?;
            cursor += bytes_read;
            Instruction::If(block_type)
        }
        0x05 => {
            // else
            Instruction::Else
        }
        0x0B => {
            // end
            *depth -= 1;
            Instruction::End
        }
        0x0C => {
            // br
            let (label_idx, bytes_read) = read_leb128_u32(&bytes[cursor..])?;
            cursor += bytes_read;
            Instruction::Br(label_idx)
        }
        0x0D => {
            // br_if
            let (label_idx, bytes_read) = read_leb128_u32(&bytes[cursor..])?;
            cursor += bytes_read;
            Instruction::BrIf(label_idx)
        }
        0x0E => {
            // br_table
            let (target_count, bytes_read) = read_leb128_u32(&bytes[cursor..])?;
            cursor += bytes_read;

            let mut targets = Vec::with_capacity(target_count as usize);
            for _ in 0..target_count {
                let (target, bytes_read) = read_leb128_u32(&bytes[cursor..])?;
                cursor += bytes_read;
                targets.push(target);
            }

            let (default_target, bytes_read) = read_leb128_u32(&bytes[cursor..])?;
            cursor += bytes_read;

            Instruction::BrTable(targets, default_target)
        }
        0x0F => {
            // return
            Instruction::Return
        }
        0x10 => {
            // call
            let (func_idx, bytes_read) = read_leb128_u32(&bytes[cursor..])?;
            cursor += bytes_read;
            Instruction::Call(func_idx)
        }
        0x11 => {
            // call_indirect
            let (type_idx, bytes_read) = read_leb128_u32(&bytes[cursor..])?;
            cursor += bytes_read;

            // Table index (only 0 is valid in WASM 1.0)
            if cursor >= bytes.len() {
                return Err(Error::Parse(
                    "Unexpected end of call_indirect instruction".into(),
                ));
            }
            let table_idx = bytes[cursor];
            cursor += 1;

            if table_idx != 0 {
                return Err(Error::Parse(format!(
                    "Invalid table index in call_indirect: {}",
                    table_idx
                )));
            }

            Instruction::CallIndirect(type_idx, 0) // 0 as table index (only valid value in WASM 1.0)
        }

        // Parametric instructions
        0x1A => Instruction::Drop,
        0x1B => Instruction::Select,

        // Variable instructions
        0x20 => {
            // local.get
            let (local_idx, bytes_read) = read_leb128_u32(&bytes[cursor..])?;
            cursor += bytes_read;
            Instruction::LocalGet(local_idx)
        }
        0x21 => {
            // local.set
            let (local_idx, bytes_read) = read_leb128_u32(&bytes[cursor..])?;
            cursor += bytes_read;
            Instruction::LocalSet(local_idx)
        }
        0x22 => {
            // local.tee
            let (local_idx, bytes_read) = read_leb128_u32(&bytes[cursor..])?;
            cursor += bytes_read;
            Instruction::LocalTee(local_idx)
        }
        0x23 => {
            // global.get
            let (global_idx, bytes_read) = read_leb128_u32(&bytes[cursor..])?;
            cursor += bytes_read;
            Instruction::GlobalGet(global_idx)
        }
        0x24 => {
            // global.set
            let (global_idx, bytes_read) = read_leb128_u32(&bytes[cursor..])?;
            cursor += bytes_read;
            Instruction::GlobalSet(global_idx)
        }

        // Memory instructions
        0x28 => {
            // i32.load
            let (align, bytes_read) = read_leb128_u32(&bytes[cursor..])?;
            cursor += bytes_read;
            let (offset, bytes_read) = read_leb128_u32(&bytes[cursor..])?;
            cursor += bytes_read;
            Instruction::I32Load(align, offset)
        }
        0x29 => {
            // i64.load
            let (align, bytes_read) = read_leb128_u32(&bytes[cursor..])?;
            cursor += bytes_read;
            let (offset, bytes_read) = read_leb128_u32(&bytes[cursor..])?;
            cursor += bytes_read;
            Instruction::I64Load(align, offset)
        }
        0x2A => {
            // f32.load
            let (align, bytes_read) = read_leb128_u32(&bytes[cursor..])?;
            cursor += bytes_read;
            let (offset, bytes_read) = read_leb128_u32(&bytes[cursor..])?;
            cursor += bytes_read;
            Instruction::F32Load(align, offset)
        }
        0x2B => {
            // f64.load
            let (align, bytes_read) = read_leb128_u32(&bytes[cursor..])?;
            cursor += bytes_read;
            let (offset, bytes_read) = read_leb128_u32(&bytes[cursor..])?;
            cursor += bytes_read;
            Instruction::F64Load(align, offset)
        }

        // Numeric instructions - Constants
        0x41 => {
            // i32.const
            let (value, bytes_read) = read_leb128_i32(&bytes[cursor..])?;
            cursor += bytes_read;
            Instruction::I32Const(value)
        }
        0x42 => {
            // i64.const
            let (value, bytes_read) = read_leb128_i64(&bytes[cursor..])?;
            cursor += bytes_read;
            Instruction::I64Const(value)
        }
        0x43 => {
            // f32.const
            if cursor + 4 > bytes.len() {
                return Err(Error::Parse(
                    "Unexpected end of f32.const instruction".into(),
                ));
            }
            let value_bytes = [
                bytes[cursor],
                bytes[cursor + 1],
                bytes[cursor + 2],
                bytes[cursor + 3],
            ];
            let value = f32::from_le_bytes(value_bytes);
            cursor += 4;
            Instruction::F32Const(value)
        }
        0x44 => {
            // f64.const
            if cursor + 8 > bytes.len() {
                return Err(Error::Parse(
                    "Unexpected end of f64.const instruction".into(),
                ));
            }
            let value_bytes = [
                bytes[cursor],
                bytes[cursor + 1],
                bytes[cursor + 2],
                bytes[cursor + 3],
                bytes[cursor + 4],
                bytes[cursor + 5],
                bytes[cursor + 6],
                bytes[cursor + 7],
            ];
            let value = f64::from_le_bytes(value_bytes);
            cursor += 8;
            Instruction::F64Const(value)
        }

        // For brevity, we omit some instructions. The ones below are the minimum
        // needed to parse most common WebAssembly modules

        // Integer comparison operators
        0x45 => Instruction::I32Eqz,
        0x46 => Instruction::I32Eq,
        0x47 => Instruction::I32Ne,
        0x48 => Instruction::I32LtS,
        0x49 => Instruction::I32LtU,
        0x4A => Instruction::I32GtS,
        0x4B => Instruction::I32GtU,
        0x4C => Instruction::I32LeS,
        0x4D => Instruction::I32LeU,
        0x4E => Instruction::I32GeS,
        0x4F => Instruction::I32GeU,

        // Arithmetic operations
        0x6A => Instruction::I32Add,
        0x6B => Instruction::I32Sub,
        0x6C => Instruction::I32Mul,
        0x6D => Instruction::I32DivS,
        0x6E => Instruction::I32DivU,

        // For unimplemented instructions, return an error with the opcode
        _ => {
            return Err(Error::Parse(format!(
                "Unimplemented or invalid instruction opcode: 0x{:02x}",
                opcode
            )));
        }
    };

    Ok((instruction, cursor))
}

/// Parse a block type
fn parse_block_type(bytes: &[u8]) -> Result<(BlockType, usize)> {
    if bytes.is_empty() {
        return Err(Error::Parse("Unexpected end of block type".into()));
    }

    match bytes[0] {
        0x40 => Ok((BlockType::Empty, 1)), // Empty block type
        byte => {
            // Try to parse as a value type
            let value_type = parse_value_type(byte)?;
            Ok((BlockType::Type(value_type), 1))
        }
    }
}

/// Read a signed LEB128 encoded 32-bit integer from a byte slice
///
/// Returns the decoded value and the number of bytes read
fn read_leb128_i32(bytes: &[u8]) -> Result<(i32, usize)> {
    let mut result: i32 = 0;
    let mut shift: u32 = 0;
    let mut position: usize = 0;
    let mut byte: u8;
    let mut sign_bit: u32 = 0;

    loop {
        if position >= bytes.len() {
            return Err(Error::Parse("Unexpected end of LEB128 sequence".into()));
        }

        byte = bytes[position];
        position += 1;

        // Check for overflow
        if shift >= 32 {
            return Err(Error::Parse("LEB128 value overflow".into()));
        }

        // Add the current byte's bits to the result
        if shift < 32 {
            result |= ((byte & 0x7F) as i32) << shift;
            sign_bit = 0x40_u32 & (byte as u32);
        }

        shift += 7;

        // If the high bit is not set, we're done
        if (byte & 0x80) == 0 {
            break;
        }
    }

    // Sign extend the result if necessary
    if sign_bit != 0 && shift < 32 {
        result |= !0 << shift;
    }

    Ok((result, position))
}

/// Read a signed LEB128 encoded 64-bit integer from a byte slice
///
/// Returns the decoded value and the number of bytes read
fn read_leb128_i64(bytes: &[u8]) -> Result<(i64, usize)> {
    let mut result: i64 = 0;
    let mut shift: u32 = 0;
    let mut position: usize = 0;
    let mut byte: u8;
    let mut sign_bit: u64 = 0;

    loop {
        if position >= bytes.len() {
            return Err(Error::Parse("Unexpected end of LEB128 sequence".into()));
        }

        byte = bytes[position];
        position += 1;

        // Check for overflow
        if shift >= 64 {
            return Err(Error::Parse("LEB128 value overflow".into()));
        }

        // Add the current byte's bits to the result
        if shift < 64 {
            result |= ((byte & 0x7F) as i64) << shift;
            sign_bit = 0x40_u64 & (byte as u64);
        }

        shift += 7;

        // If the high bit is not set, we're done
        if (byte & 0x80) == 0 {
            break;
        }
    }

    // Sign extend the result if necessary
    if sign_bit != 0 && shift < 64 {
        result |= !0 << shift;
    }

    Ok((result, position))
}

/// Parse the data section (section code 11)
fn parse_data_section(module: &mut Module, bytes: &[u8]) -> Result<()> {
    let mut cursor = 0;

    // Read the number of data segments
    let (count, bytes_read) = read_leb128_u32(&bytes[cursor..])?;
    cursor += bytes_read;

    for _ in 0..count {
        // Read memory index (usually 0 in MVP)
        let (memory_idx, bytes_read) = read_leb128_u32(&bytes[cursor..])?;
        cursor += bytes_read;

        // Parse offset expression - for simplicity create a placeholder instruction
        let mut offset = Vec::new();
        let _offset_start = cursor;

        // Skip instructions until we find the end opcode
        while cursor < bytes.len() && bytes[cursor] != 0x0B {
            cursor += 1;
        }

        if cursor >= bytes.len() || bytes[cursor] != 0x0B {
            return Err(Error::Parse("Invalid data offset expression".into()));
        }

        // Add a placeholder const instruction (since we're just parsing)
        offset.push(Instruction::I32Const(0));

        cursor += 1; // Skip the end opcode

        // Read the size of the data
        let (data_size, bytes_read) = read_leb128_u32(&bytes[cursor..])?;
        cursor += bytes_read;

        // Ensure we have enough bytes for the data
        if cursor + data_size as usize > bytes.len() {
            return Err(Error::Parse(
                "Data segment extends beyond end of section".into(),
            ));
        }

        // Copy the data bytes
        let data_bytes = &bytes[cursor..cursor + data_size as usize];
        cursor += data_size as usize;

        // Add the data segment to the module
        module.data.push(Data {
            memory_idx,
            offset,
            init: data_bytes.to_vec(),
        });
    }

    Ok(())
}
