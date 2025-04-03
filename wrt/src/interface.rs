//! WebAssembly Component Model interface types
//!
//! This module contains implementations for the WebAssembly Component Model
//! interface types and canonical ABI, including value lifting/lowering between
//! core and component types.

use crate::{
    behavior::{FrameBehavior, StackBehavior},
    error::{Error, Result},
    global::Global,
    memory::{DefaultMemory, MemoryBehavior},
    module::{ExportKind, ExportValue, Function, Import, Module},
    module_instance::ModuleInstance,
    resource::{ResourceId, ResourceTable},
    types::{ComponentType, InstanceType, ValueType},
    values::Value,
};

// Import std when available
#[cfg(feature = "std")]
use std::{boxed::Box, format, string::String, vec::Vec};

// Import alloc for no_std
#[cfg(not(feature = "std"))]
use alloc::{boxed::Box, format, string::String, vec::Vec};

/// Interface value representing a Component Model value
#[derive(Debug, Clone)]
pub enum InterfaceValue {
    /// Boolean value
    Bool(bool),
    /// Signed 8-bit integer
    S8(i8),
    /// Unsigned 8-bit integer
    U8(u8),
    /// Signed 16-bit integer
    S16(i16),
    /// Unsigned 16-bit integer
    U16(u16),
    /// Signed 32-bit integer
    S32(i32),
    /// Unsigned 32-bit integer
    U32(u32),
    /// Signed 64-bit integer
    S64(i64),
    /// Unsigned 64-bit integer
    U64(u64),
    /// 32-bit floating point
    Float32(f32),
    /// 64-bit floating point
    Float64(f64),
    /// Character
    Char(char),
    /// String
    String(String),
    /// List of values
    List(Vec<InterfaceValue>),
    /// Record with named fields
    Record(Vec<(String, InterfaceValue)>),
    /// Tuple of values
    Tuple(Vec<InterfaceValue>),
    /// Variant with a discriminant and optional payload
    Variant {
        /// Case name
        case: String,
        /// Case index
        discriminant: u32,
        /// Optional payload
        payload: Option<Box<InterfaceValue>>,
    },
    /// Enum with a discriminant
    Enum {
        /// Case name
        case: String,
        /// Case index
        discriminant: u32,
    },
    /// Flags with named bits
    Flags(Vec<String>),
    /// Option with optional value
    Option(Option<Box<InterfaceValue>>),
    /// Result with ok or error value
    Result {
        /// Is ok
        is_ok: bool,
        /// Ok value if `is_ok` is true, otherwise error value
        value: Option<Box<InterfaceValue>>,
    },
    /// Resource reference
    Resource(ResourceId),
    /// Borrowed resource reference
    Borrowed(ResourceId),
}

/// Canonical ABI helper functions for Component Model
pub struct CanonicalABI;

impl CanonicalABI {
    /// Lift a core WebAssembly value to an interface value
    pub fn lift(
        value: Value,
        ty: &ComponentType,
        memory: Option<&dyn MemoryBehavior>,
        resources: Option<&ResourceTable>,
    ) -> Result<InterfaceValue> {
        let value_clone = value.clone(); // Clone value so we can reference it later
        match (value, ty) {
            // Simple primitive types
            (Value::I32(i), ComponentType::Primitive(ValueType::I32)) => Ok(InterfaceValue::S32(i)),
            (Value::I64(i), ComponentType::Primitive(ValueType::I64)) => Ok(InterfaceValue::S64(i)),
            (Value::F32(f), ComponentType::Primitive(ValueType::F32)) => {
                Ok(InterfaceValue::Float32(f))
            }
            (Value::F64(f), ComponentType::Primitive(ValueType::F64)) => {
                Ok(InterfaceValue::Float64(f))
            }

            // Explicit boolean value
            (Value::I32(i), ComponentType::Option(box_ty))
                if matches!(*box_ty.as_ref(), ComponentType::Primitive(ValueType::I32)) =>
            {
                Ok(InterfaceValue::Bool(i != 0))
            }

            // String (represented as pointer/length in core Wasm)
            (Value::I32(ptr), ComponentType::List(box_ty))
                if matches!(box_ty.as_ref(), ComponentType::Primitive(ValueType::I32))
                    && memory.is_some() =>
            {
                let mem = memory.unwrap();
                Self::lift_string(ptr, mem)
            }

            // Resource (represented as handle in core Wasm)
            (Value::I32(handle), ComponentType::Resource(_)) if resources.is_some() => {
                let resources = resources.unwrap();
                let id = ResourceId(handle as u64);
                if resources.get(id).is_ok() {
                    Ok(InterfaceValue::Resource(id))
                } else {
                    Err(Error::Execution(format!(
                        "Invalid resource handle: {handle}"
                    )))
                }
            }

            // Borrowed resource
            (Value::I32(handle), ComponentType::Borrowed(box_ty))
                if matches!(box_ty.as_ref(), ComponentType::Resource(_)) && resources.is_some() =>
            {
                let resources = resources.unwrap();
                let id = ResourceId(handle as u64);
                if resources.get(id).is_ok() {
                    Ok(InterfaceValue::Borrowed(id))
                } else {
                    Err(Error::Execution(format!(
                        "Invalid resource handle: {handle}"
                    )))
                }
            }

            // Not supported
            _ => Err(Error::Execution(format!(
                "Cannot lift value {value_clone:?} to interface type {ty:?}"
            ))),
        }
    }

    /// Lower an interface value to a core WebAssembly value
    pub fn lower(
        value: InterfaceValue,
        memory: Option<&mut dyn MemoryBehavior>,
        resources: Option<&mut ResourceTable>,
    ) -> Result<Value> {
        match value {
            // Simple primitive types
            InterfaceValue::Bool(b) => Ok(Value::I32(if b { 1 } else { 0 })),
            InterfaceValue::S8(i) => Ok(Value::I32(i32::from(i))),
            InterfaceValue::U8(i) => Ok(Value::I32(i32::from(i))),
            InterfaceValue::S16(i) => Ok(Value::I32(i32::from(i))),
            InterfaceValue::U16(i) => Ok(Value::I32(i32::from(i))),
            InterfaceValue::S32(i) => Ok(Value::I32(i)),
            InterfaceValue::U32(i) => Ok(Value::I32(i as i32)),
            InterfaceValue::S64(i) => Ok(Value::I64(i)),
            InterfaceValue::U64(i) => Ok(Value::I64(i as i64)),
            InterfaceValue::Float32(f) => Ok(Value::F32(f)),
            InterfaceValue::Float64(f) => Ok(Value::F64(f)),
            InterfaceValue::Char(c) => Ok(Value::I32(c as i32)),

            // String (will be stored in memory and return pointer/length)
            InterfaceValue::String(s) if memory.is_some() => {
                let mem = memory.unwrap();
                Self::lower_string(s, mem)
            }

            // Resource
            InterfaceValue::Resource(id) => Ok(Value::I32(id.0 as i32)),
            InterfaceValue::Borrowed(id) => Ok(Value::I32(id.0 as i32)),

            // Complex types - these would typically be lowered to
            // multiple values or pointers to memory structures
            _ => Err(Error::Execution(format!(
                "Cannot lower interface value {value:?} to core type"
            ))),
        }
    }

    /// Lift a string from memory
    fn lift_string(ptr: i32, memory: &dyn MemoryBehavior) -> Result<InterfaceValue> {
        if ptr < 0 {
            return Err(Error::Execution(format!("Invalid string pointer: {ptr}")));
        }

        // In the canonical ABI, strings are represented as a pointer to a length-prefixed UTF-8 sequence
        let addr = ptr as u32;
        if addr + 4 > memory.size() * 65536 {
            return Err(Error::Execution(format!(
                "String pointer out of bounds: {ptr}"
            )));
        }

        // Read the length
        let length_bytes = memory.read_bytes(addr, 4)?;
        let length = u32::from_le_bytes([
            length_bytes[0],
            length_bytes[1],
            length_bytes[2],
            length_bytes[3],
        ]) as usize;

        // Read the string data
        let string_data = memory.read_bytes(addr + 4, length)?;

        // Convert to UTF-8 string
        let string = String::from_utf8(string_data.to_vec())
            .map_err(|e| Error::Execution(format!("Invalid UTF-8 string in memory: {e}")))?;

        Ok(InterfaceValue::String(string))
    }

    /// Lower a string to memory
    fn lower_string(s: String, memory: &mut dyn MemoryBehavior) -> Result<Value> {
        // Get the string as UTF-8 bytes
        let bytes = s.as_bytes();
        let length = bytes.len();

        // Ensure we have enough memory for the string
        // We need 4 bytes for the length + the string data
        let total_size = 4 + length;

        // First check if we need to grow memory
        let current_size = memory.size_bytes();
        if current_size < total_size {
            let pages_needed = (total_size / 65536) + 1;
            memory.grow(pages_needed as u32)?;
        }

        // Use the end of the current data as our allocation point
        let addr = (memory.size_bytes() - total_size) as u32;

        // Write the length
        let length_bytes = (length as u32).to_le_bytes();
        memory.write_bytes(addr, &length_bytes)?;

        // Write the string data
        memory.write_bytes(addr + 4, bytes)?;

        // Return the pointer
        Ok(Value::I32(addr as i32))
    }

    /// Lower a component value to a WebAssembly value
    pub fn lower_value(
        &self,
        value: InterfaceValue,
        ty: &ComponentType,
        memory: Option<&mut dyn MemoryBehavior>,
        _resources: Option<&mut ResourceTable>,
    ) -> Result<Value> {
        match value {
            // Simple primitive types
            InterfaceValue::Bool(b) => Ok(Value::I32(if b { 1 } else { 0 })),
            InterfaceValue::S8(i) => Ok(Value::I32(i32::from(i))),
            InterfaceValue::U8(i) => Ok(Value::I32(i32::from(i))),
            InterfaceValue::S16(i) => Ok(Value::I32(i32::from(i))),
            InterfaceValue::U16(i) => Ok(Value::I32(i32::from(i))),
            InterfaceValue::S32(i) => Ok(Value::I32(i)),
            InterfaceValue::U32(i) => Ok(Value::I32(i as i32)),
            InterfaceValue::S64(i) => Ok(Value::I64(i)),
            InterfaceValue::U64(i) => Ok(Value::I64(i as i64)),
            InterfaceValue::Float32(f) => Ok(Value::F32(f)),
            InterfaceValue::Float64(f) => Ok(Value::F64(f)),
            InterfaceValue::Char(c) => Ok(Value::I32(c as i32)),

            // String (will be stored in memory and return pointer/length)
            InterfaceValue::String(s) if memory.is_some() => Self::lower_string(s, memory.unwrap()),

            // Resource
            InterfaceValue::Resource(id) => Ok(Value::I32(id.0 as i32)),
            InterfaceValue::Borrowed(id) => Ok(Value::I32(id.0 as i32)),

            _ => Err(Error::Execution(format!(
                "Cannot lower interface value {value:?} to core type with given component type {ty:?}"
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::DefaultMemory;
    use crate::resource::{
        ResourceRepresentation, ResourceTable, ResourceType, SimpleResourceData,
    };
    use crate::types::MemoryType;

    use std::sync::Arc;

    #[test]
    fn test_lift_primitive_values() -> Result<()> {
        // Test lifting i32
        let i32_val = Value::I32(42);
        let i32_type = ComponentType::Primitive(ValueType::I32);
        let result = CanonicalABI::lift(i32_val, &i32_type, None, None)?;
        assert!(matches!(result, InterfaceValue::S32(42)));

        // Test lifting i64
        let i64_val = Value::I64(0x1234_5678_9ABC_DEF0);
        let i64_type = ComponentType::Primitive(ValueType::I64);
        let result = CanonicalABI::lift(i64_val, &i64_type, None, None)?;
        assert!(matches!(result, InterfaceValue::S64(0x1234_5678_9ABC_DEF0)));

        // Test lifting f32
        let f32_val = Value::F32(3.14);
        let f32_type = ComponentType::Primitive(ValueType::F32);
        let result = CanonicalABI::lift(f32_val, &f32_type, None, None)?;
        let InterfaceValue::Float32(f) = result else {
            return Err(Error::Execution("Expected Float32".into()));
        };
        assert_eq!(f, 3.14);

        // Test lifting f64
        let f64_val = Value::F64(2.71828);
        let f64_type = ComponentType::Primitive(ValueType::F64);
        let result = CanonicalABI::lift(f64_val, &f64_type, None, None)?;
        let InterfaceValue::Float64(f) = result else {
            return Err(Error::Execution("Expected Float64".into()));
        };
        assert_eq!(f, 2.71828);

        Ok(())
    }

    #[test]
    fn test_lower_primitive_values() -> Result<()> {
        // Test lowering bool
        let bool_val = InterfaceValue::Bool(true);
        let result = CanonicalABI::lower(bool_val, None, None)?;
        assert!(matches!(result, Value::I32(1)));

        // Test lowering char
        let char_val = InterfaceValue::Char('A');
        let result = CanonicalABI::lower(char_val, None, None)?;
        assert!(matches!(result, Value::I32(65)));

        // Test lowering s64
        let s64_val = InterfaceValue::S64(-12345);
        let result = CanonicalABI::lower(s64_val, None, None)?;
        assert!(matches!(result, Value::I64(-12345)));

        // Test lowering float32
        let f32_val = InterfaceValue::Float32(3.14);
        let result = CanonicalABI::lower(f32_val, None, None)?;
        let Value::F32(f) = result else {
            return Err(Error::Execution("Expected F32".into()));
        };
        assert_eq!(f, 3.14);

        Ok(())
    }

    #[test]
    fn test_string_operations() -> Result<()> {
        // Create a memory instance
        let mem_type = MemoryType {
            min: 1,
            max: Some(2),
        };
        let mut memory = DefaultMemory::new(mem_type);

        // Test string lowering
        let string_val = InterfaceValue::String("Hello, WebAssembly!".to_string());
        let result = CanonicalABI::lower(string_val, Some(&mut memory), None)?;

        // The result should be an i32 pointer
        let Value::I32(ptr) = result else {
            return Err(Error::Execution("Expected I32 pointer".into()));
        };

        // Now lift the string back from memory
        let list_type = ComponentType::List(Box::new(ComponentType::Primitive(ValueType::I32)));
        let lifted = CanonicalABI::lift(Value::I32(ptr), &list_type, Some(&memory), None)?;

        // Should get back the same string
        let InterfaceValue::String(s) = lifted else {
            return Err(Error::Execution("Expected String".into()));
        };
        assert_eq!(s, "Hello, WebAssembly!");

        Ok(())
    }

    #[test]
    fn test_resource_operations() -> Result<()> {
        // Create a resource table
        let mut resource_table = ResourceTable::new();

        // Create a resource type
        let resource_type = ResourceType {
            name: String::from("test:resource"),
            representation: ResourceRepresentation::Handle32,
            nullable: false,
            borrowable: true,
        };

        // Allocate a resource
        let data = Arc::new(SimpleResourceData { value: 42 });
        let id = resource_table.allocate(resource_type.clone(), data);

        // Lower the resource
        let resource_val = InterfaceValue::Resource(id);
        let result = CanonicalABI::lower(resource_val, None, Some(&mut resource_table))?;

        // The result should be an i32 handle
        let Value::I32(handle) = result else {
            return Err(Error::Execution("Expected I32 handle".into()));
        };

        // Now lift the resource back from the handle
        let resource_component_type = ComponentType::Resource(resource_type);
        let lifted = CanonicalABI::lift(
            Value::I32(handle),
            &resource_component_type,
            None,
            Some(&resource_table),
        )?;

        // Should get back the same resource ID
        let InterfaceValue::Resource(res_id) = lifted else {
            return Err(Error::Execution("Expected Resource".into()));
        };
        assert_eq!(res_id.0, id.0);

        Ok(())
    }
}

/// Instantiates a WebAssembly component based on the provided module.
///
/// This function takes a module and an optional resource table, and attempts
/// to create an instance according to the WebAssembly Component Model interface.
/// It currently returns a placeholder instance type.
///
/// # Arguments
///
/// * `module`: A reference to the parsed `Module` representing the component.
/// * `_resources`: An optional mutable reference to a `ResourceTable` (currently unused).
///
/// # Returns
///
/// A `Result` containing the `InstanceType` on success, or an `Error` on failure.
pub fn instantiate(
    module: &Module,
    _resources: Option<&mut ResourceTable>,
) -> Result<InstanceType> {
    // Create a simple instance type with no exports
    Ok(InstanceType {
        exports: Vec::new(),
    })
}
