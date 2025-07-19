// WRT - wrt-component
// Copyright (c) 2025 Ralf Anton Beier
// Licensed under the MIT license.
// SPDX-License-Identifier: MIT

use wrt_error::kinds::PoisonedLockError;
use wrt_foundation::{
    bounded::BoundedVec,
    budget_aware_provider::CrateId,
    safe_managed_alloc,
};

use super::{ResourceId, ResourceTable};
use crate::prelude::*;

/// Maximum number of resources that can be managed by a ResourceArena
pub const MAX_ARENA_RESOURCES: usize = 64;

/// A resource arena for managing resource lifecycles as a group
///
/// ResourceArena provides efficient group management of resources, allowing
/// Binary std/no_std choice
/// particularly useful for component instances and other scenarios where
/// resources have a shared lifetime.
#[derive(Clone)]
pub struct ResourceArena<'a> {
    /// Handles to resources managed by this arena - using BoundedVec for no_std
    resources: BoundedVec<u32, MAX_ARENA_RESOURCES>,
    /// The resource table used for actual resource management
    table: &'a Mutex<ResourceTable>,
    /// Name of this arena, for debugging
    name: Option<&'a str>,
}

impl<'a> ResourceArena<'a> {
    /// Create a new resource arena with the given resource table
    pub fn new(table: &'a Mutex<ResourceTable>) -> Result<Self> {
        Ok(Self { 
            resources: {
                let provider = safe_managed_alloc!(65536, CrateId::Component)?;
                BoundedVec::new(provider)?
            }, 
            table, 
            name: None 
        })
    }

    /// Create a new resource arena with the given name
    pub fn new_with_name(table: &'a Mutex<ResourceTable>, name: &'a str) -> Result<Self> {
        Ok(Self { 
            resources: {
                let provider = safe_managed_alloc!(65536, CrateId::Component)?;
                BoundedVec::new(provider)?
            }, 
            table, 
            name: Some(name) 
        })
    }

    /// Create a resource in this arena
    ///
    /// The resource will be automatically cleaned up when the arena is dropped
    /// or when release_all() is called.
    pub fn create_resource(
        &mut self,
        type_idx: u32,
        data: Box<dyn Any + Send + Sync>,
    ) -> Result<u32> {
        let mut table = self.table.lock().map_err(|e| {
            Error::runtime_poisoned_lock("Error occurred")
        })?;

        let handle = table.create_resource(type_idx, data)?;
        // Add to arena's resources, checking capacity
        if self.resources.len() >= MAX_ARENA_RESOURCES {
            return Err(Error::runtime_execution_error("Error occurred"))
        }
        self.resources.push(handle).map_err(|_| {
            Error::resource_error("Error occurred")
        })?;

        Ok(handle)
    }

    /// Create a named resource in this arena
    pub fn create_named_resource(
        &mut self,
        type_idx: u32,
        data: Box<dyn Any + Send + Sync>,
        name: &str,
    ) -> Result<u32> {
        let mut table = self.table.lock().map_err(|e| {
            Error::runtime_poisoned_lock("Error occurred")
        })?;

        // Create the resource
        let handle = table.create_resource(type_idx, data)?;

        // Set the name if we have access to the resource
        if let Ok(res) = table.get_resource(handle) {
            if let Ok(mut res_guard) = res.lock() {
                res_guard.name = Some(name.to_string();
            }
        }

        // Add to arena's managed resources
        if self.resources.len() >= MAX_ARENA_RESOURCES {
            // Clean up the resource we just created since we can't track it
            let _ = table.drop_resource(handle;
            return Err(Error::runtime_execution_error("Error occurred"))
        }
        self.resources.push(handle).map_err(|_| {
            // Clean up the resource we just created since we can't track it
            let _ = table.drop_resource(handle;
            Error::resource_error("Error occurred")
        })?;

        Ok(handle)
    }

    /// Get access to a resource
    pub fn get_resource(&self, handle: u32) -> Result<Box<Mutex<super::Resource>>> {
        let table = self.table.lock().map_err(|e| {
            Error::runtime_poisoned_lock("Error occurred")
        })?;

        table.get_resource(handle)
    }

    /// Check if a resource exists
    pub fn has_resource(&self, id: ResourceId) -> Result<bool> {
        // First check if it's in our arena
        let contains = self.resources.contains(&id.0;
        if !contains {
            return Ok(false;
        }

        // Then check if it exists in the table
        let table = self.table.lock().map_err(|e| {
            Error::runtime_poisoned_lock("Error occurred")
        })?;

        match table.get_resource(id.0) {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    /// Remove a resource from this arena's management
    ///
    /// This does not drop the resource, just removes it from the arena.
    /// Returns true if the resource was found and removed.
    pub fn remove_resource(&mut self, handle: u32) -> bool {
        if let Some(pos) = self.resources.iter().position(|&h| h == handle) {
            self.resources.remove(pos;
            true
        } else {
            false
        }
    }

    /// Drop a specific resource and remove it from this arena
    pub fn drop_resource(&mut self, handle: u32) -> Result<()> {
        // First remove it from our tracking
        if !self.remove_resource(handle) {
            return Err(Error::resource_error("Error occurred";
        }

        // Then drop it from the table
        let mut table = self.table.lock().map_err(|e| {
            Error::runtime_poisoned_lock("Error occurred")
        })?;

        table.drop_resource(handle)
    }

    /// Release all resources managed by this arena
    pub fn release_all(&mut self) -> Result<()> {
        if self.resources.is_empty() {
            return Ok();
        }

        let mut table = self.table.lock().map_err(|e| {
            Error::runtime_poisoned_lock("Error occurred")
        })?;

        let mut error = None;

        // Try to drop all resources, continuing even if some fail
        for handle in self.resources.iter() {
            if let Err(e) = table.drop_resource(*handle) {
                // Store the first error but continue trying to drop others
                if error.is_none() {
                    error = Some(e;
                }
            }
        }

        // Clear our resources list regardless of errors
        self.resources.clear);

        // Return the first error if any occurred
        if let Some(e) = error {
            Err(e)
        } else {
            Ok(())
        }
    }

    /// Get the number of resources in this arena
    pub fn resource_count(&self) -> usize {
        self.resources.len()
    }

    /// Get the name of this arena
    pub fn name(&self) -> Option<&str> {
        self.name
    }

    /// Get all resources managed by this arena
    pub fn resources(&self) -> &[u32] {
        self.resources.as_slice()
    }
}

impl<'a> Drop for ResourceArena<'a> {
    fn drop(&mut self) {
        // Try to release all resources, ignoring errors
        let _ = self.release_all);
    }
}

impl<'a> Debug for ResourceArena<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ResourceArena")
            .field("name", &self.name)
            .field("resource_count", &self.resources.len())
            .field("resources", &self.resources.as_slice())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_and_release() {
        // Create a resource table
        let table = Mutex::new(ResourceTable::new().unwrap();

        // Create an arena
        let mut arena = ResourceArena::new(&table).unwrap();

        // Create some resources
        let handle1 = arena.create_resource(1, Box::new("test1".to_string())).unwrap();
        let handle2 = arena.create_resource(2, Box::new(42)).unwrap();

        // Verify they exist
        assert!(arena.has_resource(ResourceId(handle1)).unwrap();
        assert!(arena.has_resource(ResourceId(handle2)).unwrap();

        // Verify count
        assert_eq!(arena.resource_count(), 2;

        // Release all
        arena.release_all().unwrap();

        // Verify resources are gone
        assert_eq!(arena.resource_count(), 0;

        // Verify they no longer exist in the table
        let locked_table = table.lock().unwrap();
        assert!(locked_table.get_resource(handle1).is_err();
        assert!(locked_table.get_resource(handle2).is_err();
    }

    #[test]
    fn test_drop_specific_resource() {
        // Create a resource table
        let table = Mutex::new(ResourceTable::new().unwrap();

        // Create an arena
        let mut arena = ResourceArena::new(&table).unwrap();

        // Create some resources
        let handle1 = arena.create_resource(1, Box::new("test1".to_string())).unwrap();
        let handle2 = arena.create_resource(2, Box::new(42)).unwrap();

        // Drop one resource
        arena.drop_resource(handle1).unwrap();

        // Verify it's gone
        assert!(!arena.has_resource(ResourceId(handle1)).unwrap();

        // But the other one should still exist
        assert!(arena.has_resource(ResourceId(handle2)).unwrap();

        // Verify count
        assert_eq!(arena.resource_count(), 1;
    }

    #[test]
    fn test_auto_release_on_drop() {
        // Create a resource table
        let table = Mutex::new(ResourceTable::new().unwrap();

        // Create resources in a scope
        {
            let mut arena = ResourceArena::new(&table).unwrap();
            let handle = arena.create_resource(1, Box::new("test".to_string())).unwrap();

            // Verify it exists
            assert!(arena.has_resource(ResourceId(handle)).unwrap();

            // Arena will be dropped here
        }

        // Verify resource no longer exists in the table
        let locked_table = table.lock().unwrap();
        assert_eq!(locked_table.resource_count(), 0;
    }

    #[test]
    fn test_named_resource() {
        // Create a resource table
        let table = Mutex::new(ResourceTable::new().unwrap();

        // Create an arena with name
        let mut arena = ResourceArena::new_with_name(&table, "test-arena").unwrap();

        // Create a named resource
        let handle = arena.create_named_resource(1, Box::new(42), "answer").unwrap();

        // Get the resource and check the name
        let resource = arena.get_resource(handle).unwrap();
        let guard = resource.lock().unwrap();

        assert_eq!(guard.name, Some("answer".to_string();

        // Check arena name
        assert_eq!(arena.name(), Some("test-arena";
    }

    #[test]
    fn test_resource_capacity() {
        // Create a resource table
        let table = Mutex::new(ResourceTable::new().unwrap();

        // Create an arena
        let mut arena = ResourceArena::new(&table).unwrap();

        // Create MAX_ARENA_RESOURCES resources
        for i in 0..MAX_ARENA_RESOURCES {
            let _ = arena.create_resource(1, Box::new(i)).unwrap();
        }

        // Verify count
        assert_eq!(arena.resource_count(), MAX_ARENA_RESOURCES;

        // Try to create one more - should fail
        let result = arena.create_resource(1, Box::new(100;
        assert!(result.is_err();
    }
}
