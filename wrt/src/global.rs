use crate::error::{Error, Result};
use crate::types::*;
use crate::values::Value;
use crate::{format, Vec};

/// Represents a WebAssembly global instance
#[derive(Debug, Clone)]
pub struct Global {
    /// Global type
    global_type: GlobalType,
    /// Global value
    value: Value,
}

impl Global {
    /// Creates a new global instance
    pub fn new(global_type: GlobalType, value: Value) -> Result<Self> {
        // Check that the value matches the global type
        if !value.matches_type(&global_type.content_type) {
            return Err(Error::Execution(format!(
                "Value type {:?} does not match global type {:?}",
                value.type_(),
                global_type.content_type
            )));
        }

        Ok(Self { global_type, value })
    }

    /// Returns the global type
    pub fn type_(&self) -> &GlobalType {
        &self.global_type
    }

    /// Gets the global value
    pub fn get(&self) -> Value {
        self.value.clone()
    }

    /// Sets the global value
    pub fn set(&mut self, value: Value) -> Result<()> {
        // Check mutability
        if !self.global_type.mutable {
            return Err(Error::Execution("Cannot set immutable global".into()));
        }

        // Check value type
        if !value.matches_type(&self.global_type.content_type) {
            return Err(Error::Execution(format!(
                "Value type {:?} does not match global type {:?}",
                value.type_(),
                self.global_type.content_type
            )));
        }

        self.value = value;
        Ok(())
    }
}

/// Represents a collection of global instances
#[derive(Debug)]
pub struct Globals {
    /// Global instances
    globals: Vec<Global>,
}

impl Default for Globals {
    fn default() -> Self {
        Self::new()
    }
}

impl Globals {
    /// Creates a new empty globals collection
    pub fn new() -> Self {
        Self {
            globals: Vec::new(),
        }
    }

    /// Adds a new global instance
    pub fn add(&mut self, global: Global) -> u32 {
        let idx = self.globals.len() as u32;
        self.globals.push(global);
        idx
    }

    /// Gets a global instance by index
    pub fn get(&self, idx: u32) -> Result<&Global> {
        self.globals
            .get(idx as usize)
            .ok_or_else(|| Error::Execution(format!("Global index {} out of bounds", idx)))
    }

    /// Gets a mutable reference to a global instance by index
    pub fn get_mut(&mut self, idx: u32) -> Result<&mut Global> {
        self.globals
            .get_mut(idx as usize)
            .ok_or_else(|| Error::Execution(format!("Global index {} out of bounds", idx)))
    }

    /// Returns the number of global instances
    pub fn len(&self) -> u32 {
        self.globals.len() as u32
    }

    /// Returns whether the globals collection is empty
    pub fn is_empty(&self) -> bool {
        self.globals.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Helper function to create a test global type
    fn create_test_global_type(value_type: ValueType, mutable: bool) -> GlobalType {
        GlobalType {
            content_type: value_type,
            mutable,
        }
    }

    #[test]
    fn test_global_creation() {
        let global_type = GlobalType {
            content_type: ValueType::I32,
            mutable: true,
        };
        let value = Value::I32(42);
        let global = Global::new(global_type.clone(), value.clone()).unwrap();

        assert_eq!(global.get(), value);

        // Test invalid type combination
        let wrong_value = Value::I64(42);
        assert!(Global::new(global_type, wrong_value).is_err());
    }

    #[test]
    fn test_global_mutability() -> Result<()> {
        // Test mutable global
        let global_type = create_test_global_type(ValueType::I32, true);
        let mut global = Global::new(global_type, Value::I32(42))?;

        // Valid set operation
        assert!(global.set(Value::I32(100)).is_ok());
        assert_eq!(global.get(), Value::I32(100));

        // Invalid type for set
        assert!(global.set(Value::I64(100)).is_err());

        // Test immutable global
        let global_type = create_test_global_type(ValueType::I32, false);
        let mut global = Global::new(global_type, Value::I32(42))?;

        // Attempt to modify immutable global
        let result = global.set(Value::I32(100));
        assert!(result.is_err());
        if let Err(Error::Execution(msg)) = result {
            assert!(msg.contains("Cannot set immutable global"));
        }

        Ok(())
    }

    #[test]
    fn test_globals_collection() -> Result<()> {
        let mut globals = Globals::new();
        assert!(globals.is_empty());
        assert_eq!(globals.len(), 0);

        // Add globals
        let global1 = Global::new(
            create_test_global_type(ValueType::I32, true),
            Value::I32(42),
        )?;
        let global2 = Global::new(
            create_test_global_type(ValueType::I64, false),
            Value::I64(100),
        )?;

        let idx1 = globals.add(global1);
        let idx2 = globals.add(global2);

        assert_eq!(idx1, 0);
        assert_eq!(idx2, 1);
        assert_eq!(globals.len(), 2);
        assert!(!globals.is_empty());

        // Test get operations
        let global1 = globals.get(idx1)?;
        assert_eq!(global1.get(), Value::I32(42));
        assert!(global1.type_().mutable);

        let global2 = globals.get(idx2)?;
        assert_eq!(global2.get(), Value::I64(100));
        assert!(!global2.type_().mutable);

        Ok(())
    }

    #[test]
    fn test_globals_error_handling() {
        let globals = Globals::new();

        // Test out of bounds errors
        match globals.get(0) {
            Err(Error::Execution(msg)) => assert!(msg.contains("out of bounds")),
            _ => panic!("Expected out of bounds error"),
        }

        match globals.get(100) {
            Err(Error::Execution(msg)) => assert!(msg.contains("out of bounds")),
            _ => panic!("Expected out of bounds error"),
        }
    }

    #[test]
    fn test_global_value_types() -> Result<()> {
        // Test I32
        let global = Global::new(
            create_test_global_type(ValueType::I32, true),
            Value::I32(42),
        )?;
        assert_eq!(global.get(), Value::I32(42));

        // Test I64
        let global = Global::new(
            create_test_global_type(ValueType::I64, true),
            Value::I64(42),
        )?;
        assert_eq!(global.get(), Value::I64(42));

        // Test F32
        let global = Global::new(
            create_test_global_type(ValueType::F32, true),
            Value::F32(42.0),
        )?;
        assert_eq!(global.get(), Value::F32(42.0));

        // Test F64
        let global = Global::new(
            create_test_global_type(ValueType::F64, true),
            Value::F64(42.0),
        )?;
        assert_eq!(global.get(), Value::F64(42.0));

        Ok(())
    }
}
