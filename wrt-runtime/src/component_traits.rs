use crate::func::FuncType;
use crate::prelude::*;

/// Represents a runtime component instance
pub trait ComponentInstance {
    /// Execute a function by name with the given arguments
    fn execute_function(&self, name: &str, args: &[Value]) -> Result<SafeStack<Value>>;

    /// Read from exported memory
    fn read_memory(&self, name: &str, offset: u32, size: u32) -> Result<SafeSlice<'_>>;

    /// Write to exported memory
    fn write_memory(&mut self, name: &str, offset: u32, bytes: &[u8]) -> Result<()>;

    /// Get the type of an export
    fn get_export_type(&self, name: &str) -> Result<ExternType>;

    /// Execute a function by name with the given arguments (legacy Vec API)
    #[deprecated(since = "0.2.0", note = "Use execute_function with SafeStack instead")]
    fn execute_function_vec(&self, name: &str, args: &[Value]) -> Result<Vec<Value>> {
        // Convert from the new SafeStack API to the legacy Vec API
        let safe_stack = self.execute_function(name, args)?;
        safe_stack.to_vec()
    }

    /// Read from exported memory (legacy Vec API)
    #[deprecated(since = "0.2.0", note = "Use read_memory with SafeSlice instead")]
    fn read_memory_vec(&self, name: &str, offset: u32, size: u32) -> Result<Vec<u8>> {
        // Convert from the new SafeSlice API to the legacy Vec API
        let safe_slice = self.read_memory(name, offset, size)?;
        Ok(safe_slice.data()?.to_vec())
    }
}

/// Represents a host function implementation
pub trait HostFunction {
    /// Call the host function with the given arguments
    fn call(&self, args: &[Value]) -> Result<SafeStack<Value>>;

    /// Get the function's type
    fn get_type(&self) -> FuncType;

    /// Call the host function with the given arguments (legacy Vec API)
    #[deprecated(since = "0.2.0", note = "Use call with SafeStack instead")]
    fn call_vec(&self, args: &[Value]) -> Result<Vec<Value>> {
        // Convert from the new SafeStack API to the legacy Vec API
        let safe_stack = self.call(args)?;
        safe_stack.to_vec()
    }
}

/// Represents a host function factory
pub trait HostFunctionFactory {
    /// Create a host function implementation
    fn create_function(&self, name: &str, ty: &FuncType) -> Result<Box<dyn HostFunction>>;
}

/// Represents a component runtime environment
pub trait ComponentRuntime {
    /// Create a new runtime instance
    fn new() -> Self
    where
        Self: Sized;

    /// Register a host function factory
    fn register_host_factory(&mut self, factory: Box<dyn HostFunctionFactory>);

    /// Instantiate a component
    fn instantiate(&self, component_type: &ComponentType) -> Result<Box<dyn ComponentInstance>>;

    /// Register a specific host function
    fn register_host_function<F>(&mut self, name: &str, ty: FuncType, function: F) -> Result<()>
    where
        F: Fn(&[Value]) -> Result<Vec<Value>> + 'static + Send + Sync;

    /// Set the verification level for memory operations
    fn set_verification_level(&mut self, level: VerificationLevel) -> Result<()>;

    /// Get the current verification level
    fn verification_level(&self) -> VerificationLevel;
}
