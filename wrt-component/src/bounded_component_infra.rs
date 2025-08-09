//! Bounded Infrastructure for Component Model
//!
//! This module provides bounded alternatives for component collections
//! to ensure static memory allocation throughout the component model.

use wrt_foundation::{
    bounded::{
        BoundedString,
        BoundedVec,
    },
    bounded_collections::BoundedMap,
    budget_aware_provider::CrateId,
    budget_provider::BudgetProvider,
    capabilities::MemoryFactory,
    managed_alloc,
    safe_managed_alloc,
    safe_memory::NoStdProvider,
    WrtResult,
};

/// Budget-aware memory provider for component model (128KB)
pub type ComponentProvider = NoStdProvider<131072>;

/// Maximum number of component instances
pub const MAX_COMPONENT_INSTANCES: usize = 256;

/// Maximum number of component exports
pub const MAX_COMPONENT_EXPORTS: usize = 512;

/// Maximum number of component imports
pub const MAX_COMPONENT_IMPORTS: usize = 512;

/// Maximum number of resource handles per table
pub const MAX_RESOURCE_HANDLES: usize = 4096;

/// Maximum number of resource borrows
pub const MAX_RESOURCE_BORROWS: usize = 256;

/// Maximum call stack depth
pub const MAX_CALL_STACK_DEPTH: usize = 1024;

/// Maximum operand stack size
pub const MAX_OPERAND_STACK_SIZE: usize = 4096;

/// Maximum number of locals per function
pub const MAX_LOCALS_COUNT: usize = 512;

/// Maximum number of memory instances
pub const MAX_MEMORY_INSTANCES: usize = 16;

/// Maximum number of table instances
pub const MAX_TABLE_INSTANCES: usize = 16;

/// Maximum number of global instances
pub const MAX_GLOBAL_INSTANCES: usize = 1024;

/// Maximum number of host functions
pub const MAX_HOST_FUNCTIONS: usize = 1024;

/// Maximum component name length
pub const MAX_COMPONENT_NAME_LEN: usize = 256;

/// Maximum export/import name length
pub const MAX_EXPORT_NAME_LEN: usize = 256;

/// Maximum type definitions
pub const MAX_TYPE_DEFINITIONS: usize = 2048;

/// Maximum resource types
pub const MAX_RESOURCE_TYPES: usize = 256;

/// Maximum post-return callbacks
pub const MAX_POST_RETURN_CALLBACKS: usize = 64;

/// Bounded vector for component instances
pub type BoundedComponentVec<T> = BoundedVec<T, MAX_COMPONENT_INSTANCES, ComponentProvider>;

/// Bounded vector for exports
pub type BoundedExportVec<T> = BoundedVec<T, MAX_COMPONENT_EXPORTS, ComponentProvider>;

/// Bounded vector for imports
pub type BoundedImportVec<T> = BoundedVec<T, MAX_COMPONENT_IMPORTS, ComponentProvider>;

/// Bounded vector for resource handles
pub type BoundedResourceVec<T> = BoundedVec<T, MAX_RESOURCE_HANDLES, ComponentProvider>;

/// Bounded vector for resource borrows
pub type BoundedBorrowVec<T> = BoundedVec<T, MAX_RESOURCE_BORROWS, ComponentProvider>;

/// Bounded stack for call frames
pub type BoundedCallStack<T> = BoundedVec<T, MAX_CALL_STACK_DEPTH, ComponentProvider>;

/// Bounded stack for operands
pub type BoundedOperandStack<T> = BoundedVec<T, MAX_OPERAND_STACK_SIZE, ComponentProvider>;

/// Bounded vector for locals
pub type BoundedLocalsVec<T> = BoundedVec<T, MAX_LOCALS_COUNT, ComponentProvider>;

/// Bounded vector for memory instances
pub type BoundedMemoryVec<T> = BoundedVec<T, MAX_MEMORY_INSTANCES, ComponentProvider>;

/// Bounded vector for table instances
pub type BoundedTableVec<T> = BoundedVec<T, MAX_TABLE_INSTANCES, ComponentProvider>;

/// Bounded vector for global instances
pub type BoundedGlobalVec<T> = BoundedVec<T, MAX_GLOBAL_INSTANCES, ComponentProvider>;

/// Bounded vector for host functions
pub type BoundedHostFunctionVec<T> = BoundedVec<T, MAX_HOST_FUNCTIONS, ComponentProvider>;

/// Bounded string for component names
pub type BoundedComponentName = BoundedString<MAX_COMPONENT_NAME_LEN, ComponentProvider>;

/// Bounded string for export/import names
pub type BoundedExportName = BoundedString<MAX_EXPORT_NAME_LEN, ComponentProvider>;

/// Bounded map for exports
#[cfg(not(feature = "std"))]
pub type BoundedExportMap<V> =
    BoundedMap<BoundedExportName, V, MAX_COMPONENT_EXPORTS, ComponentProvider>;

#[cfg(feature = "std")]
pub type BoundedExportMap<V> =
    BoundedMap<BoundedExportName, V, MAX_COMPONENT_EXPORTS, ComponentProvider>;

/// Bounded map for imports
#[cfg(not(feature = "std"))]
pub type BoundedImportMap<V> =
    BoundedMap<BoundedExportName, V, MAX_COMPONENT_IMPORTS, ComponentProvider>;

#[cfg(feature = "std")]
pub type BoundedImportMap<V> =
    BoundedMap<BoundedExportName, V, MAX_COMPONENT_IMPORTS, ComponentProvider>;

/// Bounded map for type definitions
#[cfg(not(feature = "std"))]
pub type BoundedTypeMap<V> = BoundedMap<
    u32, // Type index
    V,
    MAX_TYPE_DEFINITIONS,
    ComponentProvider,
>;

#[cfg(feature = "std")]
pub type BoundedTypeMap<V> = BoundedMap<
    u32, // Type index
    V,
    MAX_TYPE_DEFINITIONS,
    ComponentProvider,
>;

/// Bounded map for resource types
#[cfg(not(feature = "std"))]
pub type BoundedResourceTypeMap<V> = BoundedMap<
    u32, // Resource type ID
    V,
    MAX_RESOURCE_TYPES,
    ComponentProvider,
>;

#[cfg(feature = "std")]
pub type BoundedResourceTypeMap<V> = BoundedMap<
    u32, // Resource type ID
    V,
    MAX_RESOURCE_TYPES,
    ComponentProvider,
>;

/// Bounded vector for post-return callbacks
pub type BoundedPostReturnVec<T> = BoundedVec<T, MAX_POST_RETURN_CALLBACKS, ComponentProvider>;

/// Helper function to create a safe component provider using capability context
fn create_safe_component_provider() -> WrtResult<ComponentProvider> {
    MemoryFactory::create::<131072>(CrateId::Component)
}

/// Create a new bounded component vector
pub fn new_component_vec<T>() -> WrtResult<BoundedComponentVec<T>> {
    let provider = create_safe_component_provider()?;
    BoundedVec::new(provider)
}

/// Create a new bounded export vector
pub fn new_export_vec<T>() -> WrtResult<BoundedExportVec<T>> {
    let provider = create_safe_component_provider()?;
    BoundedVec::new(provider)
}

/// Create a new bounded import vector
pub fn new_import_vec<T>() -> WrtResult<BoundedImportVec<T>> {
    let provider = create_safe_component_provider()?;
    BoundedVec::new(provider)
}

/// Create a new bounded resource vector
pub fn new_resource_vec<T>() -> WrtResult<BoundedResourceVec<T>> {
    let provider = create_safe_component_provider()?;
    BoundedVec::new(provider)
}

/// Create a new bounded call stack
pub fn new_call_stack<T>() -> WrtResult<BoundedCallStack<T>> {
    let provider = create_safe_component_provider()?;
    BoundedVec::new(provider)
}

/// Create a new bounded operand stack
pub fn new_operand_stack<T>() -> WrtResult<BoundedOperandStack<T>> {
    let provider = create_safe_component_provider()?;
    BoundedVec::new(provider)
}

/// Create a new bounded locals vector
pub fn new_locals_vec<T>() -> WrtResult<BoundedLocalsVec<T>> {
    let provider = create_safe_component_provider()?;
    BoundedVec::new(provider)
}

/// Create a new bounded component name
pub fn new_component_name() -> WrtResult<BoundedComponentName> {
    let provider = create_safe_component_provider()?;
    Ok(BoundedString::new(provider))
}

/// Create a bounded component name from str
pub fn bounded_component_name_from_str(s: &str) -> WrtResult<BoundedComponentName> {
    let provider = create_safe_component_provider()?;
    BoundedString::from_str(s, provider)
}

/// Create a new bounded export name
pub fn new_export_name() -> WrtResult<BoundedExportName> {
    let provider = create_safe_component_provider()?;
    Ok(BoundedString::new(provider))
}

/// Create a bounded export name from str
pub fn bounded_export_name_from_str(s: &str) -> WrtResult<BoundedExportName> {
    let provider = create_safe_component_provider()?;
    BoundedString::from_str(s, provider)
}

/// Create a new bounded export map
#[cfg(not(feature = "std"))]
pub fn new_export_map<V>() -> WrtResult<BoundedExportMap<V>> {
    let provider = create_safe_component_provider()?;
    BoundedMap::new(provider)
}

#[cfg(feature = "std")]
pub fn new_export_map<V>() -> WrtResult<BoundedExportMap<V>>
where
    BoundedExportName: core::hash::Hash + Eq,
    V: Default + Clone + PartialEq + Eq,
{
    let provider = create_safe_component_provider()?;
    BoundedMap::new(provider)
}

/// Create a new bounded import map
#[cfg(not(feature = "std"))]
pub fn new_import_map<V>() -> WrtResult<BoundedImportMap<V>> {
    let provider = create_safe_component_provider()?;
    BoundedMap::new(provider)
}

#[cfg(feature = "std")]
pub fn new_import_map<V>() -> WrtResult<BoundedImportMap<V>>
where
    BoundedExportName: core::hash::Hash + Eq,
    V: Default + Clone + PartialEq + Eq,
{
    let provider = create_safe_component_provider()?;
    BoundedMap::new(provider)
}

/// Create a new bounded type map
#[cfg(not(feature = "std"))]
pub fn new_type_map<V>() -> WrtResult<BoundedTypeMap<V>> {
    let provider = create_safe_component_provider()?;
    BoundedMap::new(provider)
}

#[cfg(feature = "std")]
pub fn new_type_map<V>() -> WrtResult<BoundedTypeMap<V>>
where
    V: Default + Clone + PartialEq + Eq,
{
    let provider = create_safe_component_provider()?;
    BoundedMap::new(provider)
}

/// Create a new bounded resource type map
#[cfg(not(feature = "std"))]
pub fn new_resource_type_map<V>() -> WrtResult<BoundedResourceTypeMap<V>> {
    let provider = create_safe_component_provider()?;
    BoundedMap::new(provider)
}

#[cfg(feature = "std")]
pub fn new_resource_type_map<V>() -> WrtResult<BoundedResourceTypeMap<V>>
where
    V: Default + Clone + PartialEq + Eq,
{
    let provider = create_safe_component_provider()?;
    BoundedMap::new(provider)
}
