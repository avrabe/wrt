//! Types for the WebAssembly Component Model
//!
//! This module provides component model type definitions.

use crate::prelude::*;

#[cfg(all(feature = "std", feature = "safety-critical"))]
use wrt_foundation::allocator::{WrtVec, CrateId};

use wrt_foundation::{bounded::{BoundedVec, BoundedString}, prelude::*, traits::{Checksummable, ToBytes, FromBytes}};

use crate::{
    components::component::Component,
    instantiation::{ModuleInstance, ResolvedExport, ResolvedImport, ResourceTable},
};

#[cfg(feature = "component-model-async")]
use crate::async_::async_types::{StreamHandle, FutureHandle};

// Fallback types when async features are not enabled
#[cfg(not(feature = "component-model-async"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StreamHandle(pub u32);

#[cfg(not(feature = "component-model-async"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FutureHandle(pub u32);

/// Canonical ComponentInstance definition for ASIL-D type safety
/// 
/// This is the single source of truth for ComponentInstance across the codebase
/// to ensure type consistency and prevent safety violations.
///
/// SW-REQ-ID: REQ_TYPE_001 - Unified type definitions
/// SW-REQ-ID: REQ_SAFETY_002 - Type safety enforcement
#[derive(Debug, Clone)]
pub struct ComponentInstance {
    /// Unique instance ID
    pub id: u32,
    /// Reference to the component definition
    pub component: Component,
    /// Resolved imports for this instance
    #[cfg(all(feature = "std", feature = "safety-critical"))]
    pub imports: WrtVec<ResolvedImport, {CrateId::Component as u8}, 256>,
    #[cfg(all(feature = "std", not(feature = "safety-critical")))]
    pub imports: Vec<ResolvedImport>,
    #[cfg(not(any(feature = "std", )))]
    pub imports: BoundedVec<ResolvedImport, 256, crate::bounded_component_infra::ComponentProvider>,
    /// Resolved exports from this instance
    #[cfg(all(feature = "std", feature = "safety-critical"))]
    pub exports: WrtVec<ResolvedExport, {CrateId::Component as u8}, 256>,
    #[cfg(all(feature = "std", not(feature = "safety-critical")))]
    pub exports: Vec<ResolvedExport>,
    #[cfg(not(any(feature = "std", )))]
    pub exports: BoundedVec<ResolvedExport, 256, crate::bounded_component_infra::ComponentProvider>,
    /// Resource tables for this instance
    #[cfg(all(feature = "std", feature = "safety-critical"))]
    pub resource_tables: WrtVec<ResourceTable, {CrateId::Component as u8}, 16>,
    #[cfg(all(feature = "std", not(feature = "safety-critical")))]
    pub resource_tables: Vec<ResourceTable>,
    #[cfg(not(any(feature = "std", )))]
    pub resource_tables: BoundedVec<ResourceTable, 16, crate::bounded_component_infra::ComponentProvider>,
    /// Module instances embedded in this component
    #[cfg(all(feature = "std", feature = "safety-critical"))]
    pub module_instances: WrtVec<ModuleInstance, {CrateId::Component as u8}, 64>,
    #[cfg(all(feature = "std", not(feature = "safety-critical")))]
    pub module_instances: Vec<ModuleInstance>,
    #[cfg(not(any(feature = "std", )))]
    pub module_instances: BoundedVec<ModuleInstance, 64, crate::bounded_component_infra::ComponentProvider>,
}

/// State of a component instance
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComponentInstanceState {
    /// Instance is initialized but not started
    Initialized,
    /// Instance is running
    Running,
    /// Instance is paused
    Paused,
    /// Instance has been stopped or exited
    Stopped,
    /// Instance encountered an error
    Error,
}

impl Default for ComponentInstanceState {
    fn default() -> Self {
        Self::Initialized
    }
}

/// Component model value type
#[derive(Debug, Clone, PartialEq)]
pub enum ValType {
    /// Boolean type
    Bool,
    /// Signed 8-bit integer
    S8,
    /// Unsigned 8-bit integer  
    U8,
    /// Signed 16-bit integer
    S16,
    /// Unsigned 16-bit integer
    U16,
    /// Signed 32-bit integer
    S32,
    /// Unsigned 32-bit integer
    U32,
    /// Signed 64-bit integer
    S64,
    /// Unsigned 64-bit integer
    U64,
    /// 32-bit floating point
    F32,
    /// 64-bit floating point
    F64,
    /// Character type
    Char,
    /// String type
    String,
    /// List type with element type
    List(Box<ValType>),
    /// Record type with named fields
    Record(Record),
    /// Tuple type with element types
    Tuple(Tuple),
    /// Variant type with alternatives
    Variant(Variant),
    /// Enum type with cases
    Enum(Enum),
    /// Option type with payload
    Option(Box<ValType>),
    /// Result type with ok/error types
    Result(Result_),
    /// Flags type with bitfields
    Flags(Flags),
    /// Resource handle
    Own(u32),
    /// Borrowed resource
    Borrow(u32),
    /// Stream type with element type
    Stream(Box<ValType>),
    /// Future type with value type
    Future(Box<ValType>),
}

/// Record type definition
#[derive(Debug, Clone, PartialEq)]
pub struct Record {
    #[cfg(feature = "std")]
    pub fields: Vec<Field>,
    #[cfg(not(any(feature = "std", )))]
    pub fields: BoundedVec<Field, 64, crate::bounded_component_infra::ComponentProvider>,
}

/// Field in a record
#[derive(Debug, Clone, PartialEq)]
pub struct Field {
    #[cfg(feature = "std")]
    pub name: String,
    #[cfg(not(any(feature = "std", )))]
    pub name: BoundedString<64, crate::bounded_component_infra::ComponentProvider>,
    pub ty: ValType,
}

/// Tuple type definition
#[derive(Debug, Clone, PartialEq)]
pub struct Tuple {
    #[cfg(feature = "std")]
    pub types: Vec<ValType>,
    #[cfg(not(any(feature = "std", )))]
    pub types: BoundedVec<ValType, 32, crate::bounded_component_infra::ComponentProvider>,
}

/// Variant type definition
#[derive(Debug, Clone, PartialEq)]
pub struct Variant {
    #[cfg(feature = "std")]
    pub cases: Vec<Case>,
    #[cfg(not(any(feature = "std", )))]
    pub cases: BoundedVec<Case, 64, crate::bounded_component_infra::ComponentProvider>,
}

/// Case in a variant
#[derive(Debug, Clone, PartialEq)]
pub struct Case {
    #[cfg(feature = "std")]
    pub name: String,
    #[cfg(not(any(feature = "std", )))]
    pub name: BoundedString<64, crate::bounded_component_infra::ComponentProvider>,
    pub ty: Option<ValType>,
    pub refines: Option<u32>,
}

/// Enum type definition
#[derive(Debug, Clone, PartialEq)]
pub struct Enum {
    #[cfg(feature = "std")]
    pub cases: Vec<String>,
    #[cfg(not(any(feature = "std", )))]
    pub cases: BoundedVec<BoundedString<64, crate::bounded_component_infra::ComponentProvider>, 64, crate::bounded_component_infra::ComponentProvider>,
}

/// Result type definition (renamed to avoid conflict with std::result::Result)
#[derive(Debug, Clone, PartialEq)]
pub struct Result_ {
    pub ok: Option<Box<ValType>>,
    pub err: Option<Box<ValType>>,
}

/// Flags type definition
#[derive(Debug, Clone, PartialEq)]
pub struct Flags {
    #[cfg(feature = "std")]
    pub labels: Vec<String>,
    #[cfg(not(any(feature = "std", )))]
    pub labels: BoundedVec<BoundedString<64, crate::bounded_component_infra::ComponentProvider>, 64, crate::bounded_component_infra::ComponentProvider>,
}

/// Component model value
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value {
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
    F32(f32),
    /// 64-bit floating point
    F64(f64),
    /// Character value
    Char(char),
    /// String value
    String(BoundedString<1024, crate::bounded_component_infra::ComponentProvider>),
    /// List value
    #[cfg(feature = "std")]
    List(Vec<Value>),
    #[cfg(not(any(feature = "std", )))]
    List(BoundedVec<Value, 256, crate::bounded_component_infra::ComponentProvider>),
    /// Record value
    #[cfg(feature = "std")]
    Record(Vec<Value>),
    #[cfg(not(any(feature = "std", )))]
    Record(BoundedVec<Value, 64, crate::bounded_component_infra::ComponentProvider>),
    /// Tuple value
    #[cfg(feature = "std")]
    Tuple(Vec<Value>),
    #[cfg(not(any(feature = "std", )))]
    Tuple(BoundedVec<Value, 32, crate::bounded_component_infra::ComponentProvider>),
    /// Variant value
    Variant { discriminant: u32, value: Option<Box<Value>> },
    /// Enum value
    Enum(u32),
    /// Option value
    Option(Option<Box<Value>>),
    /// Result value
    Result(core::result::Result<Option<Box<Value>>, Box<Value>>),
    /// Flags value
    Flags(u32),
    /// Owned resource
    Own(u32),
    /// Borrowed resource
    Borrow(u32),
    /// Stream handle
    Stream(StreamHandle),
    /// Future handle
    Future(FutureHandle),
}

impl Default for Value {
    fn default() -> Self {
        Value::Bool(false)
    }
}

impl wrt_foundation::traits::ToBytes for Value {
    fn serialized_size(&self) -> usize {
        match self {
            Value::Bool(_) => 2, // discriminant + bool
            Value::S8(_) | Value::U8(_) => 2, // discriminant + byte
            Value::S16(_) | Value::U16(_) => 3, // discriminant + 2 bytes
            Value::S32(_) | Value::U32(_) | Value::F32(_) => 5, // discriminant + 4 bytes
            Value::S64(_) | Value::U64(_) | Value::F64(_) => 9, // discriminant + 8 bytes
            Value::Char(_) => 5, // discriminant + 4 bytes
            _ => 1, // just discriminant for complex types
        }
    }

    fn to_bytes_with_provider<'a, PStream: wrt_foundation::MemoryProvider>(
        &self,
        writer: &mut wrt_foundation::traits::WriteStream<'a>,
        _provider: &PStream,
    ) -> wrt_foundation::Result<()> {
        use wrt_foundation::traits::WriteStream;
        
        match self {
            Value::Bool(b) => {
                writer.write_u8(0)?; // discriminant
                writer.write_u8(if *b { 1 } else { 0 })?;
            }
            Value::S8(v) => {
                writer.write_u8(1)?;
                writer.write_i8(*v)?;
            }
            Value::U8(v) => {
                writer.write_u8(2)?;
                writer.write_u8(*v)?;
            }
            Value::S16(v) => {
                writer.write_u8(3)?;
                writer.write_i16_le(*v)?;
            }
            Value::U16(v) => {
                writer.write_u8(4)?;
                writer.write_u16_le(*v)?;
            }
            Value::S32(v) => {
                writer.write_u8(5)?;
                writer.write_i32_le(*v)?;
            }
            Value::U32(v) => {
                writer.write_u8(6)?;
                writer.write_u32_le(*v)?;
            }
            Value::S64(v) => {
                writer.write_u8(7)?;
                writer.write_i64_le(*v)?;
            }
            Value::U64(v) => {
                writer.write_u8(8)?;
                writer.write_u64_le(*v)?;
            }
            Value::F32(v) => {
                writer.write_u8(9)?;
                writer.write_f32_le(*v)?;
            }
            Value::F64(v) => {
                writer.write_u8(10)?;
                writer.write_f64_le(*v)?;
            }
            Value::Char(c) => {
                writer.write_u8(11)?;
                writer.write_u32_le(*c as u32)?;
            }
            // For complex types, just store the discriminant
            _ => {
                writer.write_u8(255)?; // generic complex type discriminant
            }
        }
        Ok(())
    }
}

impl wrt_foundation::traits::FromBytes for Value {
    fn from_bytes_with_provider<'a, PStream: wrt_foundation::MemoryProvider>(
        reader: &mut wrt_foundation::traits::ReadStream<'a>,
        _provider: &PStream,
    ) -> wrt_foundation::Result<Self> {
        use wrt_foundation::traits::ReadStream;
        
        let discriminant = reader.read_u8()?;
        
        match discriminant {
            0 => {
                let val = reader.read_u8()?;
                Ok(Value::Bool(val != 0))
            }
            1 => {
                let val = reader.read_i8()?;
                Ok(Value::S8(val))
            }
            2 => {
                let val = reader.read_u8()?;
                Ok(Value::U8(val))
            }
            3 => {
                let val = reader.read_i16_le()?;
                Ok(Value::S16(val))
            }
            4 => {
                let val = reader.read_u16_le()?;
                Ok(Value::U16(val))
            }
            5 => {
                let val = reader.read_i32_le()?;
                Ok(Value::S32(val))
            }
            6 => {
                let val = reader.read_u32_le()?;
                Ok(Value::U32(val))
            }
            7 => {
                let val = reader.read_i64_le()?;
                Ok(Value::S64(val))
            }
            8 => {
                let val = reader.read_u64_le()?;
                Ok(Value::U64(val))
            }
            9 => {
                let val = reader.read_f32_le()?;
                Ok(Value::F32(val))
            }
            10 => {
                let val = reader.read_f64_le()?;
                Ok(Value::F64(val))
            }
            11 => {
                let char_code = reader.read_u32_le()?;
                if let Some(c) = char::from_u32(char_code) {
                    Ok(Value::Char(c))
                } else {
                    Ok(Value::Char('\0'))
                }
            }
            _ => Ok(Value::Bool(false)), // default for complex/unknown types
        }
    }
}

impl wrt_foundation::traits::Checksummable for Value {
    fn update_checksum(&self, checksum: &mut wrt_foundation::verification::Checksum) {
        // Simple checksum based on the discriminant and basic content
        let discriminant = match self {
            Value::Bool(_) => 0u8,
            Value::S8(_) => 1u8,
            Value::U8(_) => 2u8,
            Value::S16(_) => 3u8,
            Value::U16(_) => 4u8,
            Value::S32(_) => 5u8,
            Value::U32(_) => 6u8,
            Value::S64(_) => 7u8,
            Value::U64(_) => 8u8,
            Value::F32(_) => 9u8,
            Value::F64(_) => 10u8,
            Value::Char(_) => 11u8,
            Value::String(_) => 12u8,
            _ => 255u8,
        };
        checksum.update(discriminant);
        
        // For simplicity, just update with basic data
        match self {
            Value::Bool(b) => checksum.update(if *b { 1u8 } else { 0u8 }),
            Value::S8(v) => checksum.update(*v as u8),
            Value::U8(v) => checksum.update(*v),
            Value::S16(v) => checksum.update_slice(&v.to_le_bytes()),
            Value::U16(v) => checksum.update_slice(&v.to_le_bytes()),
            Value::S32(v) => checksum.update_slice(&v.to_le_bytes()),
            Value::U32(v) => checksum.update_slice(&v.to_le_bytes()),
            Value::S64(v) => checksum.update_slice(&v.to_le_bytes()),
            Value::U64(v) => checksum.update_slice(&v.to_le_bytes()),
            Value::F32(v) => checksum.update_slice(&v.to_bits().to_le_bytes()),
            Value::F64(v) => checksum.update_slice(&v.to_bits().to_le_bytes()),
            Value::Char(c) => checksum.update_slice(&(*c as u32).to_le_bytes()),
            Value::String(s) => checksum.update_slice(s.as_bytes()),
            _ => {} // Skip complex types for now
        }
    }
}

/// Component instance identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ComponentInstanceId(pub u32);

/// Type identifier for generative types
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TypeId(pub u32);

/// Resource identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ResourceId(pub u32);

/// Component error types
#[derive(Debug, Clone, PartialEq)]
pub enum ComponentError {
    /// Too many generative types for a single instance
    TooManyGenerativeTypes,
    /// Too many type bounds for a single type
    TooManyTypeBounds,
    /// Resource handle already exists
    ResourceHandleAlreadyExists,
    /// Invalid type reference
    InvalidTypeReference(TypeId, TypeId),
    /// Invalid subtype relation
    InvalidSubtypeRelation(TypeId, TypeId),
    /// Component instantiation failed
    InstantiationFailed,
    /// Resource not found
    ResourceNotFound(u32),
    /// Type mismatch
    TypeMismatch,
    /// Import resolution failed
    ImportResolutionFailed,
    /// Export resolution failed
    ExportResolutionFailed,
}

impl fmt::Display for ComponentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ComponentError::TooManyGenerativeTypes => write!(f, "Too many generative types"),
            ComponentError::TooManyTypeBounds => write!(f, "Too many type bounds"),
            ComponentError::ResourceHandleAlreadyExists => {
                write!(f, "Resource handle already exists")
            }
            ComponentError::InvalidTypeReference(type_id, target_type) => {
                write!(f, "Invalid type reference from {:?} to {:?}", type_id, target_type)
            }
            ComponentError::InvalidSubtypeRelation(sub_type, super_type) => {
                write!(f, "Invalid subtype relation: {:?} <: {:?}", sub_type, super_type)
            }
            ComponentError::InstantiationFailed => write!(f, "Component instantiation failed"),
            ComponentError::ResourceNotFound(handle) => write!(f, "Resource not found: {}", handle),
            ComponentError::TypeMismatch => write!(f, "Type mismatch"),
            ComponentError::ImportResolutionFailed => write!(f, "Import resolution failed"),
            ComponentError::ExportResolutionFailed => write!(f, "Export resolution failed"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for ComponentError {}

// Conversion to wrt_error::Error for unified error handling
impl From<ComponentError> for wrt_error::Error {
    fn from(err: ComponentError) -> Self {
        use wrt_error::{ErrorCategory, codes};
        match err {
            ComponentError::TooManyGenerativeTypes => Self::new(
                ErrorCategory::ComponentRuntime,
                codes::COMPONENT_RESOURCE_LIFECYCLE_ERROR,
                "Too many generative types for component instance",
            ),
            ComponentError::TooManyTypeBounds => Self::new(
                ErrorCategory::ComponentRuntime,
                codes::COMPONENT_CONFIGURATION_INVALID,
                "Too many type bounds for component type",
            ),
            ComponentError::ResourceHandleAlreadyExists => Self::new(
                ErrorCategory::ComponentRuntime,
                codes::COMPONENT_HANDLE_REPRESENTATION_ERROR,
                "Resource handle already exists",
            ),
            ComponentError::InvalidTypeReference(_, _) => Self::new(
                ErrorCategory::ComponentRuntime,
                codes::COMPONENT_ABI_RUNTIME_ERROR,
                "Invalid type reference in component ABI",
            ),
            ComponentError::InvalidSubtypeRelation(_, _) => Self::new(
                ErrorCategory::ComponentRuntime,
                codes::COMPONENT_ABI_RUNTIME_ERROR,
                "Invalid subtype relation in component type system",
            ),
            ComponentError::InstantiationFailed => Self::new(
                ErrorCategory::ComponentRuntime,
                codes::COMPONENT_INSTANTIATION_RUNTIME_ERROR,
                "Component instantiation failed",
            ),
            ComponentError::ResourceNotFound(_) => Self::new(
                ErrorCategory::ComponentRuntime,
                codes::COMPONENT_HANDLE_REPRESENTATION_ERROR,
                "Component resource not found",
            ),
            ComponentError::TypeMismatch => Self::new(
                ErrorCategory::ComponentRuntime,
                codes::COMPONENT_ABI_RUNTIME_ERROR,
                "Component type mismatch",
            ),
            ComponentError::ImportResolutionFailed => Self::new(
                ErrorCategory::ComponentRuntime,
                codes::COMPONENT_INSTANTIATION_RUNTIME_ERROR,
                "Component import resolution failed",
            ),
            ComponentError::ExportResolutionFailed => Self::new(
                ErrorCategory::ComponentRuntime,
                codes::COMPONENT_INSTANTIATION_RUNTIME_ERROR,
                "Component export resolution failed",
            ),
        }
    }
}

// Implement required traits for BoundedVec compatibility
use wrt_foundation::traits::{WriteStream, ReadStream};

// Macro to implement basic traits for complex types
macro_rules! impl_basic_traits {
    ($type:ty, $default_val:expr) => {
        impl Checksummable for $type {
            fn update_checksum(&self, checksum: &mut wrt_foundation::traits::Checksum) {
                // Simple stub implementation
                0u32.update_checksum(checksum);
            }
        }

        impl ToBytes for $type {
            fn to_bytes_with_provider<'a, PStream: wrt_foundation::MemoryProvider>(
                &self,
                _writer: &mut WriteStream<'a>,
                _provider: &PStream,
            ) -> wrt_foundation::WrtResult<()> {
                Ok(())
            }
        }

        impl FromBytes for $type {
            fn from_bytes_with_provider<'a, PStream: wrt_foundation::MemoryProvider>(
                _reader: &mut ReadStream<'a>,
                _provider: &PStream,
            ) -> wrt_foundation::WrtResult<Self> {
                Ok($default_val)
            }
        }
    };
}

// Default implementations for complex types
impl Default for ValType {
    fn default() -> Self {
        ValType::Bool
    }
}

impl Default for Record {
    fn default() -> Self {
        Self {
            #[cfg(feature = "std")]
            fields: Vec::new(),
            #[cfg(not(any(feature = "std", )))]
            fields: BoundedVec::new(crate::bounded_component_infra::ComponentProvider::default()).unwrap(),
        }
    }
}

impl Default for Field {
    fn default() -> Self {
        Self {
            #[cfg(feature = "std")]
            name: String::new(),
            #[cfg(not(any(feature = "std", )))]
            name: BoundedString::new(crate::bounded_component_infra::ComponentProvider::default()),
            ty: ValType::default(),
        }
    }
}

impl Default for Tuple {
    fn default() -> Self {
        Self {
            #[cfg(feature = "std")]
            types: Vec::new(),
            #[cfg(not(any(feature = "std", )))]
            types: BoundedVec::new(crate::bounded_component_infra::ComponentProvider::default()).unwrap(),
        }
    }
}

impl Default for Variant {
    fn default() -> Self {
        Self {
            #[cfg(feature = "std")]
            cases: Vec::new(),
            #[cfg(not(any(feature = "std", )))]
            cases: BoundedVec::new(crate::bounded_component_infra::ComponentProvider::default()).unwrap(),
        }
    }
}

impl Default for Case {
    fn default() -> Self {
        Self {
            #[cfg(feature = "std")]
            name: String::new(),
            #[cfg(not(any(feature = "std", )))]
            name: BoundedString::new(crate::bounded_component_infra::ComponentProvider::default()),
            ty: None,
            refines: None,
        }
    }
}

// Apply macro to all complex types
impl_basic_traits!(ValType, ValType::default());
impl_basic_traits!(Record, Record::default());
impl_basic_traits!(Field, Field::default());
impl_basic_traits!(Tuple, Tuple::default());
impl_basic_traits!(Variant, Variant::default());
impl_basic_traits!(Case, Case::default());

