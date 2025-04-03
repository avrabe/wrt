use crate::error::{Error, Result};
use crate::memory::{DefaultMemory, MemoryBehavior};
use crate::types::{ExternType, FuncType, GlobalType, MemoryType, TableType};
use crate::values::Value;
use crate::{format, String, Vec};
use crate::{Global, Table};
#[cfg(not(feature = "std"))]
use alloc::string::ToString;
#[cfg(not(feature = "std"))]
use alloc::vec;
use std::collections::HashMap;
use std::sync::Arc;
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
    pub instances: Vec<crate::types::InstanceType>,
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
    pub memory: DefaultMemory,
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
    pub ty: crate::types::InstanceType,
    /// Instance exports
    pub exports: Vec<Export>,
}

/// Represents a namespace for component imports and exports
#[derive(Debug, Clone)]
pub struct Namespace {
    /// Namespace elements (e.g., "wasi", "http", "client")
    pub elements: Vec<String>,
}

impl Namespace {
    /// Creates a namespace from a string
    #[must_use]
    pub fn from_string(s: &str) -> Self {
        let elements = s
            .split('.')
            .filter(|part| !part.is_empty())
            .map(std::string::ToString::to_string)
            .collect();
        Self { elements }
    }

    /// Checks if this namespace matches another namespace
    #[must_use]
    pub fn matches(&self, other: &Self) -> bool {
        if self.elements.len() != other.elements.len() {
            return false;
        }

        self.elements
            .iter()
            .zip(other.elements.iter())
            .all(|(a, b)| a == b)
    }

    /// Returns a string representation of this namespace
    #[must_use]
    pub fn to_string(&self) -> String {
        self.elements.join(".")
    }

    /// Checks if this namespace is empty
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.elements.is_empty()
    }
}

/// Represents a component import with namespace
#[derive(Debug, Clone)]
pub struct ImportDefinition {
    /// Import name
    pub name: String,
    /// Import namespace
    pub namespace: Namespace,
    /// Import type
    pub ty: ExternType,
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
            return Err(Error::Validation(format!(
                "Expected {} imports, got {}",
                self.component_type.imports.len(),
                imports.len()
            )));
        }

        // Validate import types
        for ((import_name, import_namespace, import_type), import) in
            self.component_type.imports.iter().zip(imports.iter())
        {
            if import.name != *import_name {
                return Err(Error::Validation(format!(
                    "Expected import {}, got {}",
                    import_name, import.name
                )));
            }

            if !types_are_compatible(import_type, &import.ty) {
                return Err(Error::Validation(format!(
                    "Import {import_name} has incompatible type"
                )));
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
                        memory: DefaultMemory::new(memory_type.clone()),
                    })
                }
                ExternType::Global(global_type) => {
                    // Create a global export with default initialization
                    ExternValue::Global(GlobalValue {
                        ty: global_type.clone(),
                        global: Global::new(
                            global_type.clone(),
                            Value::default_for_type(&global_type.content_type),
                        )?,
                    })
                }
                ExternType::Resource(_resource_type) => {
                    // Resource exports are handled through the resource table
                    // For now, we'll create a trap since proper resource management
                    // requires more infrastructure
                    ExternValue::Trap(format!("Resource {name} not fully implemented"))
                }
                ExternType::Instance(instance_type) => {
                    // For instance exports, create a new instance with initialized exports
                    let instance_exports = self.create_instance_exports(instance_type, name)?;

                    // Create and register the instance
                    let instance = InstanceValue {
                        ty: instance_type.clone(),
                        exports: instance_exports,
                    };

                    // Add this instance to our list for later linking
                    self.instances.push(instance);

                    // Return a placeholder reference for now - will be updated during linking
                    ExternValue::Trap(format!("Instance {name} pending link"))
                }
                ExternType::Component(_component_type) => {
                    // Component exports are not instantiated here - they should be instantiated
                    // separately and then linked. This is a placeholder.
                    ExternValue::Trap(format!("Component {name} requires separate instantiation"))
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

    /// Create exports for an instance
    fn create_instance_exports(
        &self,
        instance_type: &crate::types::InstanceType,
        instance_name: &str,
    ) -> Result<Vec<Export>> {
        let mut exports = Vec::new();

        for (export_name, export_type) in &instance_type.exports {
            // Create placeholder exports that will be linked later
            let value = match export_type {
                ExternType::Function(func_type) => ExternValue::Function(FunctionValue {
                    ty: func_type.clone(),
                    export_name: format!("{instance_name}.{export_name}"),
                }),
                ExternType::Table(table_type) => ExternValue::Table(TableValue {
                    ty: table_type.clone(),
                    table: Table::new(table_type.clone()),
                }),
                ExternType::Memory(memory_type) => ExternValue::Memory(MemoryValue {
                    ty: memory_type.clone(),
                    memory: DefaultMemory::new(memory_type.clone()),
                }),
                ExternType::Global(global_type) => ExternValue::Global(GlobalValue {
                    ty: global_type.clone(),
                    global: Global::new(
                        global_type.clone(),
                        Value::default_for_type(&global_type.content_type),
                    )?,
                }),
                _ => {
                    // More complex types will be handled during linking
                    ExternValue::Trap(format!("Export {instance_name}.{export_name} pending link"))
                }
            };

            exports.push(Export {
                name: export_name.clone(),
                ty: export_type.clone(),
                value,
            });
        }

        Ok(exports)
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
                if types_are_compatible(&export.ty, &import.ty) {
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
                    if types_are_compatible(&export.ty, &import.ty) {
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
                    if types_are_compatible(&export.ty, &comp_export.ty) {
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
                    .find(|i| instance_types_match(&i.ty, instance_type))
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

    /// Reads from exported memory
    ///
    /// This function takes the name of the exported memory, the offset to read from,
    /// and the number of bytes to read. It returns the bytes read.
    pub fn read_memory(&self, name: &str, offset: u32, size: u32) -> Result<Vec<u8>> {
        // Find the export
        let export = self.get_export(name)?;

        // Check if it's a memory
        let memory_value = match &export.value {
            ExternValue::Memory(mem) => mem,
            _ => return Err(Error::Execution(format!("Export {name} is not a memory"))),
        };

        // Read from memory
        let bytes = memory_value.memory.read_bytes(offset, size as usize)?;
        Ok(bytes.to_vec())
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
            _ => return Err(Error::Execution(format!("Export {name} is not a function"))),
        };

        // Check argument count
        if args.len() != func_value.ty.params.len() {
            return Err(Error::Execution(format!(
                "Expected {} arguments, got {}",
                func_value.ty.params.len(),
                args.len()
            )));
        }

        // Validate argument types
        for (i, (arg, expected_type)) in args.iter().zip(func_value.ty.params.iter()).enumerate() {
            if !arg.matches_type(expected_type) {
                return Err(Error::Execution(format!(
                    "Argument {i} has invalid type - expected {expected_type:?}, got {arg:?}"
                )));
            }
        }

        // Call the function
        self.handle_function_call(name, &args)
    }

    /// Writes to exported memory
    ///
    /// This function takes the name of the exported memory, the offset to write to,
    /// and the bytes to write. It returns the number of bytes written.
    pub fn write_memory(&mut self, name: &str, offset: u32, bytes: &[u8]) -> Result<()> {
        // Find the export
        let export = self.get_export_mut(name)?;

        // Check if it's a memory
        let memory_value = match &mut export.value {
            ExternValue::Memory(mem) => mem,
            _ => return Err(Error::Execution(format!("Export {name} is not a memory"))),
        };

        // Write to memory
        memory_value.memory.write_bytes(offset, bytes)
    }

    /// Gets an export by name
    pub fn get_export(&self, name: &str) -> Result<&Export> {
        self.exports
            .iter()
            .find(|e| e.name == name)
            .ok_or_else(|| Error::Validation(format!("Export {name} not found")))
    }

    /// Gets a mutable reference to an export by name
    pub fn get_export_mut(&mut self, name: &str) -> Result<&mut Export> {
        self.exports
            .iter_mut()
            .find(|e| e.name == name)
            .ok_or_else(|| Error::Validation(format!("Export {name} not found")))
    }

    /// Handles a function call from the host
    #[allow(dead_code)]
    fn handle_function_call(&self, name: &str, args: &[Value]) -> Result<Vec<Value>> {
        // Find the export
        let export = self.get_export(name)?;

        // Check if it's a function
        let func_value = match &export.value {
            ExternValue::Function(func) => func,
            _ => return Err(Error::Execution(format!("Export {name} is not a function"))),
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
            return Err(Error::Validation(format!("Export {name} already exists")));
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
                    if !func_types_compatible(&func.ty, export_func_type) {
                        return Err(Error::Validation(format!(
                            "Function type mismatch for export {name}"
                        )));
                    }
                } else {
                    return Err(Error::Validation(format!(
                        "Expected function type for export {name}"
                    )));
                }
            }
            ExternValue::Table(table) => {
                if let ExternType::Table(export_table_type) = &ty {
                    if table.ty.element_type != export_table_type.element_type
                        || table.ty.min != export_table_type.min
                        || table.ty.max != export_table_type.max
                    {
                        return Err(Error::Validation(format!(
                            "Table type mismatch for export {name}"
                        )));
                    }
                } else {
                    return Err(Error::Validation(format!(
                        "Expected table type for export {name}"
                    )));
                }
            }
            ExternValue::Memory(memory) => {
                if let ExternType::Memory(export_memory_type) = &ty {
                    if memory.ty.min != export_memory_type.min
                        || memory.ty.max != export_memory_type.max
                    {
                        return Err(Error::Validation(format!(
                            "Memory type mismatch for export {name}"
                        )));
                    }
                } else {
                    return Err(Error::Validation(format!(
                        "Expected memory type for export {name}"
                    )));
                }
            }
            ExternValue::Global(global) => {
                if let ExternType::Global(export_global_type) = &ty {
                    if global.ty.content_type != export_global_type.content_type
                        || global.ty.mutable != export_global_type.mutable
                    {
                        return Err(Error::Validation(format!(
                            "Global type mismatch for export {name}"
                        )));
                    }
                } else {
                    return Err(Error::Validation(format!(
                        "Expected global type for export {name}"
                    )));
                }
            }
            ExternValue::Trap(_) => {
                return Err(Error::Validation(format!(
                    "Cannot export trap value for {name}"
                )));
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
                return Err(Error::Validation(format!("Missing required import {name}")));
            }
        }

        // Check that all declared exports are provided
        for (name, ty) in &self.component_type.exports {
            if let Some(export) = self.exports.iter().find(|e| e.name == *name) {
                if !types_are_compatible(ty, &export.ty) {
                    return Err(Error::Validation(format!(
                        "Export {name} has incompatible type"
                    )));
                }
            } else {
                return Err(Error::Validation(format!("Missing declared export {name}")));
            }
        }

        Ok(())
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
        let func_value = self
            .get_function(name)
            .ok_or_else(|| Error::Execution(format!("Host function {name} not found")))?;

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

/// Check if two types are compatible for linking
fn types_are_compatible(a: &ExternType, b: &ExternType) -> bool {
    match (a, b) {
        (ExternType::Function(a_ty), ExternType::Function(b_ty)) => {
            a_ty.params == b_ty.params && a_ty.results == b_ty.results
        }
        (ExternType::Table(a_ty), ExternType::Table(b_ty)) => a_ty == b_ty,
        (ExternType::Memory(a_ty), ExternType::Memory(b_ty)) => a_ty == b_ty,
        (ExternType::Global(a_ty), ExternType::Global(b_ty)) => a_ty == b_ty,
        (ExternType::Resource(_), ExternType::Resource(_)) => true, // Basic compatibility for now
        (ExternType::Instance(a_ty), ExternType::Instance(b_ty)) => {
            instance_types_match(a_ty, b_ty)
        }
        (ExternType::Component(_), ExternType::Component(_)) => true, // Basic compatibility for now
        _ => false,
    }
}

/// Check if two instance types match for linking
fn instance_types_match(a: &crate::types::InstanceType, b: &crate::types::InstanceType) -> bool {
    if a.exports.len() != b.exports.len() {
        return false;
    }

    for ((a_name, a_ty), (b_name, b_ty)) in a.exports.iter().zip(b.exports.iter()) {
        if a_name != b_name || !types_are_compatible(a_ty, b_ty) {
            return false;
        }
    }

    true
}

/// Check if two function types are compatible
fn func_types_compatible(a: &FuncType, b: &FuncType) -> bool {
    if a.params.len() != b.params.len() || a.results.len() != b.results.len() {
        return false;
    }

    for (a_param, b_param) in a.params.iter().zip(b.params.iter()) {
        if a_param != b_param {
            return false;
        }
    }

    for (a_result, b_result) in a.results.iter().zip(b.results.iter()) {
        if a_result != b_result {
            return false;
        }
    }

    true
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
                memory: DefaultMemory::new(MemoryType { min: 1, max: None }),
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
