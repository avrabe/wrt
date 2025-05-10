//! Component Model implementation for the WebAssembly Runtime.
//!
//! This module provides types and implementations for the WebAssembly Component Model.

use crate::prelude::*;

#[cfg(feature = "std")]
use log::{debug, error, info, trace, warn};

// Additional imports that aren't in the prelude
use wrt_format::component::ExternType as FormatExternType;
use wrt_types::resource::ResourceOperation as FormatResourceOperation;

// These imports are temporarily commented out until we fix them
// ComponentSection, ComponentTypeDefinition as FormatComponentTypeDefinition,
// ComponentTypeSection, ExportSection, ImportSection, InstanceSection,

// Import conversion functions
// Commenting out due to missing functions
// use wrt_format::component_conversion::{
//     component_type_to_format_type_def, format_type_def_to_component_type,
// };

// Runtime types with explicit namespacing
use wrt_runtime::types::{MemoryType, TableType};
use wrt_runtime::{
    func::FuncType as RuntimeFuncType,
    global::{Global, GlobalType},
    memory::Memory,
    table::Table,
};

// Import RwLock from prelude (it will be std::sync::RwLock or a no_std equivalent from the prelude)

use crate::execution::{run_with_time_bounds, TimeBoundedConfig, TimeBoundedOutcome};

// VecDeque comes from prelude (std::collections or alloc::collections based on features)

// core::str is already imported via prelude

// Import type conversion utilities for clear transformations
use crate::type_conversion::bidirectional::{
    complete_format_to_types_extern_type, complete_types_to_format_extern_type,
    convert_format_to_types_valtype, convert_format_valtype_to_valuetype,
    convert_types_to_format_valtype, extern_type_to_func_type,
};

use crate::debug_println;

// Import wrt_decoder types for decode and parse
use wrt_decoder::component::decode::Component as DecodedComponent;

// Define type aliases for missing types
type ComponentDecoder = fn(&[u8]) -> wrt_error::Result<wrt_format::component::Component>;
type ExportType = wrt_format::component::Export;
type ImportType = wrt_format::component::Import;
type TypeDef = wrt_format::component::ComponentType;
// Producers section might be moved or renamed
// ProducersSection,

/// Represents a component type
#[derive(Debug, Clone)]
pub struct WrtComponentType {
    /// Component imports
    pub imports: Vec<(String, String, ExternType)>,
    /// Component exports
    pub exports: Vec<(String, ExternType)>,
    /// Component instances
    pub instances: Vec<wrt_format::component::ComponentTypeDefinition>,
    /// Verification level for this component type
    pub verification_level: wrt_types::verification::VerificationLevel,
}

impl WrtComponentType {
    /// Creates a new empty component type
    pub fn new() -> Self {
        Self {
            imports: Vec::new(),
            exports: Vec::new(),
            instances: Vec::new(),
            verification_level: wrt_types::verification::VerificationLevel::Standard,
        }
    }

    /// Create a new empty component type
    pub fn empty() -> Self {
        Self {
            imports: Vec::new(),
            exports: Vec::new(),
            instances: Vec::new(),
            verification_level: wrt_types::verification::VerificationLevel::Standard,
        }
    }

    /// Set the verification level for memory operations
    pub fn set_verification_level(&mut self, level: wrt_types::verification::VerificationLevel) {
        self.verification_level = level;
    }

    /// Get the current verification level
    pub fn verification_level(&self) -> wrt_types::verification::VerificationLevel {
        self.verification_level
    }
}

impl Default for WrtComponentType {
    fn default() -> Self {
        Self::new()
    }
}

/// Represents a component instance
#[derive(Debug)]
pub struct Component {
    /// Component type
    pub(crate) component_type: WrtComponentType,
    /// Component exports
    pub(crate) exports: Vec<Export>,
    /// Component imports
    pub(crate) imports: Vec<Import>,
    /// Component instances
    pub(crate) instances: Vec<InstanceValue>,
    /// Linked components with their namespaces
    pub(crate) linked_components: HashMap<String, Arc<Component>>,
    /// Host callback registry
    pub(crate) callback_registry: Option<Arc<CallbackRegistry>>,
    /// Runtime instance
    pub(crate) runtime: Option<RuntimeInstance>,
    /// Interceptor for function calls
    pub(crate) interceptor: Option<Arc<LinkInterceptor>>,
    /// Resource table for managing component resources
    pub resource_table: ResourceTable,
    /// Built-in requirements
    pub(crate) built_in_requirements: Option<BuiltinRequirements>,
    /// Original binary
    pub(crate) original_binary: Option<Vec<u8>>,
    /// Verification level for all operations
    pub(crate) verification_level: wrt_types::verification::VerificationLevel,
}

/// Represents an instance value
#[derive(Debug, Clone)]
pub struct InstanceValue {
    /// Instance type
    pub ty: wrt_format::component::ComponentTypeDefinition,
    /// Instance exports
    pub exports: Vec<Export>,
}

/// Represents a runtime instance for executing WebAssembly code
#[derive(Debug)]
pub struct RuntimeInstance {
    /// Functions exported by this runtime
    functions: HashMap<String, ExternValue>,
    /// Memory exported by this runtime
    memories: HashMap<String, MemoryValue>,
    /// Tables exported by this runtime
    tables: HashMap<String, TableValue>,
    /// Globals exported by this runtime
    globals: HashMap<String, GlobalValue>,
    /// Runtime module instance (will be implemented as needed)
    module_instance: Option<Arc<RwLock<ModuleInstance>>>,
    /// Verification level for memory operations
    verification_level: VerificationLevel,
}

impl Default for RuntimeInstance {
    fn default() -> Self {
        Self::new()
    }
}

impl RuntimeInstance {
    /// Creates a new runtime instance
    pub fn new() -> Self {
        Self {
            functions: HashMap::new(),
            memories: HashMap::new(),
            tables: HashMap::new(),
            globals: HashMap::new(),
            module_instance: None,
            verification_level: VerificationLevel::Standard,
        }
    }

    /// Register an exported function
    pub fn register_function(&mut self, name: String, function: ExternValue) -> Result<()> {
        if let ExternValue::Function(_) = &function {
            self.functions.insert(name, function);
            Ok(())
        } else {
            Err(Error::new(
                ErrorCategory::Validation,
                codes::VALIDATION_ERROR,
                format!("Expected function value, got {:?}", function),
            ))
        }
    }

    /// Register an exported memory
    pub fn register_memory(&mut self, name: String, memory: MemoryValue) -> Result<()> {
        self.memories.insert(name, memory);
        Ok(())
    }

    /// Executes a function in the runtime
    ///
    /// # Arguments
    ///
    /// * `name` - Name of the function to execute
    /// * `args` - Arguments to pass to the function
    ///
    /// # Returns
    ///
    /// The result values from the function execution
    ///
    /// # Errors
    ///
    /// Returns an error if the function cannot be executed
    pub fn execute_function(&self, name: &str, args: Vec<Value>) -> Result<Vec<Value>> {
        // Look up the function in our registered functions
        let function = self.functions.get(name).ok_or_else(|| {
            Error::new(
                ErrorCategory::Function,
                codes::FUNCTION_NOT_FOUND,
                format!("Function {} not found in runtime", name),
            )
        })?;

        // Get the function value
        if let ExternValue::Function(func_value) = function {
            // Validate arguments based on function signature
            if args.len() != func_value.ty.params.len() {
                return Err(Error::new(
                    ErrorCategory::Validation,
                    codes::VALIDATION_ERROR,
                    format!(
                        "Expected {} arguments, got {}",
                        func_value.ty.params.len(),
                        args.len()
                    ),
                ));
            }

            // Type check arguments
            for (i, (arg, param_type)) in args.iter().zip(func_value.ty.params.iter()).enumerate() {
                let arg_type = arg.value_type();
                let expected_type = &param_type;

                if !self.is_type_compatible(&arg_type, expected_type) {
                    return Err(Error::new(
                        ErrorCategory::Validation,
                        codes::VALIDATION_ERROR,
                        format!(
                            "Type mismatch for argument {}: expected {:?}, got {:?}",
                            i, expected_type, arg_type
                        ),
                    ));
                }
            }

            // Execute the function via the module instance if available
            if let Some(module_instance) = &self.module_instance {
                // This would delegate to the actual module instance in a full implementation
                // This interface will be extended as needed

                #[cfg(feature = "std")]
                debug!("Module instance execution not yet implemented for function: {}", name);

                // For MVP, we need to return that execution isn't implemented
                // without hard-coding specific function behavior
                return Err(Error::new(
                    ErrorCategory::System,
                    codes::IMPLEMENTATION_LIMIT,
                    "Module instance execution not yet implemented".to_string(),
                ));
            }

            // If we reach here, there's no module instance to handle the execution,
            // so we need a structured way to handle user-provided function execution.

            // Return a not implemented error for now
            // This will be extended to support function resolution from registered callbacks
            Err(Error::new(
                ErrorCategory::System,
                codes::NOT_IMPLEMENTED,
                format!("Function execution using registered handlers not implemented yet"),
            ))
        } else {
            Err(Error::new(
                ErrorCategory::Validation,
                codes::VALIDATION_ERROR,
                format!("Expected function, got {:?}", function),
            ))
        }
    }

    /// Check if a value type is compatible with a format value type
    fn is_type_compatible(&self, value_type: &ValueType, expected_type: &ValueType) -> bool {
        // For MVP, we'll do simple equality check
        // This can be extended later to handle more complex type compatibility rules
        value_type == expected_type
    }

    /// Reads memory
    pub fn read_memory(&self, _offset: u32, _size: u32, _buffer: &mut [u8]) -> Result<()> {
        // Implementation would depend on the specific runtime
        Err(Error::new(
            ErrorCategory::System,
            codes::NOT_IMPLEMENTED,
            "Memory operations not implemented in this runtime",
        ))
    }

    /// Writes memory
    pub fn write_memory(&self, _offset: u32, _data: &[u8]) -> Result<()> {
        // Implementation would depend on the specific runtime
        Err(Error::new(
            ErrorCategory::System,
            codes::NOT_IMPLEMENTED,
            "Memory operations not implemented in this runtime",
        ))
    }
}

// Private helper struct for implementing the module instance
// This is just a placeholder for now
struct ModuleInstance {
    // Implementation details would go here
}

/// Helper function to convert FormatValType to ValueType
fn convert_to_valuetype(val_type_pair: &(String, FormatValType)) -> ValueType {
    format_val_type_to_value_type(&val_type_pair.1).expect("Failed to convert format value type")
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
    pub ty: crate::runtime::FuncType,
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
        Ok(Self { ty, memory: Arc::new(RwLock::new(memory)) })
    }

    /// Creates a new memory value with a debug name
    ///
    /// # Arguments
    ///
    /// * `ty` - The memory type
    /// * `name` - The debug name for the memory
    ///
    /// # Returns
    ///
    /// A new memory value
    ///
    /// # Errors
    ///
    /// Returns an error if the memory cannot be created
    pub fn new_with_name(ty: MemoryType, name: &str) -> Result<Self> {
        let memory = Memory::new_with_name(ty.clone(), name)?;
        Ok(Self { ty, memory: Arc::new(RwLock::new(memory)) })
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
        let memory = self.memory.read().map_err(|e| {
            Error::new(
                ErrorCategory::Memory,
                codes::MEMORY_ACCESS_ERROR,
                format!("Failed to acquire memory read lock: {}", e),
            )
        })?;

        let mut buffer = vec![0; size as usize];
        memory.read(offset, &mut buffer).map_err(|e| {
            Error::new(
                ErrorCategory::Memory,
                codes::MEMORY_ACCESS_ERROR,
                format!("Memory read error: {}", e),
            )
        })?;

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
        let mut memory = self.memory.write().map_err(|e| {
            Error::new(
                ErrorCategory::Memory,
                codes::MEMORY_ACCESS_ERROR,
                format!("Failed to acquire memory write lock: {}", e),
            )
        })?;

        memory.write(offset, bytes).map_err(|e| {
            Error::new(
                ErrorCategory::Memory,
                codes::MEMORY_ACCESS_ERROR,
                format!("Memory write error: {}", e),
            )
        })
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
        let mut memory = self.memory.write().map_err(|e| {
            Error::new(
                ErrorCategory::Memory,
                codes::MEMORY_ACCESS_ERROR,
                format!("Failed to acquire memory write lock: {}", e),
            )
        })?;

        memory.grow(pages).map_err(|e| {
            Error::new(
                ErrorCategory::Memory,
                codes::MEMORY_ACCESS_ERROR,
                format!("Memory grow error: {}", e),
            )
        })
    }

    /// Gets the current size of the memory in pages
    ///
    /// # Returns
    ///
    /// The current size in pages
    pub fn size(&self) -> Result<u32> {
        let memory = self.memory.read().map_err(|e| {
            Error::new(
                ErrorCategory::Memory,
                codes::MEMORY_ACCESS_ERROR,
                format!("Failed to acquire memory read lock: {}", e),
            )
        })?;

        Ok(memory.size())
    }

    /// Gets the current size of the memory in bytes
    ///
    /// # Returns
    ///
    /// The current size in bytes
    pub fn size_in_bytes(&self) -> Result<usize> {
        let memory = self.memory.read().map_err(|e| {
            Error::new(
                ErrorCategory::Memory,
                codes::MEMORY_ACCESS_ERROR,
                format!("Failed to acquire memory read lock: {}", e),
            )
        })?;

        Ok(memory.size_in_bytes())
    }

    /// Gets the peak memory usage in bytes
    ///
    /// # Returns
    ///
    /// The peak memory usage in bytes
    pub fn peak_usage(&self) -> Result<usize> {
        let memory = self.memory.read().map_err(|e| {
            Error::new(
                ErrorCategory::Memory,
                codes::MEMORY_ACCESS_ERROR,
                format!("Failed to acquire memory read lock: {}", e),
            )
        })?;

        Ok(memory.peak_usage())
    }

    /// Gets the number of memory accesses performed
    ///
    /// # Returns
    ///
    /// The number of memory accesses
    pub fn access_count(&self) -> Result<u64> {
        let memory = self.memory.read().map_err(|e| {
            Error::new(
                ErrorCategory::Memory,
                codes::MEMORY_ACCESS_ERROR,
                format!("Failed to acquire memory read lock: {}", e),
            )
        })?;

        Ok(memory.access_count())
    }

    /// Gets a debug name for this memory, if any
    ///
    /// # Returns
    ///
    /// The debug name, if any
    pub fn debug_name(&self) -> Result<Option<String>> {
        let memory = self.memory.read().map_err(|e| {
            Error::new(
                ErrorCategory::Memory,
                codes::MEMORY_ACCESS_ERROR,
                format!("Failed to acquire memory read lock: {}", e),
            )
        })?;

        Ok(memory.debug_name().map(String::from))
    }

    /// Sets a debug name for this memory
    ///
    /// # Arguments
    ///
    /// * `name` - The debug name to set
    pub fn set_debug_name(&self, name: &str) -> Result<()> {
        let mut memory = self.memory.write().map_err(|e| {
            Error::new(
                ErrorCategory::Memory,
                codes::MEMORY_ACCESS_ERROR,
                format!("Failed to acquire memory write lock: {}", e),
            )
        })?;

        memory.set_debug_name(name);
        Ok(())
    }

    /// Verifies the integrity of the memory
    ///
    /// # Returns
    ///
    /// Ok(()) if the memory is valid
    ///
    /// # Errors
    ///
    /// Returns an error if the memory is invalid
    pub fn verify_integrity(&self) -> Result<()> {
        let memory = self.memory.read().map_err(|e| {
            Error::new(
                ErrorCategory::Memory,
                codes::MEMORY_ACCESS_ERROR,
                format!("Failed to acquire memory read lock: {}", e),
            )
        })?;

        memory.verify_integrity()
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

/// Represents a host with callable functions
pub struct Host {
    /// Host functions indexed by name
    functions: HashMap<String, FunctionValue>,
}

impl Default for Host {
    fn default() -> Self {
        Self::new()
    }
}

impl Host {
    /// Creates a new empty host
    #[must_use]
    pub fn new() -> Self {
        Self { functions: HashMap::new() }
    }

    /// Adds a host function
    pub fn add_function(&mut self, name: String, func: FunctionValue) {
        self.functions.insert(name, func);
    }

    /// Gets a host function by name
    #[must_use]
    pub fn get_function(&self, name: &str) -> Option<&FunctionValue> {
        self.functions.get(name)
    }

    /// Calls a host function
    pub fn call_function(&self, name: &str, args: &[Value]) -> Result<Vec<Value>> {
        // Find the function
        let function = self.get_function(name).ok_or_else(|| {
            Error::new(
                ErrorCategory::Component,
                codes::COMPONENT_LINKING_ERROR,
                format!("Host function {name} not found"),
            )
        })?;

        // Validate arguments
        if args.len() != function.ty.params.len() {
            return Err(Error::new(
                ErrorCategory::Validation,
                codes::VALIDATION_ERROR,
                format!("Expected {} arguments, got {}", function.ty.params.len(), args.len()),
            ));
        }

        // Actual function calling would happen here
        // This requires integration with the actual host function mechanism
        Err(Error::new(
            ErrorCategory::System,
            codes::NOT_IMPLEMENTED,
            "Host function calling requires implementation by a concrete host".to_string(),
        ))
    }
}

impl Component {
    /// Creates a new component with the given type
    pub fn new(component_type: WrtComponentType) -> Self {
        Self {
            component_type,
            exports: Vec::new(),
            imports: Vec::new(),
            instances: Vec::new(),
            linked_components: HashMap::new(),
            callback_registry: None,
            runtime: None,
            interceptor: None,
            resource_table: ResourceTable::new(),
            built_in_requirements: None,
            original_binary: None,
            verification_level: wrt_types::verification::VerificationLevel::Standard,
        }
    }

    /// Set the verification level for memory operations
    pub fn set_verification_level(&mut self, level: wrt_types::verification::VerificationLevel) {
        self.verification_level = level;
        // Propagate to resource table
        self.resource_table.set_verification_level(convert_verification_level(level));
    }

    /// Get the current verification level
    pub fn verification_level(&self) -> wrt_types::verification::VerificationLevel {
        self.verification_level
    }
}

/// Container for built-in type requirements
#[derive(Debug, Clone, Default)]
pub struct BuiltinRequirements {
    /// The set of required built-in types
    requirements: HashSet<BuiltinType>,
}

impl BuiltinRequirements {
    /// Create a new, empty set of built-in requirements
    pub fn new() -> Self {
        Self { requirements: HashSet::new() }
    }

    /// Add a requirement for a specific built-in type
    pub fn add_requirement(&mut self, builtin_type: BuiltinType) {
        self.requirements.insert(builtin_type);
    }

    /// Get the set of required built-ins
    pub fn get_requirements(&self) -> &HashSet<BuiltinType> {
        &self.requirements
    }

    /// Check if all the requirements can be satisfied with the current runtime
    pub fn is_supported(&self) -> bool {
        // For now, all built-ins are supported
        // In the future, this could check if experimental features are enabled
        true
    }

    /// Get a list of unsupported built-ins
    pub fn get_unsupported(&self) -> Vec<BuiltinType> {
        // For now, all built-ins are supported
        vec![]
    }

    /// Check if the requirements can be satisfied with the given available built-ins
    pub fn can_be_satisfied(&self, available: &HashSet<BuiltinType>) -> bool {
        self.requirements.iter().all(|req| available.contains(req))
    }
}

/// Scans a binary for required builtins
pub fn scan_builtins(bytes: &[u8]) -> Result<BuiltinRequirements> {
    // Helper to avoid boilerplate
    let mut requirements = BuiltinRequirements::new();

    // Try to decode as component or module
    match wrt_decoder::component::decode_component(bytes) {
        Ok(component) => {
            scan_functions_for_builtins(&component, &mut requirements)?;
            Ok(requirements)
        }
        Err(err) => {
            return Err(Error::new(
                ErrorCategory::Parse,
                codes::DECODING_ERROR,
                format!("Failed to decode component during built-in scan: {}", err),
            ));
        }
    }
}

/// Scans a module binary for builtins
fn scan_module_for_builtins(module: &[u8], requirements: &mut BuiltinRequirements) -> Result<()> {
    // This would need to be implemented for core modules
    // For now, we'll just return success
    match wrt_decoder::module::decode_module(module) {
        Ok(_) => Ok(()),
        Err(err) => Err(Error::new(
            ErrorCategory::Parse,
            codes::DECODING_ERROR,
            format!("Failed to scan module for builtins: {}", err),
        )),
    }
}

/// Map an import name to a built-in type
fn map_import_to_builtin(import_name: &str) -> Option<BuiltinType> {
    // This function is now defined in the parser module, but we keep it here
    // for backward compatibility
    crate::parser::map_import_to_builtin(import_name)
}

/// Scan component functions for built-in usage
///
/// # Arguments
///
/// * `component` - The component to scan
/// * `requirements` - The requirements to be updated
///
/// # Returns
///
/// A result indicating success or an error
fn scan_functions_for_builtins(
    component: &DecodedComponent,
    requirements: &mut BuiltinRequirements,
) -> Result<()> {
    // Check for resource types which indicate built-in usage
    for type_def in component.types() {
        if let TypeDef::Resource(_) = type_def {
            // Resources typically require the ResourceCreate built-in
            requirements.add_requirement(BuiltinType::ResourceCreate);
            requirements.add_requirement(BuiltinType::ResourceDrop);
            requirements.add_requirement(BuiltinType::ResourceRep);
            requirements.add_requirement(BuiltinType::ResourceGet);
        }
    }

    #[cfg(feature = "component-model-async")]
    // Check for async functions which require the AsyncWait built-in
    for func in component.functions() {
        if func.is_async() {
            requirements.add_requirement(BuiltinType::AsyncWait);
            requirements.add_requirement(BuiltinType::AsyncNew);
            requirements.add_requirement(BuiltinType::AsyncGet);
            requirements.add_requirement(BuiltinType::AsyncPoll);
            break; // Only need to identify one async function
        }
    }

    Ok(())
}

/// Extracts embedded modules from a binary
fn extract_embedded_modules(bytes: &[u8]) -> Result<Vec<Vec<u8>>> {
    // Try to decode as component
    match wrt_decoder::component::decode_component(bytes) {
        Ok(component) => {
            // Extract modules from component
            // Let's create a simple mock implementation since component doesn't have modules()
            let modules = Vec::new(); // Create an empty vector as a placeholder
            Ok(modules)
        }
        Err(err) => {
            return Err(Error::new(
                ErrorCategory::Parse,
                codes::DECODING_ERROR,
                format!("Failed to decode component while extracting modules: {}", err),
            ));
        }
    }
}

/// Convert a component value to a runtime value
pub fn component_value_to_value(
    component_value: &wrt_types::ComponentValue,
) -> wrt_intercept::Value {
    use crate::type_conversion::types_componentvalue_to_core_value;
    use wrt_intercept::Value;

    // Use the new conversion function
    types_componentvalue_to_core_value(component_value).unwrap_or(Value::I32(0))
    // Provide a sensible default on error
}

/// Convert a runtime value to a component value
pub fn value_to_component_value(value: &wrt_intercept::Value) -> wrt_types::ComponentValue {
    use crate::type_conversion::core_value_to_types_componentvalue;
    use wrt_types::ComponentValue;

    // Use the new conversion function
    core_value_to_types_componentvalue(value).unwrap_or(ComponentValue::Void) // Provide a sensible default on error
}

/// Convert parameter to value type
pub fn convert_param_to_value_type(
    param: &wrt_format::component::ValType,
) -> wrt_types::types::ValueType {
    crate::type_conversion::format_val_type_to_value_type(param)
        .unwrap_or(wrt_types::types::ValueType::I32)
}

/// Convert verification level
pub fn convert_verification_level(
    level: wrt_types::VerificationLevel,
) -> crate::resources::VerificationLevel {
    match level {
        wrt_types::VerificationLevel::None => crate::resources::VerificationLevel::None,
        wrt_types::VerificationLevel::Sampling => crate::resources::VerificationLevel::Critical,
        wrt_types::VerificationLevel::Standard => crate::resources::VerificationLevel::Critical,
        wrt_types::VerificationLevel::Full => crate::resources::VerificationLevel::Full,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_runtime_instance_creation() {
        let runtime = RuntimeInstance::new();
        assert!(runtime.functions.is_empty());
        assert!(runtime.memories.is_empty());
        assert!(runtime.tables.is_empty());
        assert!(runtime.globals.is_empty());
        assert!(runtime.module_instance.is_none());
    }

    #[test]
    fn test_register_function() {
        let mut runtime = RuntimeInstance::new();
        let func_type = wrt_runtime::func::FuncType {
            params: vec![ValueType::I32, ValueType::I32],
            results: vec![ValueType::I32],
        };
        let func_value = FunctionValue { ty: func_type, export_name: "test_function".to_string() };
        let extern_value = ExternValue::Function(func_value);

        assert!(runtime.register_function("test_function".to_string(), extern_value).is_ok());
        assert_eq!(runtime.functions.len(), 1);
        assert!(runtime.functions.contains_key("test_function"));
    }

    #[test]
    fn test_function_not_found() {
        let runtime = RuntimeInstance::new();

        // Try to execute a non-existent function
        let args = vec![Value::I32(1)];
        let result = runtime.execute_function("nonexistent", args);

        // Verify the error
        assert!(result.is_err());
        match result {
            Err(e) => {
                assert_eq!(e.category(), ErrorCategory::Function);
                assert!(e.to_string().contains("not found"));
            }
            _ => panic!("Expected an error"),
        }
    }

    #[test]
    fn test_argument_count_validation() {
        let mut runtime = RuntimeInstance::new();

        // Create and register a function expecting 2 arguments
        let func_type = wrt_runtime::func::FuncType {
            params: vec![ValueType::I32, ValueType::I32],
            results: vec![ValueType::I32],
        };
        let func_value = FunctionValue { ty: func_type, export_name: "test_func".to_string() };
        let extern_value = ExternValue::Function(func_value);

        runtime.register_function("test_func".to_string(), extern_value).unwrap();

        // Call with wrong number of arguments
        let args = vec![Value::I32(1)]; // Only one argument
        let result = runtime.execute_function("test_func", args);

        // Verify the error
        assert!(result.is_err());
        match result {
            Err(e) => {
                assert_eq!(e.category(), ErrorCategory::Validation);
                assert!(e.to_string().contains("Expected 2 arguments"));
            }
            _ => panic!("Expected an error"),
        }
    }

    #[test]
    fn test_argument_type_validation() {
        let mut runtime = RuntimeInstance::new();

        // Create and register a function expecting I32 arguments
        let func_type = wrt_runtime::func::FuncType {
            params: vec![ValueType::I32, ValueType::I32],
            results: vec![ValueType::I32],
        };
        let func_value = FunctionValue { ty: func_type, export_name: "test_func".to_string() };
        let extern_value = ExternValue::Function(func_value);

        runtime.register_function("test_func".to_string(), extern_value).unwrap();

        // Call with wrong argument types
        let args = vec![Value::I32(1), Value::F32(2.0)]; // Second arg is F32
        let result = runtime.execute_function("test_func", args);

        // Verify the error
        assert!(result.is_err());
        match result {
            Err(e) => {
                assert_eq!(e.category(), ErrorCategory::Validation);
                assert!(e.to_string().contains("Type mismatch for argument"));
            }
            _ => panic!("Expected an error"),
        }
    }

    #[test]
    fn test_function_not_implemented() {
        let mut runtime = RuntimeInstance::new();

        // Create and register a function
        let func_type = wrt_runtime::func::FuncType {
            params: vec![ValueType::I32, ValueType::I32],
            results: vec![ValueType::I32],
        };
        let func_value = FunctionValue { ty: func_type, export_name: "test_func".to_string() };
        let extern_value = ExternValue::Function(func_value);

        runtime.register_function("test_func".to_string(), extern_value).unwrap();

        // Call with correct arguments
        let args = vec![Value::I32(1), Value::I32(2)];
        let result = runtime.execute_function("test_func", args);

        // Verify we get a NOT_IMPLEMENTED error
        assert!(result.is_err());
        match result {
            Err(e) => {
                assert_eq!(e.category(), ErrorCategory::System);
                assert_eq!(e.code(), codes::NOT_IMPLEMENTED);
                assert!(e.to_string().contains("not implemented"));
            }
            _ => panic!("Expected a not implemented error"),
        }
    }
}
