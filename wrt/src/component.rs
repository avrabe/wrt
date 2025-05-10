use crate::{
    prelude::{
        ComponentInterface, Value, HashMap, 
    },
};
// Import directly from wrt_error
use wrt_error::{Error, Result, kinds};
use crate::{global::Global, table::Table};
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::RwLock;
use wrt_runtime::Memory;
use wrt_types::{
    ComponentType, ExternType, FuncType, GlobalType, InstanceType, MemoryType, Namespace,
    TableType, ValueType,
};

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
    instances: Vec<InstanceValue>,
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
#[derive(Debug, Clone)]
pub enum ExternValue {
    /// Function value
    Function(FunctionValue),
    /// Table value
    Table(TableValue),
    /// Memory value
    Memory(MemoryValue),
    /// Global value
    Global(GlobalValue),
    /// Trap value
    Trap(String),
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
#[derive(Debug, Clone)]
pub struct TableValue {
    /// Table type
    pub ty: TableType,
    /// Table instance
    pub table: Table,
}

/// Represents a memory value
#[derive(Debug, Clone)]
pub struct MemoryValue {
    /// Memory type
    pub ty: MemoryType,
    /// Memory instance
    pub memory: Arc<RwLock<Memory>>,
}

impl MemoryValue {
    /// Creates a new memory value
    ///
    /// # Arguments
    ///
    /// * `ty` - The memory type
    ///
    /// # Returns
    ///
    /// A new memory value
    ///
    /// # Errors
    ///
    /// Returns an error if the memory cannot be created
    pub fn new(ty: MemoryType) -> Result<Self> {
        let memory = Memory::new(ty.clone())?;
        Ok(Self {
            ty,
            memory: Arc::new(RwLock::new(memory)),
        })
    }

    /// Reads from memory
    ///
    /// # Arguments
    ///
    /// * `offset` - The offset to read from
    /// * `size` - The number of bytes to read
    ///
    /// # Returns
    ///
    /// The bytes read from memory
    ///
    /// # Errors
    ///
    /// Returns an error if the read fails
    pub fn read(&self, offset: u32, size: u32) -> Result<Vec<u8>> {
        let memory = self
            .memory
            .read()
            .expect("Failed to acquire memory read lock");
        let mut buffer = vec![0; size as usize];
        memory.read(offset, &mut buffer)?;
        Ok(buffer)
    }

    /// Writes to memory
    ///
    /// # Arguments
    ///
    /// * `offset` - The offset to write to
    /// * `bytes` - The bytes to write
    ///
    /// # Returns
    ///
    /// Ok(()) if the write succeeds
    ///
    /// # Errors
    ///
    /// Returns an error if the write fails
    pub fn write(&self, offset: u32, bytes: &[u8]) -> Result<()> {
        let mut memory = self
            .memory
            .write()
            .expect("Failed to acquire memory write lock");
        memory.write(offset, bytes)
    }

    /// Grows the memory by the given number of pages
    ///
    /// # Arguments
    ///
    /// * `pages` - The number of pages to grow by
    ///
    /// # Returns
    ///
    /// The previous size in pages
    ///
    /// # Errors
    ///
    /// Returns an error if the memory cannot be grown
    pub fn grow(&self, pages: u32) -> Result<u32> {
        let mut memory = self
            .memory
            .write()
            .expect("Failed to acquire memory write lock");
        memory.grow(pages)
    }
}

/// Represents a global value
#[derive(Debug, Clone)]
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
    /// Creates a new component with the given type
    #[must_use]
    pub const fn new(component_type: ComponentType) -> Self {
        Self {
            component_type,
            exports: Vec::new(),
            imports: Vec::new(),
            instances: Vec::new(),
        }
    }

    /// Instantiates the component with the given imports
    pub fn instantiate(&mut self, imports: Vec<Import>) -> Result<()> {
        // Validate imports
        if imports.len() != self.component_type.imports.len() {
            return Err(Error::new(kinds::ValidationError(format!(
                "Expected {} imports, got {}",
                self.component_type.imports.len(),
                imports.len()
            ))));
        }

        // Validate import types
        for ((import_name, import_namespace, import_type), import) in
            self.component_type.imports.iter().zip(imports.iter())
        {
            if import.name != *import_name {
                return Err(Error::new(kinds::ValidationError(format!(
                    "Expected import {}, got {}",
                    import_name, import.name
                ))));
            }

            if !wrt_types::component::types_are_compatible(import_type, &import.ty) {
                return Err(Error::new(kinds::ValidationError(format!(
                    "Import {import_name} has incompatible type"
                ))));
            }

            // Validate namespace if needed
            if !import_namespace.is_empty() {
                debug_println!("Import namespace: {}", import_namespace);
                // Future: Validate namespace matches expected pattern
            }
        }

        // Store imports
        self.imports = imports;

        // Initialize exports
        self.initialize_exports()?;

        // Link instances together
        self.link_instances()?;

        Ok(())
    }

    /// Initialize component exports
    fn initialize_exports(&mut self) -> Result<()> {
        let mut exports = Vec::new();

        for (name, ty) in &self.component_type.exports {
            let value = match ty {
                ExternType::Function(func_type) => {
                    // For function exports, create a reference to the function
                    ExternValue::Function(FunctionValue {
                        ty: func_type.clone(),
                        export_name: name.clone(),
                    })
                }
                ExternType::Table(table_type) => {
                    // Create a table export with default initialization
                    ExternValue::Table(TableValue {
                        ty: table_type.clone(),
                        table: Table::new(table_type.clone()),
                    })
                }
                ExternType::Memory(memory_type) => {
                    // Create a memory export with default initialization
                    ExternValue::Memory(MemoryValue {
                        ty: memory_type.clone(),
                        memory: Arc::new(RwLock::new(Memory::new(memory_type.clone())?)),
                    })
                }
                ExternType::Global(global_type) => {
                    // Create a global export with default initialization
                    ExternValue::Global(GlobalValue {
                        ty: global_type.clone(),
                        global: Global::new(
                            global_type.clone(),
                            Value::convert_from_wrt_types(&wrt_types::Value::default_for_type(
                                &global_type.value_type,
                            )),
                        )?,
                    })
                }
                _ => {
                    // More complex types are handled differently
                    ExternValue::Trap(format!("Export {name} not fully implemented"))
                }
            };

            exports.push(Export {
                name: name.clone(),
                ty: ty.clone(),
                value,
            });
        }

        self.exports = exports;
        Ok(())
    }

    /// Link instances together
    fn link_instances(&mut self) -> Result<()> {
        // Phase 1: Link imports to component exports
        self.link_imports_to_exports()?;

        // Phase 2: Link instance exports to component imports or exports
        self.link_instance_exports()?;

        // Phase 3: Update any export references that refer to instances
        self.finalize_instance_exports()?;

        Ok(())
    }

    /// Link component imports to exports
    fn link_imports_to_exports(&mut self) -> Result<()> {
        // For each export that needs to be linked to an import
        for export in &mut self.exports {
            // If this export is implemented by an import, link them
            if let Some(import) = self.imports.iter().find(|i| i.name == export.name) {
                if wrt_types::component::types_are_compatible(&export.ty, &import.ty) {
                    // Only link if the import is not already linked to another export
                    export.value = import.value.clone();
                    debug_println!("Linked export {} to import", export.name);
                }
            }
        }

        Ok(())
    }

    /// Link instance exports to component imports or exports
    fn link_instance_exports(&mut self) -> Result<()> {
        // For each instance
        for instance in &mut self.instances {
            // For each export in the instance
            for export in &mut instance.exports {
                // Try to find a matching import or export in the component
                if let Some(import) = self.imports.iter().find(|i| i.name == export.name) {
                    // We found a matching import, link it
                    if wrt_types::component::types_are_compatible(&export.ty, &import.ty) {
                        export.value = import.value.clone();
                        debug_println!(
                            "Linked instance export {} to component import",
                            export.name
                        );
                    }
                } else if let Some(comp_export) =
                    self.exports.iter().find(|e| e.name == export.name)
                {
                    // We found a matching export, link it
                    if wrt_types::component::types_are_compatible(&export.ty, &comp_export.ty) {
                        export.value = comp_export.value.clone();
                        debug_println!(
                            "Linked instance export {} to component export",
                            export.name
                        );
                    }
                }
            }
        }

        Ok(())
    }

    /// Finalize instance exports in component exports
    fn finalize_instance_exports(&mut self) -> Result<()> {
        // Update any component exports that reference instances
        for export in &mut self.exports {
            if let ExternType::Instance(instance_type) = &export.ty {
                // Find the matching instance
                if let Some(_instance) = self
                    .instances
                    .iter()
                    .find(|i| wrt_types::component::instance_types_match(instance_type, i.ty))
                {
                    // Replace the placeholder with a proper instance reference
                    // This is still a placeholder since we don't have a proper instance value type
                    export.value = ExternValue::Trap(format!(
                        "Instance {} linked but not fully implemented",
                        export.name
                    ));
                    debug_println!("Finalized instance export {}", export.name);
                }
            }
        }

        Ok(())
    }

    /// Reads from a named memory
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the memory export
    /// * `offset` - The offset to read from
    /// * `size` - The number of bytes to read
    ///
    /// # Returns
    ///
    /// The bytes read from memory
    ///
    /// # Errors
    ///
    /// Returns an error if the read fails or the memory doesn't exist
    pub fn read_memory(&self, name: &str, offset: u32, size: u32) -> Result<Vec<u8>> {
        // Find the memory export
        let memory_export = self.get_export(name)?;

        // Extract memory value
        let memory_value = match &memory_export.value {
            ExternValue::Memory(memory) => memory,
            _ => {
                return Err(Error::new(kinds::ValidationError(format!(
                    "Export {name} is not a memory"
                ))));
            }
        };

        // Perform the read
        memory_value.read(offset, size)
    }

    /// Writes to a named memory
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the memory export
    /// * `offset` - The offset to write to
    /// * `bytes` - The bytes to write
    ///
    /// # Returns
    ///
    /// Ok(()) if the write succeeds
    ///
    /// # Errors
    ///
    /// Returns an error if the write fails or the memory doesn't exist
    pub fn write_memory(&mut self, name: &str, offset: u32, bytes: &[u8]) -> Result<()> {
        // Find the memory export
        let memory_export = self.get_export(name)?;

        // Extract memory value
        let memory_value = match &memory_export.value {
            ExternValue::Memory(memory) => memory,
            _ => {
                return Err(Error::new(kinds::ValidationError(format!(
                    "Export {name} is not a memory"
                ))));
            }
        };

        // Perform the write
        memory_value.write(offset, bytes)
    }

    /// Executes an exported function
    ///
    /// This function takes the name of the exported function and the arguments
    /// to pass to it. It validates the arguments, executes the function, and
    /// returns the result.
    pub fn execute_function(&self, name: &str, args: Vec<Value>) -> Result<Vec<Value>> {
        // Find the export
        let export = self.get_export(name)?;

        // Check if it's a function
        let func_value = match &export.value {
            ExternValue::Function(func) => func,
            _ => {
                return Err(Error::new(kinds::ExecutionError(format!(
                    "Export {name} is not a function"
                ))));
            }
        };

        // Check argument count
        if args.len() != func_value.ty.params.len() {
            return Err(Error::new(kinds::ExecutionError(format!(
                "Expected {} arguments, got {}",
                func_value.ty.params.len(),
                args.len()
            ))));
        }

        // Validate argument types
        for (i, (arg, expected_type)) in args.iter().zip(func_value.ty.params.iter()).enumerate() {
            if !arg.matches_type(expected_type) {
                return Err(Error::new(kinds::ExecutionError(format!(
                    "Argument {i} has invalid type - expected {expected_type:?}, got {arg:?}"
                ))));
            }
        }

        // Call the function
        self.handle_function_call(name, &args)
    }

    /// Gets an export by name
    pub fn get_export(&self, name: &str) -> Result<&Export> {
        self.exports.iter().find(|e| e.name == name).ok_or_else(|| {
            Error::new(kinds::ValidationError(format!(
                "Export {name} not found"
            )))
        })
    }

    /// Gets a mutable reference to an export by name
    pub fn get_export_mut(&mut self, name: &str) -> Result<&mut Export> {
        self.exports
            .iter_mut()
            .find(|e| e.name == name)
            .ok_or_else(|| {
                Error::new(kinds::ValidationError(format!(
                    "Export {name} not found"
                )))
            })
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
                return Err(Error::new(kinds::ExecutionError(format!(
                    "Export {name} is not a function"
                ))));
            }
        };

        // Check argument count
        if args.len() != func_value.ty.params.len() {
            return Err(Error::new(kinds::ExecutionError(format!(
                "Expected {} arguments, got {}",
                func_value.ty.params.len(),
                args.len()
            ))));
        }

        // Call the function using the exported name
        // In this simplified version, just return a sample value
        Ok(vec![Value::I32(42)])
    }

    /// Resolves import by name and namespace
    #[must_use]
    pub fn resolve_import(&self, name: &str, _namespace: &Namespace) -> Option<&Import> {
        // For now, we ignore namespace and just match on name
        self.imports.iter().find(|i| i.name == name)
    }

    /// Creates a new export
    pub fn create_export(
        &mut self,
        name: String,
        ty: ExternType,
        value: ExternValue,
    ) -> Result<()> {
        // Check if export already exists
        if self.exports.iter().any(|e| e.name == name) {
            return Err(Error::new(kinds::ValidationError(format!(
                "Export {name} already exists"
            ))));
        }

        // Add the export
        self.exports.push(Export { name, ty, value });

        Ok(())
    }

    /// Imports a component
    pub fn import_component(&mut self, component: &Self, namespace: Option<&str>) -> Result<()> {
        let ns = if let Some(ns_str) = namespace {
            Namespace::from_string(ns_str)
        } else {
            Namespace {
                elements: Vec::new(),
            }
        };

        // Import all exports from the component
        for export in &component.exports {
            // Skip if name already exists
            if self.exports.iter().any(|e| e.name == export.name) {
                debug_println!("Skipping import of {} - name already exists", export.name);
                continue;
            }

            // Determine export name with namespace
            let export_name = if ns.is_empty() {
                export.name.clone()
            } else {
                format!("{}.{}", ns.to_string(), export.name)
            };

            // Create the export
            self.exports.push(Export {
                name: export_name,
                ty: export.ty.clone(),
                value: export.value.clone(),
            });

            debug_println!("Imported export {} from component", export.name);
        }

        Ok(())
    }

    /// Exports a value
    pub fn export_value(&mut self, name: &str, ty: ExternType, value: ExternValue) -> Result<()> {
        // Validate type compatibility
        match &value {
            ExternValue::Function(func) => {
                if let ExternType::Function(export_func_type) = &ty {
                    if !wrt_types::component::func_types_compatible(&func.ty, export_func_type) {
                        return Err(Error::new(kinds::ValidationError(format!(
                            "Function type mismatch for export {name}"
                        ))));
                    }
                } else {
                    return Err(Error::new(kinds::ValidationError(format!(
                        "Expected function type for export {name}"
                    ))));
                }
            }
            ExternValue::Table(table) => {
                if let ExternType::Table(export_table_type) = &ty {
                    if table.ty.element_type != export_table_type.element_type
                        || table.ty.limits.min != export_table_type.limits.min
                        || table.ty.limits.max != export_table_type.limits.max
                    {
                        return Err(Error::new(kinds::ValidationError(format!(
                            "Table type mismatch for export {name}"
                        ))));
                    }
                } else {
                    return Err(Error::new(kinds::ValidationError(format!(
                        "Expected table type for export {name}"
                    ))));
                }
            }
            ExternValue::Memory(memory) => {
                if let ExternType::Memory(export_memory_type) = &ty {
                    if memory.ty.limits.min != export_memory_type.limits.min
                        || memory.ty.limits.max != export_memory_type.limits.max
                    {
                        return Err(Error::new(kinds::ValidationError(format!(
                            "Memory type mismatch for export {name}"
                        ))));
                    }
                } else {
                    return Err(Error::new(kinds::ValidationError(format!(
                        "Expected memory type for export {name}"
                    ))));
                }
            }
            ExternValue::Global(global) => {
                if let ExternType::Global(export_global_type) = &ty {
                    if global.ty.value_type != export_global_type.value_type
                        || global.ty.mutable != export_global_type.mutable
                    {
                        return Err(Error::new(kinds::ValidationError(format!(
                            "Global type mismatch for export {name}"
                        ))));
                    }
                } else {
                    return Err(Error::new(kinds::ValidationError(format!(
                        "Expected global type for export {name}"
                    ))));
                }
            }
            ExternValue::Trap(_) => {
                return Err(Error::new(kinds::ValidationError(format!(
                    "Cannot export trap value for {name}"
                ))));
            }
        }

        // Create the export
        self.create_export(name.to_string(), ty, value)
    }

    /// Validates a component
    pub fn validate(&self) -> Result<()> {
        // Check that all required imports are provided
        for (name, _namespace, _ty) in &self.component_type.imports {
            if !self.imports.iter().any(|import| import.name == *name) {
                return Err(Error::new(kinds::ValidationError(format!(
                    "Missing required import {name}"
                ))));
            }
        }

        // Check that all declared exports are provided
        for (name, ty) in &self.component_type.exports {
            if let Some(export) = self.exports.iter().find(|e| e.name == *name) {
                if !wrt_types::component::types_are_compatible(ty, &export.ty) {
                    return Err(Error::new(kinds::ValidationError(format!(
                        "Export {name} has incompatible type"
                    ))));
                }
            } else {
                return Err(Error::new(kinds::ValidationError(format!(
                    "Missing declared export {name}"
                ))));
            }
        }

        Ok(())
    }

    pub fn get_export_instance_value(&self, name: &str) -> Result<InstanceValue> {
        self.exports
            .iter()
            .find(|e| e.name == name)
            .ok_or_else(|| {
                Error::new(kinds::ValidationError(format!(
                    "Export {name} not found"
                )))
            })
            .map(|export| export.instance_value.clone())
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
    #[must_use]
    pub const fn new() -> Self {
        Self {
            functions: Vec::new(),
        }
    }

    /// Adds a host function
    pub fn add_function(&mut self, name: String, func: FunctionValue) {
        self.functions.push((name, func));
    }

    /// Gets a host function by name
    #[must_use]
    pub fn get_function(&self, name: &str) -> Option<&FunctionValue> {
        self.functions
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, f)| f)
    }

    /// Calls a host function
    pub fn call_function(&self, name: &str, args: &[Value]) -> Result<Vec<Value>> {
        let func_value = self.get_function(name).ok_or_else(|| {
            Error::new(prelude::kinds::ExecutionError(format!(
                "Host function {name} not found"
            )))
        })?;

        // Check argument count
        if args.len() != func_value.ty.params.len() {
            return Err(Error::new(prelude::kinds::ExecutionError(format!(
                "Expected {} arguments, got {}",
                func_value.ty.params.len(),
                args.len()
            ))));
        }

        // This is a simplified implementation - in a real system, you would
        // have a way to call functions based on their export_name
        Ok(vec![Value::I32(42)]) // Default implementation returns a sample value
    }
}

/// Debug print helper for non-std environments
#[cfg(feature = "std")]
#[allow(dead_code)]
fn debug_println(msg: &str) {
    if let Ok(debug_comp) = std::env::var("WRT_DEBUG_COMPONENT") {
        if debug_comp == "1" || debug_comp.to_lowercase() == "true" {
            println!("[COMPONENT_DEBUG]: {msg}");
        }
    }
}

/// Debug print helper for non-std environments
#[cfg(not(feature = "std"))]
fn debug_println(_msg: &str) {
    // This function is currently unused but kept for future debugging
}

// Add conversion helpers between wrt_types values and internal values
impl Value {
    fn convert_from_wrt_types(value: &wrt_types::Value) -> Self {
        match value {
            wrt_types::Value::I32(val) => Self::I32(*val),
            wrt_types::Value::I64(val) => Self::I64(*val),
            wrt_types::Value::F32(val) => Self::F32(*val),
            wrt_types::Value::F64(val) => Self::F64(*val),
            wrt_types::Value::V128(val) => Self::V128(*val),
            wrt_types::Value::FuncRef(_) => Self::FuncRef(0),
            wrt_types::Value::ExternRef(_) => Self::ExternRef(0),
        }
    }

    fn convert_to_wrt_types(&self) -> wrt_types::Value {
        match self {
            Self::I32(val) => wrt_types::Value::I32(*val),
            Self::I64(val) => wrt_types::Value::I64(*val),
            Self::F32(val) => wrt_types::Value::F32(*val),
            Self::F64(val) => wrt_types::Value::F64(*val),
            Self::V128(val) => wrt_types::Value::V128(*val),
            Self::FuncRef(val) => wrt_types::Value::FuncRef(Some(*val as u32)),
            Self::ExternRef(val) => wrt_types::Value::ExternRef(Some(*val as u32)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::values::Value;
    use crate::ValueType;

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
                        limits: Limits {
                            min: 1,
                            max: Some(2),
                        },
                        shared: false,
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
                assert_eq!(mem.ty.limits.min, 1);
                assert_eq!(mem.ty.limits.max, Some(2));
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
            ty: ExternType::Memory(MemoryType {
                limits: Limits { min: 1, max: None },
                shared: false,
            }),
            value: ExternValue::Memory(MemoryValue {
                ty: MemoryType {
                    limits: Limits { min: 1, max: None },
                    shared: false,
                },
                memory: Arc::new(RwLock::new(Memory::new(MemoryType {
                    limits: Limits { min: 1, max: None },
                    shared: false,
                })?)),
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
        let result = component.execute_function("multiply", vec![Value::I32(5), Value::I32(3)])?;
        assert_eq!(result, vec![Value::I32(42)]); // Default implementation returns 42

        // Test function call with wrong number of arguments
        assert!(component
            .execute_function("multiply", vec![Value::I32(5)])
            .is_err());

        // Test function call to non-existent function
        assert!(component
            .execute_function("nonexistent", vec![Value::I32(5)])
            .is_err());

        // Test function call to non-function export
        assert!(component
            .execute_function("memory", vec![Value::I32(5)])
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
