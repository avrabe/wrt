//! Component instantiation and linking
//!
//! This module implements the instantiation process for WebAssembly components,
//! including import resolution, export extraction, and instance isolation.

#[cfg(not(feature = "std"))]
use core::{fmt, mem};
#[cfg(feature = "std")]
use std::{fmt, mem};

#[cfg(all(feature = "std", feature = "safety-criticalMissing messageMissing messageMissing message"))]
use wrt_foundation::allocator::{WrtHashMap, WrtVec, CrateId};
#[cfg(all(feature = "std", not(feature = "safety-criticalMissing messageMissing messageMissing message")))]
use std::{boxed::Box, collections::BTreeMap, string::String, vec::Vec};

use wrt_foundation::{
    bounded::BoundedVec, bounded::BoundedString, prelude::*,
    budget_aware_provider::CrateId,
    safe_managed_alloc,
};
use crate::prelude::WrtComponentType;

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

// Type alias for instantiation provider - using capability-based allocation
// Provider instances are created using safe_managed_alloc! macro
type InstantiationProvider = wrt_foundation::safe_memory::NoStdProvider<65536>;

/// Import value provided during instantiation
#[derive(Debug, Clone)]
pub enum ImportValue {
    /// A function import
    Function(FunctionImport),
    /// A value import (global, memory, table)
    Value(WrtComponentValue),
    /// An instance import
    Instance(InstanceImport),
    /// A type import
    Type(WrtComponentType),
}

/// Function import descriptor
#[derive(Debug, Clone)]
pub struct FunctionImport {
    /// Function signature
    pub signature: WrtComponentType,
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
    #[cfg(all(feature = "std", feature = "safety-criticalMissing messageMissing messageMissing message"))]
    pub exports: WrtHashMap<String, ExportValue, {CrateId::Component as u8}, 256>,
    #[cfg(all(feature = "std", not(feature = "safety-criticalMissing messageMissing messageMissing message")))]
    pub exports: BTreeMap<String, ExportValue>,
    #[cfg(not(feature = "std"))]
    pub exports: BoundedVec<(BoundedString<64, InstantiationProvider>, ExportValue), MAX_EXPORTS, InstantiationProvider>,
}

/// Export value from an instance
#[derive(Debug, Clone)]
pub enum ExportValue {
    /// A function export
    Function(FunctionExport),
    /// A value export
    Value(WrtComponentValue),
    /// An instance export
    Instance(Box<ComponentInstance>),
    /// A type export
    Type(WrtComponentType),
}

/// Function export descriptor
#[derive(Debug, Clone)]
pub struct FunctionExport {
    /// Function signature
    pub signature: WrtComponentType,
    /// Function index in the instance
    pub index: u32,
}

/// Import values provided during instantiation
pub struct ImportValues {
    /// Map of import names to values
    #[cfg(all(feature = "std", feature = "safety-criticalMissing messageMissing messageMissing message"))]
    imports: WrtHashMap<String, ImportValue, {CrateId::Component as u8}, 256>,
    #[cfg(all(feature = "std", not(feature = "safety-criticalMissing messageMissing messageMissing message")))]
    imports: BTreeMap<String, ImportValue>,
    #[cfg(not(feature = "std"))]
    imports: BoundedVec<(BoundedString<64, InstantiationProvider>, ImportValue), MAX_IMPORTS, InstantiationProvider>,
}

impl ImportValues {
    /// Create new import values
    pub fn new() -> WrtResult<Self> {
        Ok(Self {
            #[cfg(all(feature = "std", feature = "safety-criticalMissing messageMissing messageMissing message"))]
            imports: WrtHashMap::new(),
            #[cfg(all(feature = "std", not(feature = "safety-criticalMissing messageMissing messageMissing message")))]
            imports: BTreeMap::new(),
            #[cfg(not(feature = "std"))]
            imports: {
                let provider = safe_managed_alloc!(65536, CrateId::Component)?;
                BoundedVec::new(provider)?
            },
        })
    }

    /// Add an import value
    #[cfg(all(feature = "std", feature = "safety-criticalMissing messageMissing messageMissing message"))]
    pub fn add(&mut self, name: String, value: ImportValue) -> WrtResult<()> {
        self.imports.insert(name, value).map_err(|_| {
            wrt_error::Error::resource_exhausted("Error occurred"Too many imports (limit: 256)Missing message")
        })?;
        Ok(()
    }
    
    /// Add an import value
    #[cfg(all(feature = "std", not(feature = "safety-criticalMissing messageMissing messageMissing message")))]
    pub fn add(&mut self, name: String, value: ImportValue) -> WrtResult<()> {
        self.imports.insert(name, value);
        Ok(()
    }

    /// Add an import value (no_std version)
    #[cfg(not(any(feature = "std", )))]
    pub fn add(&mut self, name: BoundedString<64, InstantiationProvider>, value: ImportValue) -> WrtResult<()> {
        self.imports
            .push((name, value)
            .map_err(|_| wrt_error::Error::resource_exhausted("Error occurred"Too many importsMissing messageMissing messageMissing message")
    }

    /// Get an import value by name
    #[cfg(all(feature = "std", feature = "safety-criticalMissing messageMissing messageMissing message"))]
    pub fn get(&self, name: &str) -> Option<&ImportValue> {
        self.imports.get(name)
    }
    
    /// Get an import value by name
    #[cfg(all(feature = "std", not(feature = "safety-criticalMissing messageMissing messageMissing message")))]
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
        Self::new().unwrap_or_else(|_| {
            // Fallback for default construction in case allocation fails
            Self {
                #[cfg(all(feature = "std", feature = "safety-criticalMissing messageMissing messageMissing message"))]
                imports: WrtHashMap::new(),
                #[cfg(all(feature = "std", not(feature = "safety-criticalMissing messageMissing messageMissing message")))]
                imports: BTreeMap::new(),
                #[cfg(not(feature = "std"))]
                imports: {
                    // Use fallback provider only in default construction
                    let provider = InstantiationProvider::default();
                    BoundedVec::new(provider).unwrap()
                },
            }
        })
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
            #[cfg(all(feature = "std", feature = "safety-criticalMissing messageMissing messageMissing message"))]
            imports: resolved_imports,
            #[cfg(all(feature = "std", feature = "safety-criticalMissing messageMissing messageMissing message"))]
            exports,
            #[cfg(all(feature = "std", feature = "safety-criticalMissing messageMissing messageMissing message"))]
            resource_tables,
            #[cfg(all(feature = "std", feature = "safety-criticalMissing messageMissing messageMissing message"))]
            module_instances,
            #[cfg(all(feature = "std", not(feature = "safety-criticalMissing messageMissing messageMissing message")))]
            imports: resolved_imports,
            #[cfg(all(feature = "std", not(feature = "safety-criticalMissing messageMissing messageMissing message")))]
            exports,
            #[cfg(all(feature = "std", not(feature = "safety-criticalMissing messageMissing messageMissing message")))]
            resource_tables,
            #[cfg(all(feature = "std", not(feature = "safety-criticalMissing messageMissing messageMissing message")))]
            module_instances,
            #[cfg(not(feature = "std"))]
            imports: {
                let provider = safe_managed_alloc!(65536, CrateId::Component)?;
                BoundedVec::new(provider)?
            },
            #[cfg(not(feature = "std"))]
            exports: {
                let provider = safe_managed_alloc!(65536, CrateId::Component)?;
                BoundedVec::new(provider)?
            },
            #[cfg(not(feature = "std"))]
            resource_tables: {
                let provider = safe_managed_alloc!(65536, CrateId::Component)?;
                BoundedVec::new(provider)?
            },
            #[cfg(not(feature = "std"))]
            module_instances: {
                let provider = safe_managed_alloc!(65536, CrateId::Component)?;
                BoundedVec::new(provider)?
            },
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
                        return Err(wrt_error::Error::validation_invalid_input("Error occurred"Invalid inputMissing messageMissing messageMissing message");
                    }
                }
            }
        }
        #[cfg(not(any(feature = "std", )))]
        {
            // In no_std, we have limited validation
            // Just check that we have some imports if required
            if self.imports.len() > 0 && imports.imports.len() == 0 {
                return Err(wrt_error::Error::validation_invalid_input("Error occurred"Invalid inputMissing messageMissing messageMissing message");
            }
        }

        Ok(()
    }

    /// Validate that an import value matches the expected type
    fn validate_import_type(&self, import: &Import, value: &ImportValue) -> WrtResult<()> {
        match (&import.import_type, value) {
            (crate::import::ImportType::Function(expected), ImportValue::Function(actual)) => {
                // Check function signature compatibility
                if !self.is_function_compatible(expected, &actual.signature) {
                    return Err(wrt_foundation::wrt_error::Error::type_mismatch_error("Error occurred"Function import type mismatchMissing messageMissing messageMissing message");
                }
            }
            (crate::import::ImportType::Value(expected), ImportValue::Value(actual)) => {
                // Check value type compatibility
                if !self.is_value_compatible(expected, actual) {
                    return Err(wrt_foundation::wrt_error::Error::type_mismatch_error("Error occurred"Value import type mismatchMissing messageMissing messageMissing message");
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
                return Err(wrt_foundation::wrt_error::Error::type_mismatch_error("Error occurred"Import kind mismatchMissing messageMissing messageMissing message");
            }
        }
        Ok(()
    }

    /// Check if function types are compatible
    fn is_function_compatible(&self, expected: &WrtComponentType, actual: &WrtComponentType) -> bool {
        // Check basic type equality for now
        // In a full implementation, this would check subtyping rules
        match (expected, actual) {
            (WrtComponentType::Unit, WrtComponentType::Unit) => true,
            // For other types, check structural equality
            _ => expected == actual,
        }
    }

    /// Check if value types are compatible
    fn is_value_compatible(&self, expected: &WrtComponentType, actual: &WrtComponentValue) -> bool {
        // Basic type compatibility check
        match (expected, actual) {
            (WrtComponentType::Unit, WrtComponentValue::Unit) => true,
            // For other types, this would need more complex checking
            _ => true, // Allow for now
        }
    }

    /// Create resource tables for the instance with budget enforcement
    #[cfg(all(feature = "std", feature = "safety-criticalMissing messageMissing messageMissing message"))]
    fn create_resource_tables(&self) -> WrtResult<WrtVec<ResourceTable, {CrateId::Component as u8}, 16>> {
        let mut tables = WrtVec::new();

        // Create resource tables based on component types
        // For each resource type in the component, create a table
        for _type_id in 0..self.types.len().min(16) {
            // Create a budget-aware table for this resource type
            let table = ResourceTable::new()?;
            tables.push(table).map_err(|_| {
                wrt_error::Error::resource_exhausted("Error occurred"Too many resource tables (limit: 16)Missing message")
            })?;
        }

        Ok(tables)
    }
    
    /// Create resource tables for the instance
    #[cfg(all(feature = "std", not(feature = "safety-criticalMissing messageMissing messageMissing message")))]
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
                wrt_error::Error::resource_exhausted("Error occurred"Too many resource tablesMissing message")
            })?;
        }

        Ok(tables)
    }

    /// Resolve imports into concrete values
    #[cfg(all(feature = "std", feature = "safety-criticalMissing messageMissing messageMissing message"))]
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
                    wrt_error::Error::resource_exhausted("Error occurred"Too many resolved imports (limit: 256)Missing message")
                })?;
            }
        }

        Ok(resolved)
    }
    
    /// Resolve imports into concrete values
    #[cfg(all(feature = "std", not(feature = "safety-criticalMissing messageMissing messageMissing message")))]
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
    ) -> WrtResult<BoundedVec<ResolvedImport, MAX_IMPORTS, InstantiationProvider>> {
        let provider = safe_managed_alloc!(65536, CrateId::Component)?;
        let mut resolved = BoundedVec::new(provider)?;

        for import in &self.imports {
            // Find matching import by name
            for (name, value) in imports.imports.iter() {
                if name.as_str() == import.name.as_str() {
                    let resolved_import = self.resolve_import(import, value, context)?;
                    resolved.push(resolved_import).map_err(|_| {
                        wrt_error::Error::resource_exhausted("Error occurred"Too many resolved importsMissing message")
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

                Ok(ResolvedImport::Function(func_index)
            }
            ImportValue::Value(val) => Ok(ResolvedImport::Value(val.clone())),
            ImportValue::Instance(inst) => Ok(ResolvedImport::Instance(inst.clone())),
            ImportValue::Type(ty) => Ok(ResolvedImport::Type(ty.clone())),
        }
    }

    /// Initialize embedded modules
    #[cfg(all(feature = "std", feature = "safety-criticalMissing messageMissing messageMissing message"))]
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
                wrt_error::Error::resource_exhausted("Error occurred"Too many module instances (limit: 64)Missing message")
            })?;
        }

        Ok(instances)
    }
    
    /// Initialize embedded modules
    #[cfg(all(feature = "std", not(feature = "safety-criticalMissing messageMissing messageMissing message")))]
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
        resolved_imports: &BoundedVec<ResolvedImport, MAX_IMPORTS, InstantiationProvider>,
        context: &mut InstantiationContext,
    ) -> WrtResult<BoundedVec<ModuleInstance, MAX_INSTANCES, InstantiationProvider>> {
        let provider = safe_managed_alloc!(65536, CrateId::Component)?;
        let mut instances = BoundedVec::new(provider)?;

        // Initialize each embedded module
        for (module_index, _module) in self.modules.iter().enumerate() {
            let instance = ModuleInstance { module_index: module_index as u32 };
            instances.push(instance).map_err(|_| {
                wrt_error::Error::resource_exhausted("Error occurred"Too many module instancesMissing message")
            })?;
        }

        Ok(instances)
    }

    /// Extract exports from the instance
    #[cfg(all(feature = "std", feature = "safety-criticalMissing messageMissing messageMissing message"))]
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
                        signature: WrtComponentType::Unit, // TODO: Get actual signature
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
                        value: ExportValue::Value(WrtComponentValue::Unit),
                    }
                }
                crate::export::ExportKind::Type(type_idx) => {
                    // Create type export
                    ResolvedExport {
                        name: export.name.clone(),
                        value: ExportValue::Type(WrtComponentType::Unit),
                    }
                }
                crate::export::ExportKind::Instance(inst_idx) => {
                    // Create instance export - simplified
                    ResolvedExport {
                        name: export.name.clone(),
                        value: ExportValue::Value(WrtComponentValue::Unit),
                    }
                }
            };
            exports.push(resolved).map_err(|_| {
                wrt_error::Error::resource_exhausted("Error occurred"Too many exports (limit: 256)Missing message")
            })?;
        }

        Ok(exports)
    }
    
    /// Extract exports from the instance
    #[cfg(all(feature = "std", not(feature = "safety-criticalMissing messageMissing messageMissing message")))]
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
                        signature: WrtComponentType::Unit, // TODO: Get actual signature
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
                        value: ExportValue::Value(WrtComponentValue::Unit),
                    }
                }
                crate::export::ExportKind::Type(type_idx) => {
                    // Create type export
                    ResolvedExport {
                        name: export.name.clone(),
                        value: ExportValue::Type(WrtComponentType::Unit),
                    }
                }
                crate::export::ExportKind::Instance(inst_idx) => {
                    // Create instance export - simplified
                    ResolvedExport {
                        name: export.name.clone(),
                        value: ExportValue::Value(WrtComponentValue::Unit),
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
        module_instances: &BoundedVec<ModuleInstance, MAX_INSTANCES, InstantiationProvider>,
        context: &mut InstantiationContext,
    ) -> WrtResult<BoundedVec<ResolvedExport, MAX_EXPORTS, InstantiationProvider>> {
        let provider = safe_managed_alloc!(65536, CrateId::Component)?;
        let mut exports = BoundedVec::new(provider)?;

        for export in &self.exports {
            let resolved = match &export.kind {
                crate::export::ExportKind::Func(func_idx) => {
                    let func_export =
                        FunctionExport { signature: WrtComponentType::Unit, index: *func_idx };
                    ResolvedExport {
                        name: export.name.clone(),
                        value: ExportValue::Function(func_export),
                    }
                }
                crate::export::ExportKind::Value(_) => ResolvedExport {
                    name: export.name.clone(),
                    value: ExportValue::Value(WrtComponentValue::Unit),
                },
                crate::export::ExportKind::Type(_) => ResolvedExport {
                    name: export.name.clone(),
                    value: ExportValue::Type(WrtComponentType::Unit),
                },
                crate::export::ExportKind::Instance(_) => ResolvedExport {
                    name: export.name.clone(),
                    value: ExportValue::Value(WrtComponentValue::Unit),
                },
            };
            exports.push(resolved).map_err(|_| {
                wrt_error::Error::resource_exhausted("Error occurred"Too many exportsMissing message")
            })?;
        }

        Ok(exports)
    }
}

/// Resolved import after validation and registration
#[derive(Debug, Clone)]
pub enum ResolvedImport {
    Function(u32), // Function index in execution engine
    Value(WrtComponentValue),
    Instance(InstanceImport),
    Type(WrtComponentType),
}

/// Resolved export with actual values
#[derive(Debug, Clone)]
pub struct ResolvedExport {
    #[cfg(feature = "std")]
    pub name: String,
    #[cfg(not(any(feature = "std", )))]
    pub name: BoundedString<64, InstantiationProvider>,
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
    signature: WrtComponentType,
    implementation: Box<dyn Fn(&[Value]) -> WrtResult<Value>>,
}

#[cfg(feature = "std")]
impl crate::execution_engine::HostFunction for HostFunctionWrapper {
    fn call(&mut self, args: &[Value]) -> WrtResult<Value> {
        (self.implementation)(args)
    }

    fn signature(&self) -> &WrtComponentType {
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
                signature: WrtComponentType::Unit,
                implementation: Box::new(|_args| Ok(Value::U32(42))),
            };
            imports.add("test_func".to_string(), ImportValue::Function(func)).unwrap();
            assert!(imports.get("test_funcMissing message").is_some();
            assert!(imports.get("unknownMissing message").is_none();
        }

        #[cfg(not(any(feature = "std", )))]
        {
            let func = FunctionImport {
                signature: WrtComponentType::Unit,
                implementation: |_args| Ok(Value::U32(42)),
            };
            let name = BoundedString::from_str("test_funcMissing message").unwrap();
            imports.add(name, ImportValue::Function(func)).unwrap();
            assert!(imports.get("test_funcMissing message").is_some();
            assert!(imports.get("unknownMissing message").is_none();
        }
    }

    #[test]
    fn test_instantiation_context() {
        let mut context = InstantiationContext::new();
        assert_eq!(context.next_instance_id(), 0);
        assert_eq!(context.next_instance_id(), 1);
        assert_eq!(context.next_instance_id(), 2);
    }

    // Tests migrated from simple_instantiation_test.rs

    #[test]
    fn test_value_imports() {
        let mut imports = ImportValues::new();
        
        let value_import = ImportValue::Value(WrtComponentValue::U32(100);
        
        #[cfg(feature = "std")]
        {
            let result = imports.add("test_value".to_string(), value_import);
            assert!(result.is_ok();
            
            let retrieved = imports.get("test_valueMissing message");
            assert!(retrieved.is_some();
            
            match retrieved.unwrap() {
                ImportValue::Value(WrtComponentValue::U32(val)) => {
                    assert_eq!(*val, 100);
                }
                _ => panic!("Expected value importMissing message"),
            }
        }
        
        #[cfg(not(feature = "std"))]
        {
            let name = BoundedString::from_str("test_valueMissing message").unwrap();
            let result = imports.add(name, value_import);
            assert!(result.is_ok();
            
            let retrieved = imports.get("test_valueMissing message");
            assert!(retrieved.is_some();
            
            match retrieved.unwrap() {
                ImportValue::Value(WrtComponentValue::U32(val)) => {
                    assert_eq!(*val, 100);
                }
                _ => panic!("Expected value importMissing message"),
            }
        }
    }

    // Note: The following tests are commented out as they depend on 
    // components that may not be fully implemented yet.
    // They should be uncommented and adjusted as the implementation progresses.

    /*
    #[test]
    fn test_full_instantiation_context() {
        let mut context = InstantiationContext::new();
        
        // Verify all subsystems are initialized
        assert_eq!(context.canonical_abi.string_encoding(), crate::string_encoding::StringEncoding::Utf8);
        assert_eq!(context.execution_engine.state(), &crate::execution_engine::ExecutionState::Ready);
        
        // Test registering a host function
        #[cfg(not(feature = "std"))]
        {
            fn test_host_func(_args: &[Value]) -> WrtResult<Value> {
                Ok(Value::Bool(true)
            }
            
            let func_index = context.execution_engine.register_host_function(test_host_func);
            assert!(func_index.is_ok();
            assert_eq!(func_index.unwrap(), 0);
        }
        
        #[cfg(feature = "std")]
        {
            use crate::execution_engine::HostFunction;
            
            struct TestHostFunc;
            impl HostFunction for TestHostFunc {
                fn call(&mut self, _args: &[Value]) -> WrtResult<Value> {
                    Ok(Value::Bool(true)
                }
                
                fn signature(&self) -> &WrtComponentType {
                    &WrtComponentType::Unit
                }
            }
            
            let func_index = context.execution_engine.register_host_function(Box::new(TestHostFunc);
            assert!(func_index.is_ok();
            assert_eq!(func_index.unwrap(), 0);
        }
    }

    #[test]
    fn test_resource_management() {
        let mut context = InstantiationContext::new();
        
        // Create a resource
        let resource_data = WrtComponentValue::String("test_resource".into();
        let handle = context.resource_manager.create_resource(1, resource_data);
        assert!(handle.is_ok();
        
        let handle = handle.unwrap();
        
        // Borrow the resource
        let borrowed = context.resource_manager.borrow_resource(handle);
        assert!(borrowed.is_ok();
        
        match borrowed.unwrap() {
            WrtComponentValue::String(s) => {
                assert_eq!(s.as_str(), "test_resourceMissing message");
            }
            _ => panic!("Expected string resourceMissing message"),
        }
        
        // Drop the resource
        let drop_result = context.resource_manager.drop_resource(handle);
        assert!(drop_result.is_ok();
    }

    #[test]
    fn test_memory_layout_integration() {
        let _context = InstantiationContext::new();
        
        // Test basic type layouts
        use crate::types::ValType;
        use crate::memory_layout;
        
        let bool_layout = memory_layout::calculate_layout(&ValType::Bool);
        assert_eq!(bool_layout.size, 1);
        assert_eq!(bool_layout.align, 1);
        
        let u32_layout = memory_layout::calculate_layout(&ValType::U32);
        assert_eq!(u32_layout.size, 4);
        assert_eq!(u32_layout.align, 4);
        
        let u64_layout = memory_layout::calculate_layout(&ValType::U64);
        assert_eq!(u64_layout.size, 8);
        assert_eq!(u64_layout.align, 8);
    }

    #[test]
    fn test_string_encoding_integration() {
        let _context = InstantiationContext::new();
        
        use crate::string_encoding::{StringEncoding, encode_string};
        
        // Test UTF-8 encoding (default)
        let test_string = "Hello, 世界!";
        let encoded = encode_string(test_string, StringEncoding::Utf8);
        assert!(encoded.is_ok();
        
        let encoded_bytes = encoded.unwrap();
        assert_eq!(encoded_bytes, test_string.as_bytes();
        
        // Test UTF-16LE encoding  
        let encoded_utf16 = encode_string("Hello", StringEncoding::Utf16Le);
        assert!(encoded_utf16.is_ok();
        
        // UTF-16LE encoding of "Hello" should be: H(0x48,0x00) e(0x65,0x00) l(0x6C,0x00) l(0x6C,0x00) o(0x6F,0x00)
        let expected_utf16 = vec![0x48, 0x00, 0x65, 0x00, 0x6C, 0x00, 0x6C, 0x00, 0x6F, 0x00];
        assert_eq!(encoded_utf16.unwrap(), expected_utf16);
    }
    */
}
