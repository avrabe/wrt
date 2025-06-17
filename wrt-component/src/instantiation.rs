//! Component instantiation and linking
//!
//! This module implements the instantiation process for WebAssembly components,
//! including import resolution, export extraction, and instance isolation.

#[cfg(not(feature = "std"))]
use core::{fmt, mem};
#[cfg(feature = "std")]
use std::{fmt, mem};

#[cfg(all(feature = "std", feature = "safety-critical"))]
use wrt_foundation::allocator::{WrtHashMap, WrtVec, CrateId};
#[cfg(all(feature = "std", not(feature = "safety-critical")))]
use std::{boxed::Box, collections::BTreeMap, string::String, vec::Vec};

use wrt_foundation::{
    bounded::BoundedVec, component::ComponentType, component_value::ComponentValue, prelude::*,
    budget_aware_provider::CrateId,
};

use crate::{
    canonical_abi::canonical::CanonicalABI,
    components::component::{Component, ComponentInstance},
    execution_engine::ComponentExecutionEngine,
    export::Export,
    import::Import,
    resources::resource_lifecycle::ResourceLifecycleManager,
    types::{ValType, Value},
    WrtResult,
};

/// Maximum number of imports in no_std environments
const MAX_IMPORTS: usize = 256;
/// Maximum number of exports in no_std environments  
const MAX_EXPORTS: usize = 256;
/// Maximum number of instances in no_std environments
const MAX_INSTANCES: usize = 64;

/// Import value provided during instantiation
#[derive(Debug, Clone)]
pub enum ImportValue {
    /// A function import
    Function(FunctionImport),
    /// A value import (global, memory, table)
    Value(ComponentValue),
    /// An instance import
    Instance(InstanceImport),
    /// A type import
    Type(ComponentType),
}

/// Function import descriptor
#[derive(Debug, Clone)]
pub struct FunctionImport {
    /// Function signature
    pub signature: ComponentType,
    /// Function implementation
    #[cfg(feature = "std")]
    pub implementation: Box<dyn Fn(&[Value]) -> WrtResult<Value>>,
    #[cfg(not(any(feature = "std", )))]
    pub implementation: fn(&[Value]) -> WrtResult<Value>,
}

/// Instance import descriptor
#[derive(Debug, Clone)]
pub struct InstanceImport {
    /// Instance exports
    #[cfg(all(feature = "std", feature = "safety-critical"))]
    pub exports: WrtHashMap<String, ExportValue, {CrateId::Component as u8}, 256>,
    #[cfg(all(feature = "std", not(feature = "safety-critical")))]
    pub exports: BTreeMap<String, ExportValue>,
    #[cfg(not(any(feature = "std", )))]
    pub exports: BoundedVec<(BoundedString<64, NoStdProvider<65536>>, ExportValue), MAX_EXPORTS>,
}

/// Export value from an instance
#[derive(Debug, Clone)]
pub enum ExportValue {
    /// A function export
    Function(FunctionExport),
    /// A value export
    Value(ComponentValue),
    /// An instance export
    Instance(Box<ComponentInstance>),
    /// A type export
    Type(ComponentType),
}

/// Function export descriptor
#[derive(Debug, Clone)]
pub struct FunctionExport {
    /// Function signature
    pub signature: ComponentType,
    /// Function index in the instance
    pub index: u32,
}

/// Import values provided during instantiation
pub struct ImportValues {
    /// Map of import names to values
    #[cfg(all(feature = "std", feature = "safety-critical"))]
    imports: WrtHashMap<String, ImportValue, {CrateId::Component as u8}, 256>,
    #[cfg(all(feature = "std", not(feature = "safety-critical")))]
    imports: BTreeMap<String, ImportValue>,
    #[cfg(not(any(feature = "std", )))]
    imports: BoundedVec<(BoundedString<64, NoStdProvider<65536>>, ImportValue), MAX_IMPORTS>,
}

impl ImportValues {
    /// Create new import values
    pub fn new() -> Self {
        Self {
            #[cfg(all(feature = "std", feature = "safety-critical"))]
            imports: WrtHashMap::new(),
            #[cfg(all(feature = "std", not(feature = "safety-critical")))]
            imports: BTreeMap::new(),
            #[cfg(not(any(feature = "std", )))]
            imports: BoundedVec::new(NoStdProvider::<65536>::default()).unwrap(),
        }
    }

    /// Add an import value
    #[cfg(all(feature = "std", feature = "safety-critical"))]
    pub fn add(&mut self, name: String, value: ImportValue) -> WrtResult<()> {
        self.imports.insert(name, value).map_err(|_| {
            wrt_foundation::Error::new(
                wrt_foundation::ErrorCategory::Resource,
                wrt_error::codes::RESOURCE_EXHAUSTED,
                "Too many imports (limit: 256)"
            )
        })
    }
    
    /// Add an import value
    #[cfg(all(feature = "std", not(feature = "safety-critical")))]
    pub fn add(&mut self, name: String, value: ImportValue) -> WrtResult<()> {
        self.imports.insert(name, value);
        Ok(())
    }

    /// Add an import value (no_std version)
    #[cfg(not(any(feature = "std", )))]
    pub fn add(&mut self, name: BoundedString<64, NoStdProvider<65536>>, value: ImportValue) -> WrtResult<()> {
        self.imports
            .push((name, value))
            .map_err(|_| wrt_foundation::Error::new(wrt_foundation::ErrorCategory::Resource, wrt_error::codes::RESOURCE_EXHAUSTED, "Too many imports"))
    }

    /// Get an import value by name
    #[cfg(all(feature = "std", feature = "safety-critical"))]
    pub fn get(&self, name: &str) -> Option<&ImportValue> {
        self.imports.get(name)
    }
    
    /// Get an import value by name
    #[cfg(all(feature = "std", not(feature = "safety-critical")))]
    pub fn get(&self, name: &str) -> Option<&ImportValue> {
        self.imports.get(name)
    }

    /// Get an import value by name (no_std version)
    #[cfg(not(any(feature = "std", )))]
    pub fn get(&self, name: &str) -> Option<&ImportValue> {
        self.imports.iter().find(|(n, _)| n.as_str() == name).map(|(_, v)| v)
    }
}

impl Default for ImportValues {
    fn default() -> Self {
        Self::new()
    }
}

/// Component instantiation context
pub struct InstantiationContext {
    /// Canonical ABI processor
    pub canonical_abi: CanonicalAbi,
    /// Resource lifecycle manager
    pub resource_manager: ResourceLifecycleManager,
    /// Execution engine
    pub execution_engine: ComponentExecutionEngine,
    /// Instance counter for unique IDs
    next_instance_id: u32,
}

impl InstantiationContext {
    /// Create a new instantiation context
    pub fn new() -> Self {
        Self {
            canonical_abi: CanonicalAbi::new(),
            resource_manager: ResourceLifecycleManager::new(),
            execution_engine: ComponentExecutionEngine::new(),
            next_instance_id: 0,
        }
    }

    /// Get the next instance ID
    fn next_instance_id(&mut self) -> u32 {
        let id = self.next_instance_id;
        self.next_instance_id += 1;
        id
    }
}

impl Default for InstantiationContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Component instantiation implementation
impl Component {
    /// Instantiate a component with the provided imports
    pub fn instantiate(
        &self,
        imports: &ImportValues,
        context: &mut InstantiationContext,
    ) -> WrtResult<ComponentInstance> {
        // Generate unique instance ID
        let instance_id = context.next_instance_id();

        // Step 1: Validate imports match component requirements
        self.validate_imports(imports)?;

        // Step 2: Create resource tables for this instance
        let resource_tables = self.create_resource_tables()?;

        // Step 3: Resolve imports into concrete values
        let resolved_imports = self.resolve_imports(imports, context)?;

        // Step 4: Initialize embedded modules (if any)
        let module_instances = self.initialize_modules(&resolved_imports, context)?;

        // Step 5: Extract and validate exports
        let exports = self.extract_exports(&module_instances, context)?;

        // Step 6: Create the component instance
        let instance = ComponentInstance {
            id: instance_id,
            component: self.clone(),
            #[cfg(all(feature = "std", feature = "safety-critical"))]
            imports: resolved_imports,
            #[cfg(all(feature = "std", feature = "safety-critical"))]
            exports,
            #[cfg(all(feature = "std", feature = "safety-critical"))]
            resource_tables,
            #[cfg(all(feature = "std", feature = "safety-critical"))]
            module_instances,
            #[cfg(all(feature = "std", not(feature = "safety-critical")))]
            imports: resolved_imports,
            #[cfg(all(feature = "std", not(feature = "safety-critical")))]
            exports,
            #[cfg(all(feature = "std", not(feature = "safety-critical")))]
            resource_tables,
            #[cfg(all(feature = "std", not(feature = "safety-critical")))]
            module_instances,
            #[cfg(not(any(feature = "std", )))]
            imports: BoundedVec::new(NoStdProvider::<65536>::default()).unwrap(),
            #[cfg(not(any(feature = "std", )))]
            exports: BoundedVec::new(NoStdProvider::<65536>::default()).unwrap(),
            #[cfg(not(any(feature = "std", )))]
            resource_tables: BoundedVec::new(NoStdProvider::<65536>::default()).unwrap(),
            #[cfg(not(any(feature = "std", )))]
            module_instances: BoundedVec::new(NoStdProvider::<65536>::default()).unwrap(),
        };

        Ok(instance)
    }

    /// Validate that provided imports match component requirements
    fn validate_imports(&self, imports: &ImportValues) -> WrtResult<()> {
        #[cfg(feature = "std")]
        {
            for import in &self.imports {
                match imports.get(&import.name) {
                    Some(value) => {
                        // Validate import type matches
                        self.validate_import_type(import, value)?;
                    }
                    None => {
                        return Err(wrt_foundation::Error::new(wrt_foundation::ErrorCategory::Validation, wrt_error::errors::codes::INVALID_INPUT, "Invalid input"));
                    }
                }
            }
        }
        #[cfg(not(any(feature = "std", )))]
        {
            // In no_std, we have limited validation
            // Just check that we have some imports if required
            if self.imports.len() > 0 && imports.imports.len() == 0 {
                return Err(wrt_foundation::Error::new(wrt_foundation::ErrorCategory::Validation, wrt_error::errors::codes::INVALID_INPUT, "Invalid input"));
            }
        }

        Ok(())
    }

    /// Validate that an import value matches the expected type
    fn validate_import_type(&self, import: &Import, value: &ImportValue) -> WrtResult<()> {
        match (&import.import_type, value) {
            (crate::import::ImportType::Function(expected), ImportValue::Function(actual)) => {
                // Check function signature compatibility
                if !self.is_function_compatible(expected, &actual.signature) {
                    return Err(wrt_foundation::Error::new(
                        wrt_foundation::ErrorCategory::Type,
                        wrt_error::codes::TYPE_MISMATCH_ERROR,
                        "Function import type mismatch"
                    ));
                }
            }
            (crate::import::ImportType::Value(expected), ImportValue::Value(actual)) => {
                // Check value type compatibility
                if !self.is_value_compatible(expected, actual) {
                    return Err(wrt_foundation::Error::new(
                        wrt_foundation::ErrorCategory::Type,
                        wrt_error::codes::TYPE_MISMATCH_ERROR,
                        "Value import type mismatch"
                    ));
                }
            }
            (crate::import::ImportType::Instance(_), ImportValue::Instance(_)) => {
                // Instance validation is more complex, simplified for now
                // TODO: Validate instance exports match expected interface
            }
            (crate::import::ImportType::Type(_), ImportValue::Type(_)) => {
                // Type validation
                // TODO: Implement type equality checking
            }
            _ => {
                return Err(wrt_foundation::Error::new(
                    wrt_foundation::ErrorCategory::Type,
                    wrt_error::codes::TYPE_MISMATCH_ERROR,
                    "Import kind mismatch"
                ));
            }
        }
        Ok(())
    }

    /// Check if function types are compatible
    fn is_function_compatible(&self, expected: &ComponentType, actual: &ComponentType) -> bool {
        // Check basic type equality for now
        // In a full implementation, this would check subtyping rules
        match (expected, actual) {
            (ComponentType::Unit, ComponentType::Unit) => true,
            // For other types, check structural equality
            _ => expected == actual,
        }
    }

    /// Check if value types are compatible
    fn is_value_compatible(&self, expected: &ComponentType, actual: &ComponentValue) -> bool {
        // Basic type compatibility check
        match (expected, actual) {
            (ComponentType::Unit, ComponentValue::Unit) => true,
            // For other types, this would need more complex checking
            _ => true, // Allow for now
        }
    }

    /// Create resource tables for the instance with budget enforcement
    #[cfg(all(feature = "std", feature = "safety-critical"))]
    fn create_resource_tables(&self) -> WrtResult<WrtVec<ResourceTable, {CrateId::Component as u8}, 16>> {
        let mut tables = WrtVec::new();

        // Create resource tables based on component types
        // For each resource type in the component, create a table
        for _type_id in 0..self.types.len().min(16) {
            // Create a budget-aware table for this resource type
            let table = ResourceTable::new()?;
            tables.push(table).map_err(|_| {
                wrt_foundation::Error::new(
                    wrt_foundation::ErrorCategory::Resource,
                    wrt_error::codes::RESOURCE_EXHAUSTED,
                    "Too many resource tables (limit: 16)"
                )
            })?;
        }

        Ok(tables)
    }
    
    /// Create resource tables for the instance
    #[cfg(all(feature = "std", not(feature = "safety-critical")))]
    fn create_resource_tables(&self) -> WrtResult<Vec<ResourceTable>> {
        let mut tables = Vec::new();

        // Create resource tables based on component types
        // For each resource type in the component, create a table
        for _type_id in 0..self.types.len() {
            // Create a budget-aware table for this resource type
            let table = ResourceTable::new()?;
            tables.push(table);
        }

        Ok(tables)
    }

    #[cfg(not(any(feature = "std", )))]
    fn create_resource_tables(&self) -> WrtResult<BoundedVec<ResourceTable, 16, crate::bounded_component_infra::ComponentProvider>> {
        use crate::bounded_component_infra::ComponentProvider;
        use wrt_foundation::safe_managed_alloc;
        
        let provider = safe_managed_alloc!(131072, CrateId::Component)?;
        let mut tables = BoundedVec::new(provider)?;

        // Create resource tables based on component types
        for _type_id in 0..self.types.len().min(16) {
            let table = ResourceTable::new()?;
            tables.push(table).map_err(|_| {
                wrt_foundation::Error::new(
                    wrt_foundation::ErrorCategory::Resource,
                    wrt_error::codes::RESOURCE_EXHAUSTED,
                    "Too many resource tables"
                )
            })?;
        }

        Ok(tables)
    }

    /// Resolve imports into concrete values
    #[cfg(all(feature = "std", feature = "safety-critical"))]
    fn resolve_imports(
        &self,
        imports: &ImportValues,
        context: &mut InstantiationContext,
    ) -> WrtResult<WrtVec<ResolvedImport, {CrateId::Component as u8}, 256>> {
        let mut resolved = WrtVec::new();

        for import in &self.imports {
            if let Some(value) = imports.get(&import.name) {
                let resolved_import = self.resolve_import(import, value, context)?;
                resolved.push(resolved_import).map_err(|_| {
                    wrt_foundation::Error::new(
                        wrt_foundation::ErrorCategory::Resource,
                        wrt_error::codes::RESOURCE_EXHAUSTED,
                        "Too many resolved imports (limit: 256)"
                    )
                })?;
            }
        }

        Ok(resolved)
    }
    
    /// Resolve imports into concrete values
    #[cfg(all(feature = "std", not(feature = "safety-critical")))]
    fn resolve_imports(
        &self,
        imports: &ImportValues,
        context: &mut InstantiationContext,
    ) -> WrtResult<Vec<ResolvedImport>> {
        let mut resolved = Vec::new();

        for import in &self.imports {
            if let Some(value) = imports.get(&import.name) {
                let resolved_import = self.resolve_import(import, value, context)?;
                resolved.push(resolved_import);
            }
        }

        Ok(resolved)
    }

    #[cfg(not(any(feature = "std", )))]
    fn resolve_imports(
        &self,
        imports: &ImportValues,
        context: &mut InstantiationContext,
    ) -> Wrtcore::result::Result<BoundedVec<ResolvedImport, MAX_IMPORTS, NoStdProvider<65536>>, NoStdProvider<65536>> {
        let mut resolved = BoundedVec::new(NoStdProvider::<65536>::default()).unwrap();

        for import in &self.imports {
            // Find matching import by name
            for (name, value) in imports.imports.iter() {
                if name.as_str() == import.name.as_str() {
                    let resolved_import = self.resolve_import(import, value, context)?;
                    resolved.push(resolved_import).map_err(|_| {
                        wrt_foundation::Error::new(
                            wrt_foundation::ErrorCategory::Resource,
                            wrt_error::codes::RESOURCE_EXHAUSTED,
                            "Too many resolved imports"
                        )
                    })?;
                    break;
                }
            }
        }

        Ok(resolved)
    }

    /// Resolve a single import
    fn resolve_import(
        &self,
        import: &Import,
        value: &ImportValue,
        context: &mut InstantiationContext,
    ) -> WrtResult<ResolvedImport> {
        match value {
            ImportValue::Function(func) => {
                // Register the function with the execution engine
                #[cfg(feature = "std")]
                let func_index = {
                    // Create a host function wrapper
                    let implementation = func.implementation.clone();
                    context.execution_engine.register_host_function(Box::new(
                        HostFunctionWrapper { signature: func.signature.clone(), implementation },
                    ))?
                };
                #[cfg(not(any(feature = "std", )))]
                let func_index =
                    context.execution_engine.register_host_function(func.implementation)?;

                Ok(ResolvedImport::Function(func_index))
            }
            ImportValue::Value(val) => Ok(ResolvedImport::Value(val.clone())),
            ImportValue::Instance(inst) => Ok(ResolvedImport::Instance(inst.clone())),
            ImportValue::Type(ty) => Ok(ResolvedImport::Type(ty.clone())),
        }
    }

    /// Initialize embedded modules
    #[cfg(all(feature = "std", feature = "safety-critical"))]
    fn initialize_modules(
        &self,
        resolved_imports: &[ResolvedImport],
        context: &mut InstantiationContext,
    ) -> WrtResult<WrtVec<ModuleInstance, {CrateId::Component as u8}, 64>> {
        let mut instances = WrtVec::new();

        // Initialize each embedded module
        for (module_index, _module) in self.modules.iter().enumerate() {
            // Create module instance
            let instance = ModuleInstance { module_index: module_index as u32 };
            instances.push(instance).map_err(|_| {
                wrt_foundation::Error::new(
                    wrt_foundation::ErrorCategory::Resource,
                    wrt_error::codes::RESOURCE_EXHAUSTED,
                    "Too many module instances (limit: 64)"
                )
            })?;
        }

        Ok(instances)
    }
    
    /// Initialize embedded modules
    #[cfg(all(feature = "std", not(feature = "safety-critical")))]
    fn initialize_modules(
        &self,
        resolved_imports: &[ResolvedImport],
        context: &mut InstantiationContext,
    ) -> WrtResult<Vec<ModuleInstance>> {
        let mut instances = Vec::new();

        // Initialize each embedded module
        for (module_index, _module) in self.modules.iter().enumerate() {
            // Create module instance
            let instance = ModuleInstance { module_index: module_index as u32 };
            instances.push(instance);
        }

        Ok(instances)
    }

    #[cfg(not(any(feature = "std", )))]
    fn initialize_modules(
        &self,
        resolved_imports: &BoundedVec<ResolvedImport, MAX_IMPORTS, NoStdProvider<65536>>,
        context: &mut InstantiationContext,
    ) -> Wrtcore::result::Result<BoundedVec<ModuleInstance, MAX_INSTANCES, NoStdProvider<65536>>, NoStdProvider<65536>> {
        let mut instances = BoundedVec::new(NoStdProvider::<65536>::default()).unwrap();

        // Initialize each embedded module
        for (module_index, _module) in self.modules.iter().enumerate() {
            let instance = ModuleInstance { module_index: module_index as u32 };
            instances.push(instance).map_err(|_| {
                wrt_foundation::Error::new(
                    wrt_foundation::ErrorCategory::Resource,
                    wrt_error::codes::RESOURCE_EXHAUSTED,
                    "Too many module instances"
                )
            })?;
        }

        Ok(instances)
    }

    /// Extract exports from the instance
    #[cfg(all(feature = "std", feature = "safety-critical"))]
    fn extract_exports(
        &self,
        module_instances: &[ModuleInstance],
        context: &mut InstantiationContext,
    ) -> WrtResult<WrtVec<ResolvedExport, {CrateId::Component as u8}, 256>> {
        let mut exports = WrtVec::new();

        for export in &self.exports {
            // Resolve export to actual value based on export kind
            let resolved = match &export.kind {
                crate::export::ExportKind::Func(func_idx) => {
                    // Create function export
                    let func_export = FunctionExport {
                        signature: ComponentType::Unit, // TODO: Get actual signature
                        index: *func_idx,
                    };
                    ResolvedExport {
                        name: export.name.clone(),
                        value: ExportValue::Function(func_export),
                    }
                }
                crate::export::ExportKind::Value(val_idx) => {
                    // Create value export
                    ResolvedExport {
                        name: export.name.clone(),
                        value: ExportValue::Value(ComponentValue::Unit),
                    }
                }
                crate::export::ExportKind::Type(type_idx) => {
                    // Create type export
                    ResolvedExport {
                        name: export.name.clone(),
                        value: ExportValue::Type(ComponentType::Unit),
                    }
                }
                crate::export::ExportKind::Instance(inst_idx) => {
                    // Create instance export - simplified
                    ResolvedExport {
                        name: export.name.clone(),
                        value: ExportValue::Value(ComponentValue::Unit),
                    }
                }
            };
            exports.push(resolved).map_err(|_| {
                wrt_foundation::Error::new(
                    wrt_foundation::ErrorCategory::Resource,
                    wrt_error::codes::RESOURCE_EXHAUSTED,
                    "Too many exports (limit: 256)"
                )
            })?;
        }

        Ok(exports)
    }
    
    /// Extract exports from the instance
    #[cfg(all(feature = "std", not(feature = "safety-critical")))]
    fn extract_exports(
        &self,
        module_instances: &[ModuleInstance],
        context: &mut InstantiationContext,
    ) -> WrtResult<Vec<ResolvedExport>> {
        let mut exports = Vec::new();

        for export in &self.exports {
            // Resolve export to actual value based on export kind
            let resolved = match &export.kind {
                crate::export::ExportKind::Func(func_idx) => {
                    // Create function export
                    let func_export = FunctionExport {
                        signature: ComponentType::Unit, // TODO: Get actual signature
                        index: *func_idx,
                    };
                    ResolvedExport {
                        name: export.name.clone(),
                        value: ExportValue::Function(func_export),
                    }
                }
                crate::export::ExportKind::Value(val_idx) => {
                    // Create value export
                    ResolvedExport {
                        name: export.name.clone(),
                        value: ExportValue::Value(ComponentValue::Unit),
                    }
                }
                crate::export::ExportKind::Type(type_idx) => {
                    // Create type export
                    ResolvedExport {
                        name: export.name.clone(),
                        value: ExportValue::Type(ComponentType::Unit),
                    }
                }
                crate::export::ExportKind::Instance(inst_idx) => {
                    // Create instance export - simplified
                    ResolvedExport {
                        name: export.name.clone(),
                        value: ExportValue::Value(ComponentValue::Unit),
                    }
                }
            };
            exports.push(resolved);
        }

        Ok(exports)
    }

    #[cfg(not(any(feature = "std", )))]
    fn extract_exports(
        &self,
        module_instances: &BoundedVec<ModuleInstance, MAX_INSTANCES, NoStdProvider<65536>>,
        context: &mut InstantiationContext,
    ) -> Wrtcore::result::Result<BoundedVec<ResolvedExport, MAX_EXPORTS, NoStdProvider<65536>>, NoStdProvider<65536>> {
        let mut exports = BoundedVec::new(NoStdProvider::<65536>::default()).unwrap();

        for export in &self.exports {
            let resolved = match &export.kind {
                crate::export::ExportKind::Func(func_idx) => {
                    let func_export =
                        FunctionExport { signature: ComponentType::Unit, index: *func_idx };
                    ResolvedExport {
                        name: export.name.clone(),
                        value: ExportValue::Function(func_export),
                    }
                }
                crate::export::ExportKind::Value(_) => ResolvedExport {
                    name: export.name.clone(),
                    value: ExportValue::Value(ComponentValue::Unit),
                },
                crate::export::ExportKind::Type(_) => ResolvedExport {
                    name: export.name.clone(),
                    value: ExportValue::Type(ComponentType::Unit),
                },
                crate::export::ExportKind::Instance(_) => ResolvedExport {
                    name: export.name.clone(),
                    value: ExportValue::Value(ComponentValue::Unit),
                },
            };
            exports.push(resolved).map_err(|_| {
                wrt_foundation::Error::new(
                    wrt_foundation::ErrorCategory::Resource,
                    wrt_error::codes::RESOURCE_EXHAUSTED,
                    "Too many exports"
                )
            })?;
        }

        Ok(exports)
    }
}

/// Resolved import after validation and registration
#[derive(Debug, Clone)]
pub enum ResolvedImport {
    Function(u32), // Function index in execution engine
    Value(ComponentValue),
    Instance(InstanceImport),
    Type(ComponentType),
}

/// Resolved export with actual values
#[derive(Debug, Clone)]
pub struct ResolvedExport {
    #[cfg(feature = "std")]
    pub name: String,
    #[cfg(not(any(feature = "std", )))]
    pub name: BoundedString<64, NoStdProvider<65536>>,
    pub value: ExportValue,
}

/// Resource table for a specific resource type
#[derive(Debug, Clone)]
pub struct ResourceTable {
    pub type_id: u32,
    // Simplified for now, would contain actual resource entries
}

/// Module instance within a component
#[derive(Debug, Clone)]
pub struct ModuleInstance {
    pub module_index: u32,
    // Simplified for now, would contain actual runtime instance
}

/// Host function wrapper for the execution engine
#[cfg(feature = "std")]
struct HostFunctionWrapper {
    signature: ComponentType,
    implementation: Box<dyn Fn(&[Value]) -> WrtResult<Value>>,
}

#[cfg(feature = "std")]
impl crate::execution_engine::HostFunction for HostFunctionWrapper {
    fn call(&mut self, args: &[Value]) -> WrtResult<Value> {
        (self.implementation)(args)
    }

    fn signature(&self) -> &ComponentType {
        &self.signature
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_import_values() {
        let mut imports = ImportValues::new();

        #[cfg(feature = "std")]
        {
            let func = FunctionImport {
                signature: ComponentType::Unit,
                implementation: Box::new(|_args| Ok(Value::U32(42))),
            };
            imports.add("test_func".to_string(), ImportValue::Function(func)).unwrap();
            assert!(imports.get("test_func").is_some());
            assert!(imports.get("unknown").is_none());
        }

        #[cfg(not(any(feature = "std", )))]
        {
            let func = FunctionImport {
                signature: ComponentType::Unit,
                implementation: |_args| Ok(Value::U32(42)),
            };
            let name = BoundedString::from_str("test_func").unwrap();
            imports.add(name, ImportValue::Function(func)).unwrap();
            assert!(imports.get("test_func").is_some());
            assert!(imports.get("unknown").is_none());
        }
    }

    #[test]
    fn test_instantiation_context() {
        let mut context = InstantiationContext::new();
        assert_eq!(context.next_instance_id(), 0);
        assert_eq!(context.next_instance_id(), 1);
        assert_eq!(context.next_instance_id(), 2);
    }
}
