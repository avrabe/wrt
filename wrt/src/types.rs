use std::fmt;

use crate::{String, Vec};

/// Represents a WebAssembly value type
#[derive(Debug, Clone, PartialEq, Copy, Eq, Hash)]
pub enum ValueType {
    /// 32-bit integer
    I32,
    /// 64-bit integer
    I64,
    /// 32-bit float
    F32,
    /// 64-bit float
    F64,
    /// Function reference
    FuncRef,
    /// External reference
    ExternRef,
    /// 128-bit vector for SIMD operations
    V128,
    /// Any reference type (reference types proposal)
    AnyRef,
}

/// Represents a WebAssembly function type
#[derive(Debug, Clone, PartialEq)]
pub struct FuncType {
    /// Parameter types
    pub params: Vec<ValueType>,
    /// Result types
    pub results: Vec<ValueType>,
}

/// Represents a WebAssembly table type
#[derive(Debug, Clone, PartialEq)]
pub struct TableType {
    /// Element type
    pub element_type: ValueType,
    /// Minimum size
    pub min: u32,
    /// Maximum size (optional)
    pub max: Option<u32>,
}

/// Represents a WebAssembly memory type
#[derive(Debug, Clone, PartialEq)]
pub struct MemoryType {
    /// Minimum size in pages
    pub min: u32,
    /// Maximum size in pages (optional)
    pub max: Option<u32>,
}

/// Represents a WebAssembly global type
#[derive(Debug, Clone, PartialEq)]
pub struct GlobalType {
    /// Content type
    pub content_type: ValueType,
    /// Whether the global is mutable
    pub mutable: bool,
}

/// Represents a WebAssembly external type
#[derive(Debug, Clone, PartialEq)]
pub enum ExternType {
    /// Function type
    Function(FuncType),
    /// Table type
    Table(TableType),
    /// Memory type
    Memory(MemoryType),
    /// Global type
    Global(GlobalType),
}

/// Represents a component model type
#[derive(Debug, Clone, PartialEq)]
pub enum ComponentType {
    /// Primitive type
    Primitive(ValueType),
    /// Record type
    Record(Vec<(String, ValueType)>),
    /// Tuple type
    Tuple(Vec<ValueType>),
    /// List type
    List(ValueType),
    /// Flags type
    Flags(Vec<String>),
    /// Variant type
    Variant(Vec<(String, Option<ValueType>)>),
    /// Enum type
    Enum(Vec<String>),
    /// Union type
    Union(Vec<ValueType>),
    /// Option type
    Option(ValueType),
    /// Result type
    Result {
        /// Ok type
        ok: Option<ValueType>,
        /// Error type
        err: Option<ValueType>,
    },
    /// Future type
    Future(ValueType),
    /// Stream type
    Stream {
        /// Element type
        element: ValueType,
        /// End type
        end: Option<ValueType>,
    },
    /// Unknown type
    Unknown,
}

impl fmt::Display for ValueType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ValueType::I32 => write!(f, "i32"),
            ValueType::I64 => write!(f, "i64"),
            ValueType::F32 => write!(f, "f32"),
            ValueType::F64 => write!(f, "f64"),
            ValueType::FuncRef => write!(f, "funcref"),
            ValueType::ExternRef => write!(f, "externref"),
            ValueType::V128 => write!(f, "v128"),
            ValueType::AnyRef => write!(f, "anyref"),
        }
    }
}

impl ValueType {
    /// Returns the size of the value type in bytes
    pub fn size(&self) -> usize {
        match self {
            ValueType::I32 | ValueType::F32 => 4,
            ValueType::I64 | ValueType::F64 => 8,
            ValueType::FuncRef | ValueType::ExternRef | ValueType::AnyRef => 8,
            ValueType::V128 => 16,
        }
    }

    /// Returns whether the value type is a reference type
    pub fn is_ref(&self) -> bool {
        matches!(
            self,
            ValueType::FuncRef | ValueType::ExternRef | ValueType::AnyRef
        )
    }

    /// Returns whether the value type is a numeric type
    pub fn is_numeric(&self) -> bool {
        matches!(
            self,
            ValueType::I32 | ValueType::I64 | ValueType::F32 | ValueType::F64 | ValueType::V128
        )
    }

    /// Returns whether the value type is a vector type
    pub fn is_vector(&self) -> bool {
        matches!(self, ValueType::V128)
    }
}

impl ComponentType {
    /// Returns whether the component type is a primitive type
    pub fn is_primitive(&self) -> bool {
        matches!(self, ComponentType::Primitive(_))
    }

    /// Returns whether the component type is a reference type
    pub fn is_ref(&self) -> bool {
        matches!(
            self,
            ComponentType::Primitive(ValueType::FuncRef | ValueType::ExternRef)
        )
    }

    /// Returns whether the component type is a numeric type
    pub fn is_numeric(&self) -> bool {
        matches!(
            self,
            ComponentType::Primitive(
                ValueType::I32 | ValueType::I64 | ValueType::F32 | ValueType::F64
            )
        )
    }

    /// Returns whether the component type is a record type
    pub fn is_record(&self) -> bool {
        matches!(self, ComponentType::Record(_))
    }

    /// Returns whether the component type is a tuple type
    pub fn is_tuple(&self) -> bool {
        matches!(self, ComponentType::Tuple(_))
    }

    /// Returns whether the component type is a list type
    pub fn is_list(&self) -> bool {
        matches!(self, ComponentType::List(_))
    }

    /// Returns whether the component type is a flags type
    pub fn is_flags(&self) -> bool {
        matches!(self, ComponentType::Flags(_))
    }

    /// Returns whether the component type is a variant type
    pub fn is_variant(&self) -> bool {
        matches!(self, ComponentType::Variant(_))
    }

    /// Returns whether the component type is an enum type
    pub fn is_enum(&self) -> bool {
        matches!(self, ComponentType::Enum(_))
    }

    /// Returns whether the component type is a union type
    pub fn is_union(&self) -> bool {
        matches!(self, ComponentType::Union(_))
    }

    /// Returns whether the component type is an option type
    pub fn is_option(&self) -> bool {
        matches!(self, ComponentType::Option(_))
    }

    /// Returns whether the component type is a result type
    pub fn is_result(&self) -> bool {
        matches!(self, ComponentType::Result { .. })
    }

    /// Returns whether the component type is a future type
    pub fn is_future(&self) -> bool {
        matches!(self, ComponentType::Future(_))
    }

    /// Returns whether the component type is a stream type
    pub fn is_stream(&self) -> bool {
        matches!(self, ComponentType::Stream { .. })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::values::Value;
    #[cfg(not(feature = "std"))]
    use alloc::vec;

    #[test]
    fn test_value_type_equality() {
        assert_eq!(ValueType::I32, ValueType::I32);
        assert_eq!(ValueType::I64, ValueType::I64);
        assert_eq!(ValueType::F32, ValueType::F32);
        assert_eq!(ValueType::F64, ValueType::F64);
        assert_eq!(ValueType::FuncRef, ValueType::FuncRef);
        assert_eq!(ValueType::ExternRef, ValueType::ExternRef);

        assert_ne!(ValueType::I32, ValueType::I64);
        assert_ne!(ValueType::F32, ValueType::F64);
        assert_ne!(ValueType::FuncRef, ValueType::ExternRef);
    }

    #[test]
    fn test_func_type() {
        let func_type = FuncType {
            params: vec![ValueType::I32, ValueType::I64],
            results: vec![ValueType::F32],
        };

        assert_eq!(func_type.params.len(), 2);
        assert_eq!(func_type.results.len(), 1);
        assert_eq!(func_type.params[0], ValueType::I32);
        assert_eq!(func_type.params[1], ValueType::I64);
        assert_eq!(func_type.results[0], ValueType::F32);

        // Test cloning
        let cloned = func_type.clone();
        assert_eq!(func_type, cloned);
    }

    #[test]
    fn test_table_type() {
        let table_type = TableType {
            element_type: ValueType::FuncRef,
            min: 1,
            max: Some(10),
        };

        assert_eq!(table_type.element_type, ValueType::FuncRef);
        assert_eq!(table_type.min, 1);
        assert_eq!(table_type.max, Some(10));

        // Test with no maximum
        let unlimited_table = TableType {
            element_type: ValueType::FuncRef,
            min: 0,
            max: None,
        };
        assert_eq!(unlimited_table.max, None);

        // Test cloning
        let cloned = table_type.clone();
        assert_eq!(table_type, cloned);
    }

    #[test]
    fn test_memory_type() {
        let memory_type = MemoryType {
            min: 1,
            max: Some(10),
        };

        assert_eq!(memory_type.min, 1);
        assert_eq!(memory_type.max, Some(10));

        // Test with no maximum
        let unlimited_memory = MemoryType { min: 0, max: None };
        assert_eq!(unlimited_memory.max, None);

        // Test cloning
        let cloned = memory_type.clone();
        assert_eq!(memory_type, cloned);
    }

    #[test]
    fn test_global_type() {
        let global_type = GlobalType {
            content_type: ValueType::I32,
            mutable: true,
        };

        assert_eq!(global_type.content_type, ValueType::I32);
        assert!(global_type.mutable);

        // Test immutable global
        let immutable_global = GlobalType {
            content_type: ValueType::F64,
            mutable: false,
        };
        assert!(!immutable_global.mutable);

        // Test cloning
        let cloned = global_type.clone();
        assert_eq!(global_type, cloned);
    }

    #[test]
    fn test_extern_type() {
        // Test function external type
        let func_type = FuncType {
            params: vec![ValueType::I32],
            results: vec![ValueType::I32],
        };
        let func_extern = ExternType::Function(func_type.clone());
        match &func_extern {
            ExternType::Function(ft) => assert_eq!(ft, &func_type),
            _ => panic!("Expected Function variant"),
        }

        // Test table external type
        let table_type = TableType {
            element_type: ValueType::FuncRef,
            min: 1,
            max: Some(10),
        };
        let table_extern = ExternType::Table(table_type.clone());
        match &table_extern {
            ExternType::Table(tt) => assert_eq!(tt, &table_type),
            _ => panic!("Expected Table variant"),
        }

        // Test memory external type
        let memory_type = MemoryType {
            min: 1,
            max: Some(10),
        };
        let memory_extern = ExternType::Memory(memory_type.clone());
        match &memory_extern {
            ExternType::Memory(mt) => assert_eq!(mt, &memory_type),
            _ => panic!("Expected Memory variant"),
        }

        // Test global external type
        let global_type = GlobalType {
            content_type: ValueType::I32,
            mutable: true,
        };
        let global_extern = ExternType::Global(global_type.clone());
        match &global_extern {
            ExternType::Global(gt) => assert_eq!(gt, &global_type),
            _ => panic!("Expected Global variant"),
        }

        // Test cloning
        let cloned = func_extern.clone();
        assert_eq!(func_extern, cloned);
    }

    #[test]
    fn test_value_type_operations() {
        // Test value type creation and comparison
        assert_eq!(ValueType::I32, ValueType::I32);
        assert_ne!(ValueType::I32, ValueType::I64);
        assert_ne!(ValueType::F32, ValueType::F64);

        // Test default values for types
        assert!(matches!(
            Value::default_for_type(&ValueType::I32),
            Value::I32(0)
        ));
        assert!(matches!(
            Value::default_for_type(&ValueType::I64),
            Value::I64(0)
        ));
        assert!(matches!(
            Value::default_for_type(&ValueType::F32),
            Value::F32(0.0)
        ));
        assert!(matches!(
            Value::default_for_type(&ValueType::F64),
            Value::F64(0.0)
        ));
    }
}
