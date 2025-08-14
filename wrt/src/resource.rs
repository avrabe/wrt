//! WebAssembly Component Model resource types
//!
//! This module contains implementations for the WebAssembly Component Model
//! resource types, including resource tables, lifetime management, and
//! reference counting.

#[cfg(not(feature = "std"))]
use alloc::sync::Arc;
#[cfg(not(feature = "std"))]
use alloc::vec::Vec;
#[cfg(not(feature = "std"))]
use core::{
    any::Any,
    cmp::{
        Eq,
        PartialEq,
    },
    fmt,
};
#[cfg(feature = "std")]
use std::sync::Arc;
#[cfg(all(feature = "std", not(feature = "safety-critical")))]
use std::vec::Vec;
#[cfg(feature = "std")]
use std::{
    any::Any,
    cmp::{
        Eq,
        PartialEq,
    },
    fmt,
};

// Conditional imports for WRT allocator
#[cfg(all(feature = "std", feature = "safety-critical"))]
use wrt_foundation::allocator::{
    CrateId,
    WrtVec,
};

use crate::{
    bounded_wrt_infra::{
        new_loaded_module_vec,
        BoundedLoadedModuleVec,
        WrtProvider,
    },
    error::{
        kinds,
        Error,
        Result,
    },
    prelude::{
        format,
        Mutex,
        String,
    },
};

/// A unique identifier for a resource instance
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ResourceId(pub u32);

/// A resource type with metadata
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceType {
    /// Name of the resource type
    pub name:           String,
    /// Resource representation type (typically a handle or other data)
    pub representation: ResourceRepresentation,
    /// Whether the resource is nullable
    pub nullable:       bool,
    /// Whether the resource is borrowable
    pub borrowable:     bool,
}

/// Resource representation types
#[derive(Debug, Clone)]
pub enum ResourceRepresentation {
    /// Represented as a 32-bit integer handle
    Handle32,
    /// Represented as a 64-bit integer handle
    Handle64,
    /// Represented as a specific record type
    #[cfg(all(feature = "std", feature = "safety-critical"))]
    Record(WrtVec<String, { CrateId::Wrt as u8 }, 32>),
    #[cfg(not(all(feature = "std", feature = "safety-critical")))]
    Record(Vec<String>),
    /// Aggregated resource (composed of other resources)
    #[cfg(all(feature = "std", feature = "safety-critical"))]
    Aggregate(WrtVec<ResourceType, { CrateId::Wrt as u8 }, 16>),
    #[cfg(not(all(feature = "std", feature = "safety-critical")))]
    Aggregate(Vec<ResourceType>),
}

impl PartialEq for ResourceRepresentation {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Handle32, Self::Handle32) => true,
            (Self::Handle64, Self::Handle64) => true,
            (Self::Record(a), Self::Record(b)) => a == b,
            (Self::Aggregate(a), Self::Aggregate(b)) => {
                if a.len() != b.len() {
                    return false;
                }

                for (a_item, b_item) in a.iter().zip(b.iter()) {
                    if a_item.name != b_item.name
                        || a_item.nullable != b_item.nullable
                        || a_item.borrowable != b_item.borrowable
                        || a_item.representation != b_item.representation
                    {
                        return false;
                    }
                }
                true
            },
            _ => false,
        }
    }
}

impl Eq for ResourceRepresentation {}

/// A resource instance
#[derive(Debug)]
pub struct Resource {
    /// Resource type
    pub resource_type: ResourceType,
    /// Resource ID
    pub id:            ResourceId,
    /// Resource data (implementation-specific)
    pub data:          Arc<dyn ResourceData>,
    /// Reference count
    ref_count:         usize,
}

/// Trait for resource data
pub trait ResourceData: fmt::Debug + Send + Sync {
    /// Downcast this resource to a concrete type
    fn as_any(&self) -> &dyn std::any::Any;
}

/// Resource table for tracking resource instances
#[derive(Debug, Default)]
pub struct ResourceTable {
    /// Resources indexed by ID using bounded collections
    resources: BoundedLoadedModuleVec<Option<Resource>>,
    /// Next available resource ID
    next_id:   u32,
}

impl ResourceRepresentation {
    /// Create a new Record representation with bounded capacity
    #[cfg(feature = "safety-critical")]
    pub fn new_record() -> Result<Self> {
        Ok(Self::Record(WrtVec::new()))
    }

    #[cfg(not(feature = "safety-critical"))]
    pub fn new_record() -> Result<Self> {
        Ok(Self::Record(Vec::new()))
    }

    /// Create a new Aggregate representation with bounded capacity
    #[cfg(feature = "safety-critical")]
    pub fn new_aggregate() -> Result<Self> {
        Ok(Self::Aggregate(WrtVec::new()))
    }

    #[cfg(not(feature = "safety-critical"))]
    pub fn new_aggregate() -> Result<Self> {
        Ok(Self::Aggregate(Vec::new()))
    }

    /// Add a field to a Record representation
    pub fn add_field(&mut self, field_name: String) -> Result<()> {
        match self {
            #[cfg(feature = "safety-critical")]
            Self::Record(fields) => fields.push(field_name).map_err(|_| {
                kinds::ExecutionError("Record field capacity exceeded (limit: 32)".to_string())
                    .into()
            }),
            #[cfg(not(feature = "safety-critical"))]
            Self::Record(fields) => {
                fields.push(field_name);
                Ok(())
            },
            _ => Err(kinds::ExecutionError(
                "Cannot add field to non-Record representation".to_string(),
            )
            .into()),
        }
    }

    /// Add a resource to an Aggregate representation
    pub fn add_resource(&mut self, resource_type: ResourceType) -> Result<()> {
        match self {
            #[cfg(feature = "safety-critical")]
            Self::Aggregate(resources) => resources.push(resource_type).map_err(|_| {
                kinds::ExecutionError(
                    "Aggregate resource capacity exceeded (limit: 16)".to_string(),
                )
                .into()
            }),
            #[cfg(not(feature = "safety-critical"))]
            Self::Aggregate(resources) => {
                resources.push(resource_type);
                Ok(())
            },
            _ => Err(kinds::ExecutionError(
                "Cannot add resource to non-Aggregate representation".to_string(),
            )
            .into()),
        }
    }
}

impl ResourceTable {
    /// Creates a new resource table
    #[must_use]
    pub fn new() -> Self {
        Self {
            resources: new_loaded_module_vec(),
            next_id:   1, // Start at 1, 0 can be used as a null handle
        }
    }

    /// Allocates a new resource in the table
    pub fn allocate(
        &mut self,
        resource_type: ResourceType,
        data: Arc<dyn ResourceData>,
    ) -> ResourceId {
        let id = ResourceId(self.next_id);
        self.next_id += 1;

        let resource = Resource {
            resource_type,
            id,
            data,
            ref_count: 1,
        };

        // Find an empty slot or add to the end
        let index = self.resources.iter().position(std::option::Option::is_none);
        if let Some(index) = index {
            self.resources[index] = Some(resource);
        } else {
            self.resources.push(Some(resource));
        }

        id
    }

    /// Gets a resource by ID
    pub fn get(&self, id: ResourceId) -> Result<&Resource> {
        let index = id.0 as usize - 1;
        if index >= self.resources.len() {
            return Err(kinds::ExecutionError(format!("Invalid resource ID: {:?}", id)).into());
        }

        if let Some(ref resource) = self.resources[index] {
            Ok(resource)
        } else {
            Err(kinds::ExecutionError(format!("Resource not found: {:?}", id)).into())
        }
    }

    /// Gets a mutable resource by ID
    pub fn get_mut(&mut self, id: ResourceId) -> Result<&mut Resource> {
        let index = id.0 as usize - 1;
        if index >= self.resources.len() {
            return Err(kinds::ExecutionError(format!("Invalid resource ID: {:?}", id)).into());
        }

        if let Some(ref mut resource) = self.resources[index] {
            Ok(resource)
        } else {
            Err(kinds::ExecutionError(format!("Resource not found: {:?}", id)).into())
        }
    }

    /// Adds a reference to a resource
    pub fn add_ref(&mut self, id: ResourceId) -> Result<()> {
        let resource = self.get_mut(id)?;
        resource.ref_count += 1;
        Ok(())
    }

    /// Drops a reference to a resource
    pub fn drop_ref(&mut self, id: ResourceId) -> Result<()> {
        let index = id.0 as usize - 1;
        if index >= self.resources.len() {
            return Err(kinds::ExecutionError(format!("Invalid resource ID: {:?}", id)).into());
        }

        let drop_resource = {
            let resource = self.get_mut(id)?;
            resource.ref_count -= 1;
            resource.ref_count == 0
        };

        if drop_resource {
            self.resources[index] = None;
        }

        Ok(())
    }

    /// Counts the number of resources in the table
    #[must_use]
    pub fn count(&self) -> usize {
        self.resources.iter().filter(|r| r.is_some()).count()
    }
}

/// Simple resource data implementation for testing
#[derive(Debug)]
pub struct SimpleResourceData {
    /// Value stored in the resource
    pub value: u64,
}

impl ResourceData for SimpleResourceData {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_resource_type() -> ResourceType {
        ResourceType {
            name:           String::from("test:resource"),
            representation: ResourceRepresentation::Handle32,
            nullable:       false,
            borrowable:     true,
        }
    }

    #[test]
    fn test_resource_allocation() {
        let mut table = ResourceTable::new();
        let resource_type = create_test_resource_type();

        let data = Arc::new(SimpleResourceData { value: 42 });
        let id = table.allocate(resource_type.clone(), data.clone());

        assert_eq!(id.0, 1);
        assert_eq!(table.count(), 1);

        let resource = table.get(id).unwrap();
        assert_eq!(resource.id.0, 1);
        assert_eq!(resource.resource_type.name, "test:resource");
        assert_eq!(resource.ref_count, 1);
    }

    #[test]
    fn test_resource_reference_counting() -> Result<()> {
        let mut table = ResourceTable::new();
        let resource_type = create_test_resource_type();

        let data = Arc::new(SimpleResourceData { value: 42 });
        let id = table.allocate(resource_type, data);

        // Add references
        table.add_ref(id)?;
        table.add_ref(id)?;

        let resource = table.get(id)?;
        assert_eq!(resource.ref_count, 3);

        // Drop references
        table.drop_ref(id)?;
        table.drop_ref(id)?;

        let resource = table.get(id)?;
        assert_eq!(resource.ref_count, 1);

        // Final drop should remove the resource
        table.drop_ref(id)?;
        assert_eq!(table.count(), 0);

        // Resource should no longer be accessible
        assert!(table.get(id).is_err());

        Ok(())
    }

    #[test]
    fn test_multiple_resources() {
        let mut table = ResourceTable::new();
        let resource_type = create_test_resource_type();

        let data1 = Arc::new(SimpleResourceData { value: 42 });
        let id1 = table.allocate(resource_type.clone(), data1);

        let data2 = Arc::new(SimpleResourceData { value: 84 });
        let id2 = table.allocate(resource_type.clone(), data2);

        assert_eq!(id1.0, 1);
        assert_eq!(id2.0, 2);
        assert_eq!(table.count(), 2);

        // Resources should have correct data
        let resource1 = table.get(id1).unwrap();
        let data1 = resource1.data.as_any().downcast_ref::<SimpleResourceData>().unwrap();
        assert_eq!(data1.value, 42);

        let resource2 = table.get(id2).unwrap();
        let data2 = resource2.data.as_any().downcast_ref::<SimpleResourceData>().unwrap();
        assert_eq!(data2.value, 84);
    }
}
