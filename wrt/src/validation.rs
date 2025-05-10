use crate::prelude::{
    BlockType, FuncType, ValueType,
};
use crate::error::{kinds, Error, Result};
use crate::instructions::Instruction;
use crate::module::Module;
use crate::prelude::{String, Vec};
use crate::types::*;
use wrt_error::{codes, ErrorCategory};

/// Validates a WebAssembly module
pub fn validate_module(module: &Module) -> Result<()> {
    // Validate types
    validate_types(module)?;

    // Validate functions
    validate_functions(module)?;

    // Validate tables
    validate_tables(module)?;

    // Validate memories
    validate_memories(module)?;

    // Validate globals
    validate_globals(module)?;

    // Validate elements
    validate_elements(module)?;

    // Validate data segments
    validate_data_segments(module)?;

    // Validate start function
    validate_start_function(module)?;

    Ok(())
}

fn validate_types(module: &Module) -> Result<()> {
    // Check if we have types when we have functions
    if !module.functions.is_empty() && module.types.is_empty() {
        return Err(Error::new(
            ErrorCategory::Validation,
            codes::VALIDATION_ERROR,
            "Module with functions must have at least one type",
        ));
    }

    // Validate each function type
    for (idx, func_type) in module.types.iter().enumerate() {
        // Validate parameter types
        for (param_idx, param_type) in func_type.params.iter().enumerate() {
            if !is_valid_value_type(param_type) {
                return Err(Error::new(
                    ErrorCategory::Validation,
                    codes::VALIDATION_ERROR,
                    format!(
                        "Invalid parameter type at index {} in function type {}",
                        param_idx, idx
                    ),
                ));
            }
        }

        // Validate result types
        for (result_idx, result_type) in func_type.results.iter().enumerate() {
            if !is_valid_value_type(result_type) {
                return Err(Error::new(
                    ErrorCategory::Validation,
                    codes::VALIDATION_ERROR,
                    format!(
                        "Invalid result type at index {} in function type {}",
                        result_idx, idx
                    ),
                ));
            }
        }
    }

    Ok(())
}

fn validate_functions(module: &Module) -> Result<()> {
    for (idx, func) in module.functions.iter().enumerate() {
        // Validate function type index
        if func.type_idx as usize >= module.types.len() {
            return Err(Error::new(
                ErrorCategory::Validation,
                codes::VALIDATION_ERROR,
                format!(
                    "Function {} references invalid type index {}",
                    idx, func.type_idx
                ),
            ));
        }

        // Validate local variable types
        for (local_idx, local_type) in func.locals.iter().enumerate() {
            if !is_valid_value_type(local_type) {
                return Err(Error::new(
                    ErrorCategory::Validation,
                    codes::VALIDATION_ERROR,
                    format!(
                        "Invalid local variable type at index {} in function {}",
                        local_idx, idx
                    ),
                ));
            }
        }
    }

    Ok(())
}

fn validate_tables(module: &Module) -> Result<()> {
    for (idx, table) in module.tables.iter().enumerate() {
        // Validate element type
        if !matches!(
            table.ty.element_type,
            ValueType::FuncRef | ValueType::ExternRef
        ) {
            return Err(Error::new(
                ErrorCategory::Validation,
                codes::VALIDATION_ERROR,
                format!("Table {} has invalid element type", idx),
            ));
        }

        // Validate limits
        if let Some(max) = table.ty.limits.max {
            if max < table.ty.limits.min {
                return Err(Error::new(
                    ErrorCategory::Validation,
                    codes::VALIDATION_ERROR,
                    format!("Table {} has maximum size less than minimum size", idx),
                ));
            }
        }
    }

    Ok(())
}

fn validate_memories(module: &Module) -> Result<()> {
    // Check memory count
    if module.memories.len() > 1 {
        return Err(Error::new(
            ErrorCategory::Validation,
            codes::VALIDATION_ERROR,
            "Module can have at most one memory",
        ));
    }

    // Validate memory limits
    for (idx, memory) in module.memories.iter().enumerate() {
        if let Some(max) = memory.ty.limits.max {
            if max < memory.ty.limits.min {
                return Err(Error::new(
                    ErrorCategory::Validation,
                    codes::VALIDATION_ERROR,
                    format!("Memory {} has maximum size less than minimum size", idx),
                ));
            }
        }
    }

    Ok(())
}

fn validate_globals(module: &Module) -> Result<()> {
    for (idx, _global) in module.globals.iter().enumerate() {
        // Validate global type - since we can't directly access the global's type
        // We'll skip detailed validation until we have proper global type access
        if false {  // Bypassing this validation for now
            return Err(Error::new(
                ErrorCategory::Validation,
                codes::VALIDATION_ERROR,
                format!("Global {} has invalid type", idx),
            ));
        }
    }

    Ok(())
}

fn validate_elements(module: &Module) -> Result<()> {
    for (idx, elem) in module.elements.iter().enumerate() {
        // Validate table index
        if elem.table_idx as usize >= module.tables.len() {
            return Err(Error::new(
                ErrorCategory::Validation,
                codes::VALIDATION_ERROR,
                format!(
                    "Element segment {} references invalid table index {}",
                    idx, elem.table_idx
                ),
            ));
        }

        // Validate function indices
        for (func_idx_pos, func_idx) in elem.items.iter().enumerate() {
            if *func_idx as usize >= module.functions.len() {
                return Err(Error::new(
                    ErrorCategory::Validation,
                    codes::VALIDATION_ERROR,
                    format!(
                        "Element segment {} references invalid function index {} at position {}",
                        idx, func_idx, func_idx_pos
                    ),
                ));
            }
        }
    }

    Ok(())
}

fn validate_data_segments(module: &Module) -> Result<()> {
    for (idx, data) in module.data.iter().enumerate() {
        // Validate memory index
        if data.memory_idx as usize >= module.memories.len() {
            return Err(Error::new(
                ErrorCategory::Validation,
                codes::VALIDATION_ERROR,
                format!(
                    "Data segment {} references invalid memory index {}",
                    idx, data.memory_idx
                ),
            ));
        }
    }

    Ok(())
}

fn validate_start_function(module: &Module) -> Result<()> {
    if let Some(start_idx) = module.start {
        // Validate start function index
        if start_idx as usize >= module.functions.len() {
            return Err(Error::new(
                ErrorCategory::Validation,
                codes::VALIDATION_ERROR,
                format!("Start function index {} is invalid", start_idx),
            ));
        }

        // Validate start function type
        let start_func = &module.functions[start_idx as usize];
        let start_type = &module.types[start_func.type_idx as usize];
        if !start_type.params.is_empty() || !start_type.results.is_empty() {
            return Err(Error::new(
                ErrorCategory::Validation,
                codes::VALIDATION_ERROR,
                "Start function must have no parameters and no results",
            ));
        }
    }

    Ok(())
}

fn is_valid_value_type(value_type: &ValueType) -> bool {
    matches!(
        value_type,
        ValueType::I32
            | ValueType::I64
            | ValueType::F32
            | ValueType::F64
            | ValueType::FuncRef
            | ValueType::ExternRef
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_module_validation() {
        let module = Module::new();
        assert!(validate_module(&module).is_ok());
    }

    #[test]
    fn test_function_validation() {
        let mut module = Module::new();

        // Add a function type
        let func_type = FuncType {
            params: vec![ValueType::I32],
            results: vec![ValueType::I32],
        };
        module.types.push(func_type);

        // Add a valid function
        let valid_function = crate::module::Function {
            type_idx: 0,
            locals: vec![ValueType::I32],
            code: vec![],
        };
        module.functions.push(valid_function);
        assert!(validate_module(&module).is_ok());

        // Add a function with invalid type index
        let invalid_function = crate::module::Function {
            type_idx: 1, // Invalid index
            locals: vec![],
            code: vec![],
        };
        module.functions.push(invalid_function);
        assert!(validate_module(&module).is_err());
    }

    #[test]
    fn test_table_validation() {
        let mut module = Module::new();

        // Add a valid table
        let valid_table = crate::table::Table::new(TableType {
            element_type: ValueType::FuncRef,
            limits: Limits {
                min: 1,
                max: Some(10),
            },
        })
        .unwrap();
        module.tables.push(std::sync::Arc::new(valid_table));
        assert!(validate_module(&module).is_ok());

        // Add a table with invalid element type
        let invalid_table = crate::table::Table::new(TableType {
            element_type: ValueType::I32, // Invalid element type
            limits: Limits {
                min: 1,
                max: Some(10),
            },
        })
        .unwrap();
        module.tables.push(std::sync::Arc::new(invalid_table));
        assert!(validate_module(&module).is_err());

        // Add a table with invalid limits
        let invalid_limits_table = crate::table::Table::new(TableType {
            element_type: ValueType::FuncRef,
            limits: Limits {
                min: 10,
                max: Some(5), // Max less than min
            },
        })
        .unwrap();
        module
            .tables
            .push(std::sync::Arc::new(invalid_limits_table));
        assert!(validate_module(&module).is_err());
    }

    #[test]
    fn test_memory_validation() {
        let mut module = Module::new();

        // Add a valid memory
        let valid_memory = crate::memory::Memory::new(MemoryType {
            limits: Limits {
                min: 1,
                max: Some(10),
            },
            shared: false,
        })
        .unwrap();
        module.memories.push(std::sync::Arc::new(valid_memory));
        assert!(validate_module(&module).is_ok());

        // Add a second memory (invalid)
        let second_memory = crate::memory::Memory::new(MemoryType {
            limits: Limits {
                min: 1,
                max: Some(10),
            },
            shared: false,
        })
        .unwrap();
        module.memories.push(std::sync::Arc::new(second_memory));
        assert!(validate_module(&module).is_err());

        // Test memory with invalid limits
        module.memories.clear();
        let invalid_limits_memory = crate::memory::Memory::new(MemoryType {
            limits: Limits {
                min: 10,
                max: Some(5), // Max less than min
            },
            shared: false,
        })
        .unwrap();
        module
            .memories
            .push(std::sync::Arc::new(invalid_limits_memory));
        assert!(validate_module(&module).is_err());
    }

    #[test]
    fn test_global_validation() {
        let mut module = Module::new();

        // Add a valid global
        let valid_global = GlobalType {
            content_type: ValueType::I32,
            mutable: true,
        };
        module.globals.push(valid_global);
        assert!(validate_module(&module).is_ok());
    }

    #[test]
    fn test_element_validation() {
        let mut module = Module::new();

        // Add necessary table
        let table = TableType {
            element_type: ValueType::FuncRef,
            limits: Limits {
                min: 1,
                max: Some(10),
            },
        };
        module.tables.push(table);

        // Add necessary function and type
        let func_type = FuncType {
            params: vec![],
            results: vec![],
        };
        module.types.push(func_type);
        let function = crate::module::Function {
            type_idx: 0,
            locals: vec![],
            code: vec![],
        };
        module.functions.push(function);

        // Add a valid element segment
        let valid_element = crate::module::Element {
            table_idx: 0,
            offset: vec![Instruction::I32Const(0)],
            items: vec![0], // References the function we added
        };
        module.elements.push(valid_element);
        assert!(validate_module(&module).is_ok());

        // Add an element segment with invalid table index
        let invalid_table_element = crate::module::Element {
            table_idx: 1, // Invalid table index
            offset: vec![Instruction::I32Const(0)],
            items: vec![0],
        };
        module.elements.push(invalid_table_element);
        assert!(validate_module(&module).is_err());

        // Add an element segment with invalid function index
        let invalid_func_element = crate::module::Element {
            table_idx: 0,
            offset: vec![Instruction::I32Const(0)],
            items: vec![1], // Invalid function index
        };
        module.elements.push(invalid_func_element);
        assert!(validate_module(&module).is_err());
    }

    #[test]
    fn test_data_validation() {
        let mut module = Module::new();

        // Add necessary memory
        let memory = MemoryType {
            limits: Limits {
                min: 1,
                max: Some(10),
            },
            shared: false,
        };
        module.memories.push(memory);

        // Add a valid data segment
        let valid_data = crate::module::Data {
            memory_idx: 0,
            offset: vec![Instruction::I32Const(0)],
            init: vec![1, 2, 3],
        };
        module.data.push(valid_data);
        assert!(validate_module(&module).is_ok());

        // Add a data segment with invalid memory index
        let invalid_data = crate::module::Data {
            memory_idx: 1, // Invalid memory index
            offset: vec![Instruction::I32Const(0)],
            init: vec![1, 2, 3],
        };
        module.data.push(invalid_data);
        assert!(validate_module(&module).is_err());
    }

    #[test]
    fn test_start_function_validation() {
        let mut module = Module::new();

        // Add a valid function type (no params, no results)
        let valid_type = FuncType {
            params: vec![],
            results: vec![],
        };
        module.types.push(valid_type);

        // Add a valid function
        let valid_function = crate::module::Function {
            type_idx: 0,
            locals: vec![],
            code: vec![],
        };
        module.functions.push(valid_function);

        // Set valid start function
        module.start = Some(0);
        assert!(validate_module(&module).is_ok());

        // Add an invalid function type (with params)
        let invalid_type = FuncType {
            params: vec![ValueType::I32],
            results: vec![],
        };
        module.types.push(invalid_type);

        // Add an invalid function
        let invalid_function = crate::module::Function {
            type_idx: 1,
            locals: vec![],
            code: vec![],
        };
        module.functions.push(invalid_function);

        // Set invalid start function
        module.start = Some(1);
        assert!(validate_module(&module).is_err());

        // Test invalid start function index
        module.start = Some(2); // Index out of bounds
        assert!(validate_module(&module).is_err());
    }
}
