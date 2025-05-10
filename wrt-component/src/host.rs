//! Host implementation for the WebAssembly Component Model.
//!
//! This module provides the Host type for managing host functions.

use crate::prelude::*;

use wrt_format::component::ExternType;
use wrt_host::callback::CallbackType;

/// Represents a function provided by the host
#[derive(Debug, Clone)]
pub struct HostFunction {
    /// Function type
    pub ty: ExternType,
    /// Function implementation
    pub implementation: HostFunctionImpl,
}

/// Host function implementation
#[derive(Debug, Clone)]
pub enum HostFunctionImpl {
    /// Callback function
    Callback(String),
    /// Trap (unimplemented function)
    Trap(String),
}

/// Host environment for component model
#[derive(Debug, Default)]
pub struct Host {
    /// Host functions
    functions: HashMap<String, HostFunction>,
}

impl Host {
    /// Creates a new empty host
    pub fn new() -> Self {
        Self { functions: HashMap::new() }
    }

    /// Adds a host function
    pub fn add_function(&mut self, name: String, function: HostFunction) {
        self.functions.insert(name, function);
    }

    /// Gets a host function by name
    pub fn get_function(&self, name: &str) -> Option<&HostFunction> {
        self.functions.get(name)
    }

    /// Calls a host function by name
    pub fn call_function(
        &self,
        name: &str,
        args: Vec<Value>,
        registry: &CallbackRegistry,
        target: &mut dyn core::any::Any,
    ) -> Result<Vec<Value>> {
        let function = self.functions.get(name).ok_or_else(|| {
            Error::new(
                ErrorCategory::Runtime,
                codes::EXECUTION_ERROR,
                kinds::ExecutionError(format!("Host function not found: {}", name)),
            )
        })?;

        match &function.implementation {
            HostFunctionImpl::Callback(callback_name) => {
                let callback_type = CallbackType::Logging; // This should be properly implemented

                // Note: Registry access needs to be correctly implemented with the proper type
                registry.call_host_function(target, "wrt_component", callback_name, args)
            }
            HostFunctionImpl::Trap(message) => Err(Error::new(
                ErrorCategory::Runtime,
                codes::EXECUTION_ERROR,
                kinds::ExecutionError(format!("Function {} trapped: {}", name, message)),
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wrt_format::component::ValType;

    #[test]
    fn test_host_function_management() {
        let mut host = Host::new();

        let func_type = ExternType::Function {
            params: vec![("a".to_string(), ValType::S32), ("b".to_string(), ValType::S32)],
            results: vec![ValType::S32],
        };

        let function = HostFunction {
            ty: func_type,
            implementation: HostFunctionImpl::Callback("add".to_string()),
        };

        host.add_function("add".to_string(), function);

        let retrieved = host.get_function("add");
        assert!(retrieved.is_some());

        let not_found = host.get_function("non_existent");
        assert!(not_found.is_none());
    }
}
