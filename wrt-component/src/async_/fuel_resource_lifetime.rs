//! Resource lifetime management with fuel tracking
//!
//! This module provides deterministic resource lifetime management for async
//! operations, ensuring resources are properly tracked and cleaned up.

use crate::{
    async_::{
        fuel_async_executor::{FuelAsyncTask, AsyncTaskState},
        fuel_error_context::{AsyncErrorKind, async_error},
    },
    prelude::*,
};
use core::{
    sync::atomic::{AtomicU64, AtomicBool, Ordering},
    marker::PhantomData,
};
use wrt_foundation::{
    bounded_collections::{BoundedHashMap, BoundedVec},
    operations::{record_global_operation, Type as OperationType},
    verification::VerificationLevel,
    safe_managed_alloc, CrateId,
    Arc, Weak,
};

/// Maximum number of resources per component
const MAX_RESOURCES_PER_COMPONENT: usize = 256;

/// Maximum resource reference count
const MAX_RESOURCE_REFS: u64 = 1000;

/// Fuel costs for resource operations
const RESOURCE_CREATE_FUEL: u64 = 15;
const RESOURCE_ACQUIRE_FUEL: u64 = 5;
const RESOURCE_RELEASE_FUEL: u64 = 5;
const RESOURCE_DROP_FUEL: u64 = 10;
const RESOURCE_TRANSFER_FUEL: u64 = 8;

/// Resource handle type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ResourceHandle(pub u64);

/// Resource state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceState {
    /// Resource is available for use
    Available,
    /// Resource is currently in use
    InUse,
    /// Resource is being transferred
    Transferring,
    /// Resource has been dropped
    Dropped,
    /// Resource is in error state
    Error,
}

/// Resource metadata
#[derive(Debug)]
pub struct ResourceMetadata {
    /// Resource type name
    pub type_name: String,
    /// Component that owns the resource
    pub owner_component: u64,
    /// Task that created the resource
    pub creator_task: Option<u64>,
    /// Creation timestamp (in fuel units)
    pub created_at: u64,
    /// Last access timestamp (in fuel units)
    pub last_accessed: AtomicU64,
}

/// Tracked resource with lifetime management
pub struct TrackedResource<T> {
    /// Resource handle
    pub handle: ResourceHandle,
    /// Resource data
    pub data: Option<T>,
    /// Resource state
    pub state: ResourceState,
    /// Reference count
    pub ref_count: AtomicU64,
    /// Metadata
    pub metadata: ResourceMetadata,
    /// Fuel consumed by this resource
    pub fuel_consumed: AtomicU64,
    /// Verification level
    pub verification_level: VerificationLevel,
    /// Cleanup registered flag
    pub cleanup_registered: AtomicBool,
}

impl<T> TrackedResource<T> {
    /// Create a new tracked resource
    pub fn new(
        handle: ResourceHandle,
        data: T,
        owner_component: u64,
        creator_task: Option<u64>,
        type_name: String,
        verification_level: VerificationLevel,
    ) -> Result<Self> {
        let created_at = wrt_foundation::operations::global_fuel_consumed();
        
        // Record resource creation
        record_global_operation(OperationType::Other)?;
        
        Ok(Self {
            handle,
            data: Some(data),
            state: ResourceState::Available,
            ref_count: AtomicU64::new(1),
            metadata: ResourceMetadata {
                type_name,
                owner_component,
                creator_task,
                created_at,
                last_accessed: AtomicU64::new(created_at),
            },
            fuel_consumed: AtomicU64::new(RESOURCE_CREATE_FUEL),
            verification_level,
            cleanup_registered: AtomicBool::new(false),
        })
    }
    
    /// Acquire a reference to the resource
    pub fn acquire(&self) -> Result<()> {
        // Check state
        match self.state {
            ResourceState::Available | ResourceState::InUse => {},
            _ => return Err(Error::new(
                ErrorCategory::Resource,
                codes::RESOURCE_ACCESS_ERROR,
                "Resource not available for acquisition",
            )),
        }
        
        // Increment reference count
        let old_count = self.ref_count.fetch_add(1, Ordering::AcqRel);
        if old_count >= MAX_RESOURCE_REFS {
            self.ref_count.fetch_sub(1, Ordering::AcqRel);
            return Err(Error::new(
                ErrorCategory::Resource,
                codes::RESOURCE_LIMIT_EXCEEDED,
                "Resource reference count exceeded",
            ));
        }
        
        // Update last accessed time and consume fuel
        self.metadata.last_accessed.store(
            wrt_foundation::operations::global_fuel_consumed(),
            Ordering::Release
        );
        self.fuel_consumed.fetch_add(RESOURCE_ACQUIRE_FUEL, Ordering::AcqRel);
        
        Ok(())
    }
    
    /// Release a reference to the resource
    pub fn release(&self) -> Result<bool> {
        let old_count = self.ref_count.fetch_sub(1, Ordering::AcqRel);
        if old_count == 0 {
            return Err(Error::new(
                ErrorCategory::Resource,
                codes::RESOURCE_ERROR,
                "Resource reference count underflow",
            ));
        }
        
        // Consume fuel
        self.fuel_consumed.fetch_add(RESOURCE_RELEASE_FUEL, Ordering::AcqRel);
        
        // Return true if this was the last reference
        Ok(old_count == 1)
    }
    
    /// Get current reference count
    pub fn ref_count(&self) -> u64 {
        self.ref_count.load(Ordering::Acquire)
    }
    
    /// Check if resource can be dropped
    pub fn can_drop(&self) -> bool {
        self.ref_count() == 0 && self.state != ResourceState::Dropped
    }
}

/// Resource guard for RAII-style resource management
pub struct ResourceGuard<T> {
    resource: Arc<TrackedResource<T>>,
    released: bool,
}

impl<T> ResourceGuard<T> {
    /// Create a new resource guard
    pub fn new(resource: Arc<TrackedResource<T>>) -> Result<Self> {
        resource.acquire()?;
        Ok(Self {
            resource,
            released: false,
        })
    }
    
    /// Get reference to the resource data
    pub fn get(&self) -> Result<&T> {
        self.resource.data.as_ref().ok_or_else(|| {
            Error::new(
                ErrorCategory::Resource,
                codes::RESOURCE_NOT_FOUND,
                "Resource data not available",
            )
        })
    }
    
    /// Release the guard early
    pub fn release(mut self) -> Result<()> {
        if !self.released {
            self.resource.release()?;
            self.released = true;
        }
        Ok(())
    }
}

impl<T> Drop for ResourceGuard<T> {
    fn drop(&mut self) {
        if !self.released {
            let _ = self.resource.release();
        }
    }
}

/// Resource lifetime manager
pub struct ResourceLifetimeManager {
    /// Resources by handle
    resources: BoundedHashMap<ResourceHandle, Arc<dyn core::any::Any + Send + Sync>, MAX_RESOURCES_PER_COMPONENT>,
    /// Next resource handle
    next_handle: AtomicU64,
    /// Component ID
    component_id: u64,
    /// Global fuel budget for resources
    global_fuel_budget: u64,
    /// Total fuel consumed
    total_fuel_consumed: AtomicU64,
    /// Cleanup callbacks
    cleanup_callbacks: BoundedVec<Box<dyn FnOnce() + Send>, MAX_RESOURCES_PER_COMPONENT>,
}

impl ResourceLifetimeManager {
    /// Create a new resource lifetime manager
    pub fn new(component_id: u64, global_fuel_budget: u64) -> Result<Self> {
        let provider = safe_managed_alloc!(8192, CrateId::Component)?;
        let resources = BoundedHashMap::new(provider.clone())?;
        let cleanup_callbacks = BoundedVec::new(provider)?;
        
        Ok(Self {
            resources,
            next_handle: AtomicU64::new(1),
            component_id,
            global_fuel_budget,
            total_fuel_consumed: AtomicU64::new(0),
            cleanup_callbacks,
        })
    }
    
    /// Create a new resource
    pub fn create_resource<T: Send + Sync + 'static>(
        &mut self,
        data: T,
        creator_task: Option<u64>,
        type_name: &str,
        verification_level: VerificationLevel,
    ) -> Result<ResourceHandle> {
        // Check fuel budget
        let current_fuel = self.total_fuel_consumed.load(Ordering::Acquire);
        if current_fuel.saturating_add(RESOURCE_CREATE_FUEL) > self.global_fuel_budget {
            return Err(Error::new(
                ErrorCategory::Resource,
                codes::RESOURCE_LIMIT_EXCEEDED,
                "Resource fuel budget exceeded",
            ));
        }
        
        // Generate handle
        let handle = ResourceHandle(self.next_handle.fetch_add(1, Ordering::AcqRel));
        
        // Create tracked resource
        let resource = TrackedResource::new(
            handle,
            data,
            self.component_id,
            creator_task,
            type_name.to_string(),
            verification_level,
        )?;
        
        let arc_resource = Arc::new(resource);
        
        // Store resource
        self.resources.insert(handle, arc_resource.clone() as Arc<dyn core::any::Any + Send + Sync>)?;
        
        // Update fuel consumption
        self.total_fuel_consumed.fetch_add(RESOURCE_CREATE_FUEL, Ordering::AcqRel);
        
        Ok(handle)
    }
    
    /// Get a resource by handle
    pub fn get_resource<T: Send + Sync + 'static>(
        &self,
        handle: ResourceHandle,
    ) -> Result<Arc<TrackedResource<T>>> {
        let resource = self.resources.get(&handle).ok_or_else(|| {
            Error::new(
                ErrorCategory::Resource,
                codes::RESOURCE_NOT_FOUND,
                "Resource not found",
            )
        })?;
        
        // Downcast to specific type
        resource.clone()
            .downcast::<TrackedResource<T>>()
            .map_err(|_| Error::new(
                ErrorCategory::Type,
                codes::TYPE_MISMATCH,
                "Resource type mismatch",
            ))
    }
    
    /// Transfer resource ownership
    pub fn transfer_resource(
        &mut self,
        handle: ResourceHandle,
        new_owner: u64,
    ) -> Result<()> {
        // Check fuel
        let current_fuel = self.total_fuel_consumed.load(Ordering::Acquire);
        if current_fuel.saturating_add(RESOURCE_TRANSFER_FUEL) > self.global_fuel_budget {
            return Err(Error::new(
                ErrorCategory::Resource,
                codes::RESOURCE_LIMIT_EXCEEDED,
                "Resource fuel budget exceeded",
            ));
        }
        
        // Remove from our resources
        let resource = self.resources.remove(&handle).ok_or_else(|| {
            Error::new(
                ErrorCategory::Resource,
                codes::RESOURCE_NOT_FOUND,
                "Resource not found for transfer",
            )
        })?;
        
        // Update fuel
        self.total_fuel_consumed.fetch_add(RESOURCE_TRANSFER_FUEL, Ordering::AcqRel);
        
        // Note: In a real implementation, we would notify the new owner
        // For now, we just remove it from our tracking
        
        Ok(())
    }
    
    /// Drop a resource
    pub fn drop_resource(&mut self, handle: ResourceHandle) -> Result<()> {
        // Remove resource
        let _resource = self.resources.remove(&handle).ok_or_else(|| {
            Error::new(
                ErrorCategory::Resource,
                codes::RESOURCE_NOT_FOUND,
                "Resource not found",
            )
        })?;
        
        // Update fuel
        self.total_fuel_consumed.fetch_add(RESOURCE_DROP_FUEL, Ordering::AcqRel);
        
        Ok(())
    }
    
    /// Register cleanup callback
    pub fn register_cleanup<F: FnOnce() + Send + 'static>(&mut self, callback: F) -> Result<()> {
        self.cleanup_callbacks.push(Box::new(callback))?;
        Ok(())
    }
    
    /// Cleanup all resources for a task
    pub fn cleanup_task_resources(&mut self, task_id: u64) -> Result<()> {
        let handles_to_drop: Vec<ResourceHandle> = self.resources
            .iter()
            .filter_map(|(handle, _resource)| {
                // In a real implementation, we would check if the resource
                // was created by this task
                Some(*handle)
            })
            .collect();
        
        for handle in handles_to_drop {
            self.drop_resource(handle)?;
        }
        
        Ok(())
    }
    
    /// Run all cleanup callbacks
    pub fn run_cleanup(&mut self) -> Result<()> {
        while let Some(callback) = self.cleanup_callbacks.pop() {
            callback();
        }
        Ok(())
    }
}

/// Resource scope for automatic cleanup
pub struct ResourceScope {
    manager: Arc<wrt_foundation::sync::Mutex<ResourceLifetimeManager>>,
    resources: Vec<ResourceHandle>,
}

impl ResourceScope {
    /// Create a new resource scope
    pub fn new(manager: Arc<wrt_foundation::sync::Mutex<ResourceLifetimeManager>>) -> Self {
        Self {
            manager,
            resources: Vec::new(),
        }
    }
    
    /// Create a resource within this scope
    pub fn create_resource<T: Send + Sync + 'static>(
        &mut self,
        data: T,
        creator_task: Option<u64>,
        type_name: &str,
        verification_level: VerificationLevel,
    ) -> Result<ResourceHandle> {
        let handle = self.manager.lock()?.create_resource(
            data,
            creator_task,
            type_name,
            verification_level,
        )?;
        
        self.resources.push(handle);
        Ok(handle)
    }
}

impl Drop for ResourceScope {
    fn drop(&mut self) {
        // Clean up all resources in reverse order
        if let Ok(mut manager) = self.manager.lock() {
            for handle in self.resources.iter().rev() {
                let _ = manager.drop_resource(*handle);
            }
        }
    }
}

/// Component resource tracker
pub struct ComponentResourceTracker {
    /// Resource managers by component ID
    managers: BoundedHashMap<u64, Arc<wrt_foundation::sync::Mutex<ResourceLifetimeManager>>, 128>,
    /// Global resource fuel budget
    global_fuel_budget: u64,
}

impl ComponentResourceTracker {
    /// Create a new component resource tracker
    pub fn new(global_fuel_budget: u64) -> Result<Self> {
        let provider = safe_managed_alloc!(4096, CrateId::Component)?;
        let managers = BoundedHashMap::new(provider)?;
        
        Ok(Self {
            managers,
            global_fuel_budget,
        })
    }
    
    /// Get or create resource manager for component
    pub fn get_or_create_manager(
        &mut self,
        component_id: u64,
    ) -> Result<Arc<wrt_foundation::sync::Mutex<ResourceLifetimeManager>>> {
        if let Some(manager) = self.managers.get(&component_id) {
            Ok(manager.clone())
        } else {
            let manager = ResourceLifetimeManager::new(
                component_id,
                self.global_fuel_budget / 10, // Each component gets 10% of global budget
            )?;
            let arc_manager = Arc::new(wrt_foundation::sync::Mutex::new(manager));
            self.managers.insert(component_id, arc_manager.clone())?;
            Ok(arc_manager)
        }
    }
    
    /// Cleanup all resources for a component
    pub fn cleanup_component(&mut self, component_id: u64) -> Result<()> {
        if let Some(manager) = self.managers.remove(&component_id) {
            if let Ok(mut manager) = manager.lock() {
                manager.run_cleanup()?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_resource_creation() {
        let mut manager = ResourceLifetimeManager::new(1, 1000).unwrap();
        
        let handle = manager.create_resource(
            42u32,
            Some(1),
            "test_resource",
            VerificationLevel::Basic,
        );
        
        assert!(handle.is_ok());
        let handle = handle.unwrap();
        assert_eq!(handle.0, 1);
    }
    
    #[test]
    fn test_resource_guard() {
        let mut manager = ResourceLifetimeManager::new(1, 1000).unwrap();
        
        let handle = manager.create_resource(
            "test_data",
            None,
            "string_resource",
            VerificationLevel::Basic,
        ).unwrap();
        
        let resource = manager.get_resource::<&str>(handle).unwrap();
        
        {
            let guard = ResourceGuard::new(resource.clone()).unwrap();
            assert_eq!(*guard.get().unwrap(), "test_data");
            assert_eq!(resource.ref_count(), 2); // Original + guard
        }
        
        // Guard dropped, ref count should decrease
        assert_eq!(resource.ref_count(), 1);
    }
    
    #[test]
    fn test_resource_scope() {
        let manager = Arc::new(wrt_foundation::sync::Mutex::new(
            ResourceLifetimeManager::new(1, 1000).unwrap()
        ));
        
        let mut scope = ResourceScope::new(manager.clone());
        
        let handle = scope.create_resource(
            vec![1, 2, 3],
            Some(1),
            "vec_resource",
            VerificationLevel::Basic,
        ).unwrap();
        
        // Verify resource exists
        assert!(manager.lock().unwrap().get_resource::<Vec<u32>>(handle).is_ok());
        
        // Scope will clean up resources when dropped
    }
    
    #[test]
    fn test_fuel_budget_enforcement() {
        let mut manager = ResourceLifetimeManager::new(1, 20).unwrap();
        
        // First resource should succeed
        let result1 = manager.create_resource(
            1u32,
            None,
            "resource1",
            VerificationLevel::Basic,
        );
        assert!(result1.is_ok());
        
        // Second resource should fail due to fuel budget
        let result2 = manager.create_resource(
            2u32,
            None,
            "resource2", 
            VerificationLevel::Basic,
        );
        assert!(result2.is_err());
    }
}