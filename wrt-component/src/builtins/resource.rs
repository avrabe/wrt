// Resource built-ins implementation for the WebAssembly Component Model
//
// This module implements the resource-related built-in functions:
// - resource.create: Create a new resource
// - resource.drop: Drop a resource
// - resource.rep: Get the representation of a resource
// - resource.get: Get a resource handle

#[cfg(feature = "std")]
use std::{
    boxed::Box,
    sync::{Arc, Mutex},
    vec::Vec,
};

#[cfg(not(feature = "std"))]
use wrt_foundation::{
    BoundedVec as Vec, 
    safe_memory::NoStdProvider,
    budget_aware_provider::CrateId,
    safe_managed_alloc,
};

// Enable vec! macro for no_std
#[cfg(not(feature = "std"))]
extern crate alloc;
#[cfg(not(feature = "std"))]
use alloc::{boxed::Box, sync::Arc, vec};
#[cfg(not(feature = "std"))]
use wrt_sync::Mutex;

use wrt_error::{Error, Result};
#[cfg(feature = "std")]
use wrt_foundation::{builtin::BuiltinType, component_value::ComponentValue};

#[cfg(not(feature = "std"))]
use crate::types::Value as ComponentValue;

#[cfg(not(feature = "std"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuiltinType {
    ResourceCreate,
    ResourceDrop,
    ResourceRep,
    ResourceGet,
}

use crate::{
    builtins::BuiltinHandler,
    resources::{ResourceId, ResourceManager},
};

/// Handler for the resource.create built-in function
pub struct ResourceCreateHandler {
    resource_manager: Arc<Mutex<ResourceManager>>,
}

impl ResourceCreateHandler {
    /// Create a new resource.create handler
    pub fn new(resource_manager: Arc<Mutex<ResourceManager>>) -> Self {
        Self { resource_manager }
    }
}

impl BuiltinHandler for ResourceCreateHandler {
    fn builtin_type(&self) -> crate::builtins::BuiltinType {
        crate::builtins::BuiltinType::ResourceCreate
    }

    fn execute(&self, args: &[ComponentValue]) -> Result<Vec<ComponentValue>> {
        // Validate args
        if args.len() != 1 {
            return Err(Error::runtime_execution_error("resource.create requires exactly one argument";
        }

        // Extract the resource representation from args
        let rep = match &args[0] {
            ComponentValue::U32(value) => *value,
            ComponentValue::U64(value) => *value as u32,
            _ => {
                return Err(Error::new(
                    wrt_error::ErrorCategory::Parameter,
                    wrt_error::codes::TYPE_MISMATCH,
                    "Expected U32 or U64 for resource representation";
            }
        };

        // Create a new resource based on the representation
        let mut manager = self.resource_manager.lock().unwrap();
        let id = manager.add_host_resource(rep;

        // Return the resource ID
        #[cfg(feature = "std")]
        {
            Ok(vec![ComponentValue::U32(id.0)])
        }
        #[cfg(not(feature = "std"))]
        {
            let provider = safe_managed_alloc!(65536, CrateId::Component)?;
            let mut result = BoundedVec::new(provider).map_err(|_| {
                Error::runtime_execution_error("Failed to create result vector")
            })?;
            result.push(ComponentValue::U32(id.0)).map_err(|_| {
                Error::runtime_execution_error("Failed to add result value")
            })?;
            Ok(result)
        }
    }

    fn clone_handler(&self) -> Box<dyn BuiltinHandler> {
        Box::new(Self { resource_manager: self.resource_manager.clone() })
    }
}

/// Handler for the resource.drop built-in function
pub struct ResourceDropHandler {
    resource_manager: Arc<Mutex<ResourceManager>>,
}

impl ResourceDropHandler {
    /// Create a new resource.drop handler
    pub fn new(resource_manager: Arc<Mutex<ResourceManager>>) -> Self {
        Self { resource_manager }
    }
}

impl BuiltinHandler for ResourceDropHandler {
    fn builtin_type(&self) -> crate::builtins::BuiltinType {
        crate::builtins::BuiltinType::ResourceDrop
    }

    fn execute(&self, args: &[ComponentValue]) -> Result<Vec<ComponentValue>> {
        // Validate args
        if args.len() != 1 {
            return Err(Error::new(
                wrt_error::ErrorCategory::Parameter,
                wrt_error::codes::EXECUTION_ERROR,
                "resource.drop requires exactly one argument";
        }

        // Extract the resource ID from args
        let id = match &args[0] {
            ComponentValue::U32(value) => ResourceId(*value),
            _ => {
                return Err(Error::runtime_execution_error("Expected U32 for resource ID";
            }
        };

        // Drop the resource
        let mut manager = self.resource_manager.lock().unwrap();
        if !manager.has_resource(id) {
            return Err(Error::new(
                wrt_error::ErrorCategory::Resource,
                wrt_error::codes::RESOURCE_NOT_FOUND,
                "Resource not found";
        }

        manager.delete_resource(id;

        // Return empty result
        Ok(vec![])
    }

    fn clone_handler(&self) -> Box<dyn BuiltinHandler> {
        Box::new(Self { resource_manager: self.resource_manager.clone() })
    }
}

/// Handler for the resource.rep built-in function
pub struct ResourceRepHandler {
    resource_manager: Arc<Mutex<ResourceManager>>,
}

impl ResourceRepHandler {
    /// Create a new resource.rep handler
    pub fn new(resource_manager: Arc<Mutex<ResourceManager>>) -> Self {
        Self { resource_manager }
    }
}

impl BuiltinHandler for ResourceRepHandler {
    fn builtin_type(&self) -> crate::builtins::BuiltinType {
        crate::builtins::BuiltinType::ResourceRep
    }

    fn execute(&self, args: &[ComponentValue]) -> Result<Vec<ComponentValue>> {
        // Validate args
        if args.len() != 1 {
            return Err(Error::runtime_execution_error("resource.rep requires exactly one argument";
        }

        // Extract the resource ID from args
        let id = match &args[0] {
            ComponentValue::U32(value) => ResourceId(*value),
            _ => {
                return Err(Error::new(
                    wrt_error::ErrorCategory::Parameter,
                    wrt_error::codes::TYPE_MISMATCH,
                    "Expected U32 or U64 for resource representation";
            }
        };

        // Get the resource representation
        let manager = self.resource_manager.lock().unwrap();
        if !manager.has_resource(id) {
            return Err(Error::runtime_execution_error("Resource not found";
        }

        // Get the resource as u32
        let resource = manager.get_host_resource::<u32>(id)?;
        let rep = *resource.lock().unwrap();

        // Return the representation
        Ok(vec![ComponentValue::U32(rep)])
    }

    fn clone_handler(&self) -> Box<dyn BuiltinHandler> {
        Box::new(Self { resource_manager: self.resource_manager.clone() })
    }
}

/// Handler for the resource.get built-in function
pub struct ResourceGetHandler {
    resource_manager: Arc<Mutex<ResourceManager>>,
}

impl ResourceGetHandler {
    /// Create a new resource.get handler
    pub fn new(resource_manager: Arc<Mutex<ResourceManager>>) -> Self {
        Self { resource_manager }
    }
}

impl BuiltinHandler for ResourceGetHandler {
    fn builtin_type(&self) -> crate::builtins::BuiltinType {
        crate::builtins::BuiltinType::ResourceGet
    }

    fn execute(&self, args: &[ComponentValue]) -> Result<Vec<ComponentValue>> {
        // Validate args
        if args.len() != 1 {
            return Err(Error::new(
                wrt_error::ErrorCategory::Parameter,
                wrt_error::codes::EXECUTION_ERROR,
                "resource.get requires exactly one argument";
        }

        // Extract the resource representation from args
        let rep = match &args[0] {
            ComponentValue::U32(value) => *value,
            ComponentValue::U64(value) => *value as u32,
            _ => {
                return Err(Error::runtime_execution_error("Expected U32 for resource ID";
            }
        };

        // Find or create resource with this representation
        let mut manager = self.resource_manager.lock().unwrap();

        // Try to find an existing resource with this rep
        for (resource_id, resource) in manager.get_resources_iter() {
            if let Ok(resource_value) = manager.get_host_resource::<u32>(*resource_id) {
                if *resource_value.lock().unwrap() == rep {
                    return Ok(vec![ComponentValue::U32(resource_id.0)];
                }
            }
        }

        // Not found, create a new one
        let id = manager.add_host_resource(rep;
        #[cfg(feature = "std")]
        {
            Ok(vec![ComponentValue::U32(id.0)])
        }
        #[cfg(not(feature = "std"))]
        {
            let provider = safe_managed_alloc!(65536, CrateId::Component)?;
            let mut result = BoundedVec::new(provider).map_err(|_| {
                Error::runtime_execution_error("Failed to create result vector")
            })?;
            result.push(ComponentValue::U32(id.0)).map_err(|_| {
                Error::runtime_execution_error("Failed to add result value")
            })?;
            Ok(result)
        }
    }

    fn clone_handler(&self) -> Box<dyn BuiltinHandler> {
        Box::new(Self { resource_manager: self.resource_manager.clone() })
    }
}

/// Create all resource-related built-in handlers
pub fn create_resource_handlers(
    resource_manager: Arc<Mutex<ResourceManager>>,
) -> Vec<Box<dyn BuiltinHandler>> {
    vec![
        Box::new(ResourceCreateHandler::new(resource_manager.clone())),
        Box::new(ResourceDropHandler::new(resource_manager.clone())),
        Box::new(ResourceRepHandler::new(resource_manager.clone())),
        Box::new(ResourceGetHandler::new(resource_manager.clone())),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resources::ResourceManager;

    #[test]
    fn test_resource_create() {
        let resource_manager = Arc::new(Mutex::new(ResourceManager::new();
        let handler = ResourceCreateHandler::new(resource_manager.clone();

        // Test with valid args
        let args = vec![ComponentValue::U32(42)];
        let result = handler.execute(&args).unwrap();

        assert_eq!(result.len(), 1;
        match &result[0] {
            ComponentValue::U32(id) => {
                // Verify the resource was created
                let manager = resource_manager.lock().unwrap();
                assert!(manager.has_resource(ResourceId(*id));
            }
            _ => panic!("Expected U32 result"),
        }

        // Test with invalid args
        let invalid_args = vec![ComponentValue::String("not a number".into())];
        let error = handler.execute(&invalid_args;
        assert!(error.is_err();
    }

    #[test]
    fn test_resource_drop() {
        let resource_manager = Arc::new(Mutex::new(ResourceManager::new();

        // Create a resource
        let id = {
            let mut manager = resource_manager.lock().unwrap();
            manager.add_host_resource(42)
        };

        let handler = ResourceDropHandler::new(resource_manager.clone();

        // Test with valid args
        let args = vec![ComponentValue::U32(id.0)];
        let result = handler.execute(&args).unwrap();

        assert_eq!(result.len(), 0;

        // Verify the resource was dropped
        let manager = resource_manager.lock().unwrap();
        assert!(!manager.has_resource(id);
    }

    #[test]
    fn test_resource_rep() {
        let resource_manager = Arc::new(Mutex::new(ResourceManager::new();

        // Create a resource
        let id = {
            let mut manager = resource_manager.lock().unwrap();
            manager.add_host_resource(42u32)
        };

        let handler = ResourceRepHandler::new(resource_manager;

        // Test with valid args
        let args = vec![ComponentValue::U32(id.0)];
        let result = handler.execute(&args).unwrap();

        assert_eq!(result.len(), 1;
        match &result[0] {
            ComponentValue::U32(rep) => {
                assert_eq!(*rep, 42;
            }
            _ => panic!("Expected U32 result"),
        }
    }

    #[test]
    fn test_resource_get() {
        let resource_manager = Arc::new(Mutex::new(ResourceManager::new();
        let handler = ResourceGetHandler::new(resource_manager.clone();

        // Test with new representation
        let args = vec![ComponentValue::U32(42)];
        let result = handler.execute(&args).unwrap();

        assert_eq!(result.len(), 1;
        let first_id = match &result[0] {
            ComponentValue::U32(id) => *id,
            _ => panic!("Expected U32 result"),
        };

        // Test with same representation (should return same ID)
        let result2 = handler.execute(&args).unwrap();
        let second_id = match &result2[0] {
            ComponentValue::U32(id) => *id,
            _ => panic!("Expected U32 result"),
        };

        assert_eq!(first_id, second_id;
    }
}
