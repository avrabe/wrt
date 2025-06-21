//! Bounded Infrastructure for WRT Main Crate
//!
//! This module provides bounded alternatives for main runtime collections
//! to ensure static memory allocation throughout the main WRT interface.

use wrt_foundation::{
    bounded::{BoundedString, BoundedVec},
    managed_alloc,
    budget_aware_provider::CrateId,
    bounded_collections::BoundedMap as BoundedHashMap,
    WrtResult,
};

#[cfg(feature = "std")]
use std::boxed::Box;
#[cfg(not(feature = "std"))]
use alloc::boxed::Box;

/// Default memory size for WRT allocations (256KB)
pub const WRT_DEFAULT_MEMORY_SIZE: usize = 262144;

/// Maximum number of WRT engines
pub const MAX_WRT_ENGINES: usize = 16;

/// Maximum number of loaded modules
pub const MAX_LOADED_MODULES: usize = 128;

/// Maximum number of runtime configurations
pub const MAX_RUNTIME_CONFIGS: usize = 32;

/// Maximum number of engine instances
pub const MAX_ENGINE_INSTANCES: usize = 64;

/// Maximum number of linker imports
pub const MAX_LINKER_IMPORTS: usize = 1024;

/// Maximum number of linker exports
pub const MAX_LINKER_EXPORTS: usize = 1024;

/// Maximum number of stores
pub const MAX_STORES: usize = 32;

/// Maximum configuration key length
pub const MAX_CONFIG_KEY_LEN: usize = 128;

/// Maximum configuration value length
pub const MAX_CONFIG_VALUE_LEN: usize = 512;

/// Maximum module path length
pub const MAX_MODULE_PATH_LEN: usize = 512;

/// Maximum engine name length
pub const MAX_ENGINE_NAME_LEN: usize = 128;

/// Maximum number of engine features
pub const MAX_ENGINE_FEATURES: usize = 64;

/// Maximum number of wasi modules
pub const MAX_WASI_MODULES: usize = 32;

/// Maximum number of component instances
pub const MAX_COMPONENT_INSTANCES: usize = 256;

/// Create a new bounded vector with managed allocation
#[macro_export]
macro_rules! bounded_vec {
    ($max_size:expr) => {{
        use wrt_foundation::safe_managed_alloc, budget_aware_provider::CrateId};
        let guard = safe_managed_alloc!($crate::bounded_wrt_infra::WRT_DEFAULT_MEMORY_SIZE, CrateId::Runtime)?;
        wrt_foundation::bounded::BoundedVec::new(guard.provider().clone())
    }};
}

/// Create a new bounded string with managed allocation
#[macro_export]
macro_rules! bounded_string {
    ($max_len:expr) => {{
        use wrt_foundation::safe_managed_alloc, budget_aware_provider::CrateId};
        let guard = safe_managed_alloc!($crate::bounded_wrt_infra::WRT_DEFAULT_MEMORY_SIZE, CrateId::Runtime)?;
        Ok(wrt_foundation::bounded::BoundedString::new(guard.provider().clone()))
    }};
    ($s:expr, $max_len:expr) => {{
        use wrt_foundation::safe_managed_alloc, budget_aware_provider::CrateId};
        let guard = safe_managed_alloc!($crate::bounded_wrt_infra::WRT_DEFAULT_MEMORY_SIZE, CrateId::Runtime)?;
        wrt_foundation::bounded::BoundedString::from_str($s, guard.provider().clone())
    }};
}

/// Create a new bounded map with managed allocation
#[macro_export]
macro_rules! bounded_map {
    ($max_entries:expr) => {{
        use wrt_foundation::safe_managed_alloc, budget_aware_provider::CrateId};
        let guard = safe_managed_alloc!($crate::bounded_wrt_infra::WRT_DEFAULT_MEMORY_SIZE, CrateId::Runtime)?;
        wrt_foundation::bounded_collections::BoundedMap::new(guard.provider().clone())
    }};
}

// Helper type aliases for documentation purposes only
// Actual types are created dynamically with the macros above

/// Type alias for WRT engine vector (for documentation)
pub type WrtEngineVecDoc<T> = BoundedVec<T, MAX_WRT_ENGINES, ()>;

/// Type alias for loaded module vector (for documentation)
pub type LoadedModuleVecDoc<T> = BoundedVec<T, MAX_LOADED_MODULES, ()>;

/// Type alias for configuration key string (for documentation)
pub type ConfigKeyDoc = BoundedString<MAX_CONFIG_KEY_LEN, ()>;

/// Type alias for configuration value string (for documentation)
pub type ConfigValueDoc = BoundedString<MAX_CONFIG_VALUE_LEN, ()>;

/// Type alias for module path string (for documentation)
pub type ModulePathDoc = BoundedString<MAX_MODULE_PATH_LEN, ()>;

/// Type alias for engine name string (for documentation)
pub type EngineNameDoc = BoundedString<MAX_ENGINE_NAME_LEN, ()>;

// Factory functions that hide provider details

/// Create a new WRT engine vector
pub fn new_wrt_engine_vec<T>() -> WrtResult<Box<dyn core::any::Any>> {
    bounded_vec!(MAX_WRT_ENGINES).map(|v| Box::new(v) as Box<dyn core::any::Any>)
}

/// Create a new loaded module vector
pub fn new_loaded_module_vec<T>() -> WrtResult<Box<dyn core::any::Any>> {
    bounded_vec!(MAX_LOADED_MODULES).map(|v| Box::new(v) as Box<dyn core::any::Any>)
}

/// Create a new runtime config vector
pub fn new_runtime_config_vec<T>() -> WrtResult<Box<dyn core::any::Any>> {
    bounded_vec!(MAX_RUNTIME_CONFIGS).map(|v| Box::new(v) as Box<dyn core::any::Any>)
}

/// Create a new engine instance vector
pub fn new_engine_instance_vec<T>() -> WrtResult<Box<dyn core::any::Any>> {
    bounded_vec!(MAX_ENGINE_INSTANCES).map(|v| Box::new(v) as Box<dyn core::any::Any>)
}

/// Create a new store vector
pub fn new_store_vec<T>() -> WrtResult<Box<dyn core::any::Any>> {
    bounded_vec!(MAX_STORES).map(|v| Box::new(v) as Box<dyn core::any::Any>)
}

/// Create a new configuration key
pub fn new_config_key() -> WrtResult<Box<dyn core::any::Any>> {
    bounded_string!(MAX_CONFIG_KEY_LEN).map(|s| Box::new(s) as Box<dyn core::any::Any>)
}

/// Create a configuration key from str
pub fn bounded_config_key_from_str(s: &str) -> WrtResult<Box<dyn core::any::Any>> {
    bounded_string!(s, MAX_CONFIG_KEY_LEN).map(|s| Box::new(s) as Box<dyn core::any::Any>)
}

/// Create a new configuration value
pub fn new_config_value() -> WrtResult<Box<dyn core::any::Any>> {
    bounded_string!(MAX_CONFIG_VALUE_LEN).map(|s| Box::new(s) as Box<dyn core::any::Any>)
}

/// Create a configuration value from str
pub fn bounded_config_value_from_str(s: &str) -> WrtResult<Box<dyn core::any::Any>> {
    bounded_string!(s, MAX_CONFIG_VALUE_LEN).map(|s| Box::new(s) as Box<dyn core::any::Any>)
}

/// Create a new module path
pub fn new_module_path() -> WrtResult<Box<dyn core::any::Any>> {
    bounded_string!(MAX_MODULE_PATH_LEN).map(|s| Box::new(s) as Box<dyn core::any::Any>)
}

/// Create a module path from str
pub fn bounded_module_path_from_str(s: &str) -> WrtResult<Box<dyn core::any::Any>> {
    bounded_string!(s, MAX_MODULE_PATH_LEN).map(|s| Box::new(s) as Box<dyn core::any::Any>)
}

/// Create a new engine name
pub fn new_engine_name() -> WrtResult<Box<dyn core::any::Any>> {
    bounded_string!(MAX_ENGINE_NAME_LEN).map(|s| Box::new(s) as Box<dyn core::any::Any>)
}

/// Create an engine name from str
pub fn bounded_engine_name_from_str(s: &str) -> WrtResult<Box<dyn core::any::Any>> {
    bounded_string!(s, MAX_ENGINE_NAME_LEN).map(|s| Box::new(s) as Box<dyn core::any::Any>)
}

/// Create a new configuration map
pub fn new_config_map() -> WrtResult<Box<dyn core::any::Any>> {
    bounded_map!(MAX_RUNTIME_CONFIGS).map(|m| Box::new(m) as Box<dyn core::any::Any>)
}

/// Create a new module map
pub fn new_module_map<V>() -> WrtResult<Box<dyn core::any::Any>> {
    bounded_map!(MAX_LOADED_MODULES).map(|m| Box::new(m) as Box<dyn core::any::Any>)
}

/// Create a new engine map
pub fn new_engine_map<V>() -> WrtResult<Box<dyn core::any::Any>> {
    bounded_map!(MAX_WRT_ENGINES).map(|m| Box::new(m) as Box<dyn core::any::Any>)
}

/// Create a new linker function map
pub fn new_linker_function_map<V>() -> WrtResult<Box<dyn core::any::Any>> {
    bounded_map!(MAX_LINKER_IMPORTS).map(|m| Box::new(m) as Box<dyn core::any::Any>)
}