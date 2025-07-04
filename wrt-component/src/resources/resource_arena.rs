// WRT - wrt-component
// Copyright (c) 2025 Ralf Anton Beier
// Licensed under the MIT license.
// SPDX-License-Identifier: MIT

use wrt_error::kinds::PoisonedLockError;

use super::{ResourceId, ResourceTable};
use crate::prelude::*;

/// A resource arena for managing resource lifecycles as a group
///
/// ResourceArena provides efficient group management of resources, allowing
/// Binary std/no_std choice
/// particularly useful for component instances and other scenarios where
/// resources have a shared lifetime.
pub struct ResourceArena {
    /// Handles to resources managed by this arena
    resources: Vec<u32>,
    /// The resource table used for actual resource management
    table: Arc<Mutex<ResourceTable>>,
    /// Name of this arena, for debugging
    name: Option<String>,
}

impl ResourceArena {
    /// Create a new resource arena with the given resource table
    pub fn new(table: Arc<Mutex<ResourceTable>>) -> Self {
        Self { resources: Vec::new(), table, name: None }
    }

    /// Create a new resource arena with the given name
    pub fn new_with_name(table: Arc<Mutex<ResourceTable>>, name: &str) -> Self {
        Self { resources: Vec::new(), table, name: Some(name.to_string()) }
    }

    /// Create a resource in this arena
    ///
    /// The resource will be automatically cleaned up when the arena is dropped
    /// or when release_all() is called.
    pub fn create_resource(
        &mut self,
        type_idx: u32,
        data: Arc<dyn Any + Send + Sync>,
    ) -> Result<u32> {
        let mut table = self.table.lock().map_err(|e| {
            Error::runtime_poisoned_lock("Component not found"),
            )
        })?;

        let handle = table.create_resource(type_idx, data)?;
        self.resources.push(handle);
        Ok(handle)
    }

    /// Create a named resource in this arena
    pub fn create_named_resource(
        &mut self,
        type_idx: u32,
        data: Arc<dyn Any + Send + Sync>,
        name: &str,
    ) -> Result<u32> {
        let mut table = self.table.lock().map_err(|e| {
            Error::runtime_poisoned_lock("Component not found"),
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

        // Add to arena's managed resources
        self.resources.push(handle);

        Ok(handle)
    }

    /// Create a resource in this arena using a ResourceId
    ///
    /// Convenience method for APIs that use ResourceId
    pub fn add_resource<T: 'static + Send + Sync>(&mut self, resource: T) -> Result<ResourceId> {
        let handle = self.create_resource(0, Arc::new(resource))?;
        Ok(ResourceId(handle))
    }

    /// Get access to a resource
    pub fn get_resource(&self, handle: u32) -> Result<Arc<Mutex<super::Resource>>> {
        let table = self.table.lock().map_err(|e| {
            Error::runtime_poisoned_lock("Component not found"),
            )
        })?;

        table.get_resource(handle)
    }

    /// Check if a resource exists
    pub fn has_resource(&self, id: ResourceId) -> Result<bool> {
        let table = self.table.lock().map_err(|e| {
            Error::runtime_poisoned_lock("Component not found"),
            )
        })?;

        // First check if it's in our arena
        if !self.resources.contains(&id.0) {
            return Ok(false);
        }

        // Then check if it exists in the table
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
            self.resources.swap_remove(pos);
            true
        } else {
            false
        }
    }

    /// Drop a specific resource and remove it from this arena
    pub fn drop_resource(&mut self, handle: u32) -> Result<()> {
        // First remove it from our tracking
        if !self.remove_resource(handle) {
            return Err(Error::resource_error("Component not found"));
        }

        // Then drop it from the table
        let mut table = self.table.lock().map_err(|e| {
            Error::runtime_poisoned_lock("Component not found"),
            )
        })?;

        table.drop_resource(handle)
    }

    /// Release all resources managed by this arena
    pub fn release_all(&mut self) -> Result<()> {
        if self.resources.is_empty() {
            return Ok(());
        }

        let mut table = self.table.lock().map_err(|e| {
            Error::runtime_poisoned_lock("Component not found"),
            )
        })?;

        let mut error = None;

        // Try to drop all resources, continuing even if some fail
        for handle in self.resources.drain(..) {
            if let Err(e) = table.drop_resource(handle) {
                // Store the first error but continue trying to drop others
                if error.is_none() {
                    error = Some(e);
                }
            }
        }

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
        self.name.as_deref()
    }

    /// Set the name of this arena
    pub fn set_name(&mut self, name: &str) {
        self.name = Some(name.to_string());
    }

    /// Get all resources managed by this arena
    pub fn resources(&self) -> &[u32] {
        &self.resources
    }
}

impl Drop for ResourceArena {
    fn drop(&mut self) {
        // Try to release all resources, ignoring errors
        let _ = self.release_all();
    }
}

impl fmt::Debug for ResourceArena {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ResourceArena")
            .field("name", &self.name)
            .field("resource_count", &self.resources.len())
            .field("resources", &self.resources)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_and_release() {
        // Create a resource table
        let table = Arc::new(Mutex::new(ResourceTable::new()));

        // Create an arena
        let mut arena = ResourceArena::new(table.clone());

        // Create some resources
        let handle1 = arena.create_resource(1, Arc::new("test1".to_string())).unwrap();
        let handle2 = arena.create_resource(2, Arc::new(42)).unwrap();

        // Verify they exist
        assert!(arena.has_resource(ResourceId(handle1)).unwrap());
        assert!(arena.has_resource(ResourceId(handle2)).unwrap());

        // Verify count
        assert_eq!(arena.resource_count(), 2);

        // Release all
        arena.release_all().unwrap();

        // Verify resources are gone
        assert_eq!(arena.resource_count(), 0);

        // Verify they no longer exist in the table
        let locked_table = table.lock().unwrap();
        assert!(locked_table.get_resource(handle1).is_err());
        assert!(locked_table.get_resource(handle2).is_err());
    }

    #[test]
    fn test_drop_specific_resource() {
        // Create a resource table
        let table = Arc::new(Mutex::new(ResourceTable::new()));

        // Create an arena
        let mut arena = ResourceArena::new(table.clone());

        // Create some resources
        let handle1 = arena.create_resource(1, Arc::new("test1".to_string())).unwrap();
        let handle2 = arena.create_resource(2, Arc::new(42)).unwrap();

        // Drop one resource
        arena.drop_resource(handle1).unwrap();

        // Verify it's gone
        assert!(!arena.has_resource(ResourceId(handle1)).unwrap());

        // But the other one should still exist
        assert!(arena.has_resource(ResourceId(handle2)).unwrap());

        // Verify count
        assert_eq!(arena.resource_count(), 1);
    }

    #[test]
    fn test_auto_release_on_drop() {
        // Create a resource table
        let table = Arc::new(Mutex::new(ResourceTable::new()));

        // Create resources in a scope
        {
            let mut arena = ResourceArena::new(table.clone());
            let handle = arena.create_resource(1, Arc::new("test".to_string())).unwrap();

            // Verify it exists
            assert!(arena.has_resource(ResourceId(handle)).unwrap());

            // Arena will be dropped here
        }

        // Verify resource no longer exists in the table
        let locked_table = table.lock().unwrap();
        assert_eq!(locked_table.resource_count(), 0);
    }

    #[test]
    fn test_named_resource() {
        // Create a resource table
        let table = Arc::new(Mutex::new(ResourceTable::new()));

        // Create an arena
        let mut arena = ResourceArena::new_with_name(table, "test-arena");

        // Create a named resource
        let handle = arena.create_named_resource(1, Arc::new(42), "answer").unwrap();

        // Get the resource and check the name
        let resource = arena.get_resource(handle).unwrap();
        let guard = resource.lock().unwrap();

        assert_eq!(guard.name, Some("answer".to_string()));

        // Check arena name
        assert_eq!(arena.name(), Some("test-arena"));
    }

    #[test]
    fn test_multiple_arenas() {
        // Create a resource table
        let table = Arc::new(Mutex::new(ResourceTable::new()));

        // Create two arenas
        let mut arena1 = ResourceArena::new_with_name(table.clone(), "arena1");
        let mut arena2 = ResourceArena::new_with_name(table.clone(), "arena2");

        // Add resources to each
        let handle1 = arena1.create_resource(1, Arc::new("test1".to_string())).unwrap();
        let handle2 = arena2.create_resource(2, Arc::new("test2".to_string())).unwrap();

        // Resource should only exist in its arena
        assert!(arena1.has_resource(ResourceId(handle1)).unwrap());
        assert!(!arena1.has_resource(ResourceId(handle2)).unwrap());

        assert!(!arena2.has_resource(ResourceId(handle1)).unwrap());
        assert!(arena2.has_resource(ResourceId(handle2)).unwrap());

        // Release one arena
        arena1.release_all().unwrap();

        // Resources from arena1 should be gone, but arena2's should remain
        let locked_table = table.lock().unwrap();
        assert!(locked_table.get_resource(handle1).is_err());
        assert!(locked_table.get_resource(handle2).is_ok());
    }
}
