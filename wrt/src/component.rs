use crate::error::{Error, Result};
use crate::types::*;
use crate::values::Value;
use crate::{format, String, Vec};
use crate::{Global, Memory, Table};
#[cfg(not(feature = "std"))]
use alloc::string::ToString;
#[cfg(not(feature = "std"))]
use alloc::vec;
#[cfg(feature = "std")]
use std::vec;

/// Represents a component instance
#[derive(Debug)]
pub struct Component {
    /// Component type
    component_type: ComponentType,
    /// Component exports
    exports: Vec<Export>,
    /// Component imports
    imports: Vec<Import>,
    /// Component instances
    #[allow(dead_code)]
    instances: Vec<InstanceValue>,
}

/// Represents a component type
#[derive(Debug)]
pub struct ComponentType {
    /// Component imports
    pub imports: Vec<(String, String, ExternType)>,
    /// Component exports
    pub exports: Vec<(String, ExternType)>,
    /// Component instances
    pub instances: Vec<InstanceType>,
}

/// Represents an instance type
#[derive(Debug)]
pub struct InstanceType {
    /// Instance exports
    pub exports: Vec<(String, ExternType)>,
}

/// Represents a component export
#[derive(Debug)]
pub struct Export {
    /// Export name
    pub name: String,
    /// Export type
    pub ty: ExternType,
    /// Export value
    pub value: ExternValue,
}

/// Represents a component import
#[derive(Debug)]
pub struct Import {
    /// Import name
    pub name: String,
    /// Import type
    pub ty: ExternType,
    /// Import value
    pub value: ExternValue,
}

/// Represents an external value
#[derive(Debug)]
pub enum ExternValue {
    /// Function value
    Function(FunctionValue),
    /// Table value
    Table(TableValue),
    /// Memory value
    Memory(MemoryValue),
    /// Global value
    Global(GlobalValue),
}

/// Represents a function value
#[derive(Debug, Clone)]
pub struct FunctionValue {
    /// Function type
    pub ty: FuncType,
    /// Export name that this function refers to
    pub export_name: String,
}

/// Represents a table value
#[derive(Debug)]
pub struct TableValue {
    /// Table type
    pub ty: TableType,
    /// Table instance
    pub table: Table,
}

/// Represents a memory value
#[derive(Debug)]
pub struct MemoryValue {
    /// Memory type
    pub ty: MemoryType,
    /// Memory instance
    pub memory: Memory,
}

/// Represents a global value
#[derive(Debug)]
pub struct GlobalValue {
    /// Global type
    pub ty: GlobalType,
    /// Global instance
    pub global: Global,
}

/// Represents an instance value
#[derive(Debug)]
pub struct InstanceValue {
    /// Instance type
    pub ty: InstanceType,
    /// Instance exports
    pub exports: Vec<Export>,
}

impl Component {
    /// Creates a new component instance
    pub fn new(component_type: ComponentType) -> Self {
        Self {
            component_type,
            exports: Vec::new(),
            imports: Vec::new(),
            instances: Vec::new(),
        }
    }

    /// Instantiates a component
    pub fn instantiate(&mut self, imports: Vec<Import>) -> Result<()> {
        // Validate imports
        if imports.len() != self.component_type.imports.len() {
            return Err(Error::Validation(format!(
                "Expected {} imports, got {}",
                self.component_type.imports.len(),
                imports.len()
            )));
        }

        // Check import types
        for ((expected_name, _expected_module, expected_type), import) in
            self.component_type.imports.iter().zip(imports.iter())
        {
            if import.name != *expected_name || import.ty != *expected_type {
                return Err(Error::Validation(format!(
                    "Import {} has invalid type",
                    import.name
                )));
            }
        }

        // Store imports
        self.imports = imports;

        // Initialize exports
        for (name, ty) in &self.component_type.exports {
            let value = match ty {
                ExternType::Function(func_type) => {
                    // Create a named function instead of a closure to avoid self capture
                    let export_name = name.clone();
                    ExternValue::Function(FunctionValue {
                        ty: func_type.clone(),
                        // Use an unbound function reference instead of a closure
                        export_name,
                    })
                }
                ExternType::Table(table_type) => ExternValue::Table(TableValue {
                    ty: table_type.clone(),
                    table: Table::new(table_type.clone()),
                }),
                ExternType::Memory(memory_type) => ExternValue::Memory(MemoryValue {
                    ty: memory_type.clone(),
                    memory: Memory::new(memory_type.clone()),
                }),
                ExternType::Global(global_type) => ExternValue::Global(GlobalValue {
                    ty: global_type.clone(),
                    global: Global::new(
                        global_type.clone(),
                        Value::default_for_type(&global_type.content_type),
                    )?,
                }),
            };

            self.exports.push(Export {
                name: name.clone(),
                ty: ty.clone(),
                value,
            });
        }

        Ok(())
    }

    /// Gets an export by name
    pub fn get_export(&self, name: &str) -> Result<&Export> {
        self.exports
            .iter()
            .find(|e| e.name == name)
            .ok_or_else(|| Error::Validation(format!("Export {} not found", name)))
    }

    /// Gets a mutable reference to an export by name
    pub fn get_export_mut(&mut self, name: &str) -> Result<&mut Export> {
        self.exports
            .iter_mut()
            .find(|e| e.name == name)
            .ok_or_else(|| Error::Validation(format!("Export {} not found", name)))
    }

    /// Handles a function call from the host
    #[allow(dead_code)]
    fn handle_function_call(&self, name: &str, args: &[Value]) -> Result<Vec<Value>> {
        // Find the export
        let export = self.get_export(name)?;

        // Check if it's a function
        let func_value = match &export.value {
            ExternValue::Function(func) => func,
            _ => {
                return Err(Error::Execution(format!(
                    "Export {} is not a function",
                    name
                )))
            }
        };

        // Check argument count
        if args.len() != func_value.ty.params.len() {
            return Err(Error::Execution(format!(
                "Expected {} arguments, got {}",
                func_value.ty.params.len(),
                args.len()
            )));
        }

        // Call the function using the exported name
        // In this simplified version, just return a sample value
        Ok(vec![Value::I32(42)])
    }
}

/// Represents a host implementation
#[derive(Debug)]
pub struct Host {
    /// Host functions
    functions: Vec<(String, FunctionValue)>,
}

impl Default for Host {
    fn default() -> Self {
        Self::new()
    }
}

impl Host {
    /// Creates a new host implementation
    pub fn new() -> Self {
        Self {
            functions: Vec::new(),
        }
    }

    /// Adds a host function
    pub fn add_function(&mut self, name: String, func: FunctionValue) {
        self.functions.push((name, func));
    }

    /// Gets a host function by name
    pub fn get_function(&self, name: &str) -> Option<&FunctionValue> {
        self.functions
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, f)| f)
    }

    /// Calls a host function
    pub fn call_function(&self, name: &str, args: &[Value]) -> Result<Vec<Value>> {
        let func_value = self
            .get_function(name)
            .ok_or_else(|| Error::Execution(format!("Host function {} not found", name)))?;

        // Check argument count
        if args.len() != func_value.ty.params.len() {
            return Err(Error::Execution(format!(
                "Expected {} arguments, got {}",
                func_value.ty.params.len(),
                args.len()
            )));
        }

        // This is a simplified implementation - in a real system, you would
        // have a way to call functions based on their export_name
        Ok(vec![Value::I32(42)]) // Default implementation returns a sample value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Helper function to create a test component type
    fn create_test_component_type() -> ComponentType {
        ComponentType {
            imports: vec![(
                "add".to_string(),
                "math".to_string(),
                ExternType::Function(FuncType {
                    params: vec![ValueType::I32, ValueType::I32],
                    results: vec![ValueType::I32],
                }),
            )],
            exports: vec![
                (
                    "multiply".to_string(),
                    ExternType::Function(FuncType {
                        params: vec![ValueType::I32, ValueType::I32],
                        results: vec![ValueType::I32],
                    }),
                ),
                (
                    "memory".to_string(),
                    ExternType::Memory(MemoryType {
                        min: 1,
                        max: Some(2),
                    }),
                ),
            ],
            instances: Vec::new(),
        }
    }

    // Helper function to create a test import
    fn create_test_import() -> Import {
        Import {
            name: "add".to_string(),
            ty: ExternType::Function(FuncType {
                params: vec![ValueType::I32, ValueType::I32],
                results: vec![ValueType::I32],
            }),
            value: ExternValue::Function(FunctionValue {
                ty: FuncType {
                    params: vec![ValueType::I32, ValueType::I32],
                    results: vec![ValueType::I32],
                },
                export_name: "add".to_string(),
            }),
        }
    }

    #[test]
    fn test_component_creation_and_instantiation() -> Result<()> {
        let component_type = create_test_component_type();
        let mut component = Component::new(component_type);
        let import = create_test_import();

        // Test instantiation
        assert!(component.instantiate(vec![import]).is_ok());

        // Test export access
        let export = component.get_export("multiply")?;
        assert_eq!(export.name, "multiply");
        match &export.ty {
            ExternType::Function(func_type) => {
                assert_eq!(func_type.params.len(), 2);
                assert_eq!(func_type.results.len(), 1);
            }
            _ => panic!("Expected function type"),
        }

        Ok(())
    }

    #[test]
    fn test_component_export_types() -> Result<()> {
        let component_type = create_test_component_type();
        let mut component = Component::new(component_type);
        let import = create_test_import();
        component.instantiate(vec![import])?;

        // Test function export
        let func_export = component.get_export("multiply")?;
        assert_eq!(func_export.name, "multiply");
        match &func_export.value {
            ExternValue::Function(func) => {
                assert_eq!(func.ty.params.len(), 2);
                assert_eq!(func.ty.results.len(), 1);
            }
            _ => panic!("Expected function export"),
        }

        // Test memory export
        let mem_export = component.get_export("memory")?;
        assert_eq!(mem_export.name, "memory");
        match &mem_export.value {
            ExternValue::Memory(mem) => {
                assert_eq!(mem.ty.min, 1);
                assert_eq!(mem.ty.max, Some(2));
            }
            _ => panic!("Expected memory export"),
        }

        Ok(())
    }

    #[test]
    fn test_component_invalid_instantiation() {
        let component_type = ComponentType {
            imports: vec![(
                "add".to_string(),
                "math".to_string(),
                ExternType::Function(FuncType {
                    params: vec![ValueType::I32, ValueType::I32],
                    results: vec![ValueType::I32],
                }),
            )],
            exports: Vec::new(),
            instances: Vec::new(),
        };

        let mut component = Component::new(component_type);

        // Test instantiation with wrong number of imports
        assert!(component.instantiate(vec![]).is_err());

        // Test instantiation with wrong import type
        let wrong_import = Import {
            name: "add".to_string(),
            ty: ExternType::Memory(MemoryType { min: 1, max: None }),
            value: ExternValue::Memory(MemoryValue {
                ty: MemoryType { min: 1, max: None },
                memory: Memory::new(MemoryType { min: 1, max: None }),
            }),
        };
        assert!(component.instantiate(vec![wrong_import]).is_err());
    }

    #[test]
    fn test_component_function_calls() -> Result<()> {
        let component_type = create_test_component_type();
        let mut component = Component::new(component_type);
        let import = create_test_import();
        component.instantiate(vec![import])?;

        // Test valid function call
        let result = component.handle_function_call("multiply", &[Value::I32(5), Value::I32(3)])?;
        assert_eq!(result, vec![Value::I32(42)]); // Default implementation returns 42

        // Test function call with wrong number of arguments
        assert!(component
            .handle_function_call("multiply", &[Value::I32(5)])
            .is_err());

        // Test function call to non-existent function
        assert!(component
            .handle_function_call("nonexistent", &[Value::I32(5)])
            .is_err());

        // Test function call to non-function export
        assert!(component
            .handle_function_call("memory", &[Value::I32(5)])
            .is_err());

        Ok(())
    }

    #[test]
    fn test_host_function_management() -> Result<()> {
        let mut host = Host::new();

        // Add a host function
        let func_value = FunctionValue {
            ty: FuncType {
                params: vec![ValueType::I32],
                results: vec![ValueType::I32],
            },
            export_name: "increment".to_string(),
        };
        host.add_function("increment".to_string(), func_value);

        // Test function retrieval
        let retrieved_func = host.get_function("increment");
        assert!(retrieved_func.is_some());
        assert_eq!(retrieved_func.unwrap().export_name, "increment");

        // Test nonexistent function
        assert!(host.get_function("nonexistent").is_none());

        // Test function call
        let result = host.call_function("increment", &[Value::I32(5)])?;
        assert_eq!(result, vec![Value::I32(42)]); // Default implementation returns 42

        Ok(())
    }

    #[test]
    fn test_host_function_call_validation() {
        let mut host = Host::new();

        // Add a host function
        let func_value = FunctionValue {
            ty: FuncType {
                params: vec![ValueType::I32, ValueType::I32],
                results: vec![ValueType::I32],
            },
            export_name: "add".to_string(),
        };
        host.add_function("add".to_string(), func_value);

        // Test call with wrong number of arguments
        let result = host.call_function("add", &[Value::I32(5)]);
        assert!(result.is_err());

        // Test call to nonexistent function
        let result = host.call_function("nonexistent", &[Value::I32(5)]);
        assert!(result.is_err());
    }

    #[test]
    fn test_export_mutation() -> Result<()> {
        let component_type = create_test_component_type();
        let mut component = Component::new(component_type);
        let import = create_test_import();
        component.instantiate(vec![import])?;

        // Test mutable access to export
        let export = component.get_export_mut("multiply")?;
        assert_eq!(export.name, "multiply");

        // Test mutable access to nonexistent export
        assert!(component.get_export_mut("nonexistent").is_err());

        Ok(())
    }
}
