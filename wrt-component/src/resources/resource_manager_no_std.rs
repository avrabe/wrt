// WRT - wrt-component
// Copyright (c) 2025 Ralf Anton Beier
// Licensed under the MIT license.
// SPDX-License-Identifier: MIT

use wrt_error::kinds::PoisonedLockError;
use wrt_foundation::{bounded::BoundedString, safe_memory::NoStdProvider};

#[cfg(not(feature = "std"))]
use alloc::{boxed::Box, string::String, sync::Arc};
#[cfg(feature = "std")]
use std::sync::Arc;

use super::{MemoryStrategy, Resource, ResourceArena, ResourceTable, VerificationLevel};

/// Unique identifier for a resource
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ResourceId(pub u32);

/// Trait representing a host resource
pub trait HostResource {}

// Implement HostResource for common types
impl<T: 'static + Send + Sync> HostResource for T {}

/// Manager for WebAssembly Component Model resource instances (no_std
/// compatible)
#[derive(Clone)]
pub struct ResourceManager {
    /// Resource table for this manager
    table: Arc<Mutex<ResourceTable>>,
    /// Component instance ID  
    instance_id: String,
    /// Default memory strategy
    default_memory_strategy: MemoryStrategy,
    /// Default verification level
    default_verification_level: VerificationLevel,
    /// Maximum allowed resources
    max_resources: usize,
}

impl ResourceManager {
    /// Create a new resource manager with default settings
    pub fn new() -> Self {
        Self::new_with_id("default-instance")
    }

    /// Create a new resource manager with a specific instance ID
    pub fn new_with_id(instance_id: &str) -> Self {
        Self {
            table: Arc::new(Mutex::new(ResourceTable::new())),
            instance_id: instance_id.to_string(),
            default_memory_strategy: MemoryStrategy::default(),
            default_verification_level: VerificationLevel::Critical,
            max_resources: 64, // Default to MAX_RESOURCES from resource_table_no_std
        }
    }

    /// Create a new resource manager with custom settings
    pub fn new_with_config(
        instance_id: &str,
        memory_strategy: MemoryStrategy,
        verification_level: VerificationLevel,
    ) -> Self {
        Self {
            table: Arc::new(Mutex::new(ResourceTable::new())),
            instance_id: instance_id.to_string(),
            default_memory_strategy: memory_strategy,
            default_verification_level: verification_level,
            max_resources: 64,
        }
    }

    /// Create a new resource
    pub fn create_resource(&self, type_idx: u32, data: Box<dyn Any + Send + Sync>) -> Result<u32> {
        let mut table = self.table.lock().map_err(|e| {
            Error::new(
                ErrorCategory::Runtime,
                codes::POISONED_LOCK,
                PoisonedLockError("Component not found"),
            )
        })?;

        table.create_resource(type_idx, data)
    }

    /// Create a named resource (with debug name)
    pub fn create_named_resource(
        &self,
        type_idx: u32,
        data: Box<dyn Any + Send + Sync>,
        name: &str,
    ) -> Result<u32> {
        let mut table = self.table.lock().map_err(|e| {
            Error::new(
                ErrorCategory::Runtime,
                codes::POISONED_LOCK,
                PoisonedLockError("Component not found"),
            )
        })?;

        // Create the resource
        let handle = table.create_resource(type_idx, data)?;

        // Set the name if we have access to the resource
        if let Ok(res) = table.get_resource(handle) {
            if let Ok(mut res_guard) = res.lock() {
                res_guard.name = Some(name.to_string());
            }
        }

        Ok(handle)
    }

    /// Get a resource by handle
    pub fn get_resource(&self, handle: u32) -> Result<Box<Mutex<Resource>>> {
        let table = self.table.lock().map_err(|e| {
            Error::new(
                ErrorCategory::Runtime,
                codes::POISONED_LOCK,
                PoisonedLockError("Component not found"),
            )
        })?;

        table.get_resource(handle)
    }

    /// Drop a resource
    pub fn drop_resource(&self, handle: u32) -> Result<()> {
        let mut table = self.table.lock().map_err(|e| {
            Error::new(
                ErrorCategory::Runtime,
                codes::POISONED_LOCK,
                PoisonedLockError("Component not found"),
            )
        })?;

        table.drop_resource(handle)
    }

    /// Check if a resource exists
    pub fn has_resource(&self, id: ResourceId) -> Result<bool> {
        match self.get_resource(id.0) {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    /// Set memory strategy for a resource
    pub fn set_memory_strategy(&self, handle: u32, strategy: MemoryStrategy) -> Result<()> {
        let mut table = self.table.lock().map_err(|e| {
            Error::new(
                ErrorCategory::Runtime,
                codes::POISONED_LOCK,
                PoisonedLockError("Component not found"),
            )
        })?;

        table.set_memory_strategy(handle, strategy)
    }

    /// Set verification level for a resource
    pub fn set_verification_level(&self, handle: u32, level: VerificationLevel) -> Result<()> {
        let mut table = self.table.lock().map_err(|e| {
            Error::new(
                ErrorCategory::Runtime,
                codes::POISONED_LOCK,
                PoisonedLockError("Component not found"),
            )
        })?;

        table.set_verification_level(handle, level)
    }

    /// Get the default memory strategy
    pub fn default_memory_strategy(&self) -> MemoryStrategy {
        self.default_memory_strategy
    }

    /// Get the default verification level
    pub fn default_verification_level(&self) -> VerificationLevel {
        self.default_verification_level
    }

    /// Set the default memory strategy
    pub fn set_default_memory_strategy(&mut self, strategy: MemoryStrategy) {
        self.default_memory_strategy = strategy;
    }

    /// Set the default verification level
    pub fn set_default_verification_level(&mut self, level: VerificationLevel) {
        self.default_verification_level = level;
    }

    /// Get the number of resources
    pub fn resource_count(&self) -> Result<usize> {
        let table = self.table.lock().map_err(|e| {
            Error::new(
                ErrorCategory::Runtime,
                codes::POISONED_LOCK,
                PoisonedLockError("Component not found"),
            )
        })?;

        Ok(table.resource_count())
    }

    /// Get the component instance ID
    pub fn instance_id(&self) -> &str {
        self.instance_id
    }

    /// Create a new resource arena that uses this manager's resource table
    pub fn create_arena(&self) -> Result<ResourceArena> {
        ResourceArena::new(self.table)
    }

    /// Create a new resource arena with the given name
    pub fn create_named_arena(&self, name: &str) -> Result<ResourceArena> {
        ResourceArena::new_with_name(self.table, name)
    }
}

impl<'a> Debug for ResourceManager<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Get the resource count, or show an error if we can't access it
        let count = self.resource_count().unwrap_or(0);

        f.debug_struct("ResourceManager")
            .field("instance_id", &self.instance_id)
            .field("resource_count", &count)
            .field("default_memory_strategy", &self.default_memory_strategy)
            .field("default_verification_level", &self.default_verification_level)
            .field("max_resources", &self.max_resources)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resource_creation() {
        let table = Mutex::new(ResourceTable::new());
        let manager = ResourceManager::new(&table);

        // Create a string resource
        let data = Box::new("test".to_string());
        let handle = manager.create_resource(1, data).unwrap();

        // Verify it exists
        let result = manager.has_resource(ResourceId(handle)).unwrap();
        assert!(result);

        // Get the resource
        let resource = manager.get_resource(handle).unwrap();
        let guard = resource.lock().unwrap();

        // Verify type index
        assert_eq!(guard.type_idx, 1);
    }

    #[test]
    fn test_named_resource() {
        let table = Mutex::new(ResourceTable::new());
        let manager = ResourceManager::new(&table);

        // Create a named resource
        let data = Box::new(42i32);
        let handle = manager.create_named_resource(1, data, "answer").unwrap();

        // Get the resource and check the name
        let resource = manager.get_resource(handle).unwrap();
        let guard = resource.lock().unwrap();

        assert_eq!(guard.name, Some("answer".to_string()));
    }

    #[test]
    fn test_resource_lifecycle() {
        let table = Mutex::new(ResourceTable::new());
        let manager = ResourceManager::new(&table);

        // Add a resource
        let data = Box::new(42i32);
        let handle = manager.create_resource(1, data).unwrap();

        // Verify it exists
        let exists = manager.has_resource(ResourceId(handle)).unwrap();
        assert!(exists);

        // Drop it
        manager.drop_resource(handle).unwrap();

        // Verify it's gone
        let exists = manager.has_resource(ResourceId(handle)).unwrap();
        assert!(!exists);
    }

    #[test]
    fn test_with_arena() {
        let table = Mutex::new(ResourceTable::new());
        let manager = ResourceManager::new(&table);

        // Create an arena
        let mut arena = manager.create_arena().unwrap();

        // Add a resource through the arena
        let handle = arena.create_resource(1, Box::new("test".to_string())).unwrap();

        // Verify it exists
        assert!(manager.has_resource(ResourceId(handle)).unwrap());

        // Release arena
        arena.release_all().unwrap();

        // Verify resource is gone
        assert!(!manager.has_resource(ResourceId(handle)).unwrap());
    }
}
