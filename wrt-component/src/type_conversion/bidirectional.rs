//! Bidirectional type conversion between format and runtime types
//!
//! This module provides comprehensive bidirectional conversion between
//! `wrt_format::component` types and `wrt_foundation` types, ensuring type
//! compatibility across the system boundary.
//!
//! # Examples
//!
//! ```
//! use wrt_component::type_conversion::bidirectional::{
//!     format_to_runtime_extern_type, runtime_to_format_extern_type
//! };
//! use wrt_format::component::ValType as FormatValType;
//! use wrt_format::component::ExternType as FormatExternType;
//! use wrt_foundation::ExternType as RuntimeExternType;
//!
//! // Convert a format function type to a runtime function type
//! let format_func = FormatExternType::Function {
//!     params: vec![("arg".to_string(), FormatValType::S32)],
//!     results: vec![FormatValType::S32],
//! };
//!
//! let runtime_func = format_to_runtime_extern_type(&format_func).unwrap();
//!
//! // Convert back to format type
//! let format_func_again = runtime_to_format_extern_type(&runtime_func).unwrap();
//! ```

// Explicitly import the types we need to avoid confusion
use wrt_error::kinds::{InvalidArgumentError, NotImplementedError};
use wrt_format::component::{
    ComponentTypeDefinition, ConstValue as FormatConstValue, ExternType as FormatExternType,
    FormatResourceOperation, ResourceRepresentation, ValType as FormatValType,
};
use wrt_foundation::{
    component::{ComponentType, FuncType as TypesFuncType, InstanceType},
    component_value::{ComponentValue, ValType as TypesValType},
    resource::{ResourceOperation, ResourceType},
    types::ValueType,
    values::Value,
    ExternType as TypesExternType,
};

use crate::prelude::*;

// Helper functions to handle type conversions with correct parameters

// Special helper functions for FormatValType to ValueType conversion
fn convert_format_valtype_to_valuetype(format_val_type: &FormatValType) -> Result<ValueType> {
    match format_val_type {
        FormatValType::S32 => Ok(ValueType::I32),
        FormatValType::S64 => Ok(ValueType::I64),
        FormatValType::F32 => Ok(ValueType::F32),
        FormatValType::F64 => Ok(ValueType::F64),
        _ => Err(Error::new(
            ErrorCategory::Type,
            codes::NOT_IMPLEMENTED,
            "Component not found",
        )),
    }
}

// Variant that accepts ValType (TypesValType) for use at call sites
fn convert_types_valtype_to_valuetype(val_type: &TypesValType) -> Result<ValueType> {
    match val_type {
        TypesValType::S32 => Ok(ValueType::I32),
        TypesValType::S64 => Ok(ValueType::I64),
        TypesValType::F32 => Ok(ValueType::F32),
        TypesValType::F64 => Ok(ValueType::F64),
        _ => Err(Error::new(
            ErrorCategory::Type,
            codes::NOT_IMPLEMENTED,
            "Component not found",
        )),
    }
}

// Special helper function for FormatValType to TypesValType conversion
fn convert_format_to_types_valtype(format_val_type: &FormatValType) -> TypesValType {
    match format_val_type {
        FormatValType::Bool => TypesValType::Bool,
        FormatValType::S8 => TypesValType::S8,
        FormatValType::U8 => TypesValType::U8,
        FormatValType::S16 => TypesValType::S16,
        FormatValType::U16 => TypesValType::U16,
        FormatValType::S32 => TypesValType::S32,
        FormatValType::U32 => TypesValType::U32,
        FormatValType::S64 => TypesValType::S64,
        FormatValType::U64 => TypesValType::U64,
        FormatValType::F32 => TypesValType::F32,
        FormatValType::F64 => TypesValType::F64,
        FormatValType::Char => TypesValType::Char,
        FormatValType::String => TypesValType::String,
        FormatValType::Ref(idx) => TypesValType::Ref(*idx),
        FormatValType::Own(idx) => TypesValType::Own(*idx),
        FormatValType::Borrow(idx) => TypesValType::Borrow(*idx),
        _ => TypesValType::Void, // Default fallback
    }
}

// Variant that takes a ValType directly for use at call sites
fn convert_types_valtype_identity(val_type: &TypesValType) -> TypesValType {
    val_type.clone()
}

// Special helper function for TypesValType to FormatValType conversion
fn convert_types_to_format_valtype(types_val_type: &TypesValType) -> FormatValType {
    match types_val_type {
        TypesValType::Bool => FormatValType::Bool,
        TypesValType::S8 => FormatValType::S8,
        TypesValType::U8 => FormatValType::U8,
        TypesValType::S16 => FormatValType::S16,
        TypesValType::U16 => FormatValType::U16,
        TypesValType::S32 => FormatValType::S32,
        TypesValType::U32 => FormatValType::U32,
        TypesValType::S64 => FormatValType::S64,
        TypesValType::U64 => FormatValType::U64,
        TypesValType::F32 => FormatValType::F32,
        TypesValType::F64 => FormatValType::F64,
        TypesValType::Char => FormatValType::Char,
        TypesValType::String => FormatValType::String,
        TypesValType::Ref(idx) => FormatValType::Ref(*idx),
        TypesValType::Own(idx) => FormatValType::Own(*idx),
        TypesValType::Borrow(idx) => FormatValType::Borrow(*idx),
        _ => FormatValType::Bool, // Default fallback
    }
}

/// Convert a ValueType to a FormatValType
///
/// This function converts from wrt_foundation::types::ValueType to
/// wrt_format::component::ValType directly.
///
/// # Arguments
///
/// * `value_type` - The core WebAssembly value type to convert
///
/// # Returns
///
/// A Result containing the converted format value type, or an error if
/// conversion is not possible
///
/// # Examples
///
/// ```
/// use wrt_component::type_conversion::bidirectional::value_type_to_format_val_type;
/// use wrt_foundation::types::ValueType;
///
/// let i32_type = ValueType::I32;
/// let format_type = value_type_to_format_val_type(&i32_type).unwrap();
/// assert!(matches!(format_type, wrt_format::component::ValType::S32));
/// ```
pub fn value_type_to_format_val_type(value_type: &ValueType) -> Result<FormatValType> {
    match value_type {
        ValueType::I32 => Ok(FormatValType::S32),
        ValueType::I64 => Ok(FormatValType::S64),
        ValueType::F32 => Ok(FormatValType::F32),
        ValueType::F64 => Ok(FormatValType::F64),
        ValueType::FuncRef => Err(Error::new(
            ErrorCategory::Type,
            codes::NOT_IMPLEMENTED,
            NotImplementedError(
                "FuncRef type not directly convertible to component format".to_string(),
            ),
        )),
        ValueType::ExternRef => Err(Error::new(
            ErrorCategory::Type,
            codes::NOT_IMPLEMENTED,
            NotImplementedError(
                "ExternRef type not directly convertible to component format".to_string(),
            ),
        )),
    }
}

/// Convert FormatValType to ValueType
///
/// Converts a component model value type to a core WebAssembly value type.
///
/// # Arguments
///
/// * `format_val_type` - The format value type to convert
///
/// # Returns
///
/// A Result containing the converted core value type, or an error if
/// conversion is not possible
///
/// # Examples
///
/// ```
/// use wrt_component::type_conversion::bidirectional::format_val_type_to_value_type;
/// use wrt_format::component::ValType;
///
/// let s32_type = ValType::S32;
/// let core_type = format_val_type_to_value_type(&s32_type).unwrap();
/// assert!(matches!(core_type, wrt_foundation::types::ValueType::I32));
/// ```
pub fn format_val_type_to_value_type(format_val_type: &FormatValType) -> Result<ValueType> {
    convert_format_valtype_to_valuetype(format_val_type)
}

/// Convert TypesValType to ValueType
///
/// Converts a runtime component value type to a core WebAssembly value type.
///
/// # Arguments
///
/// * `types_val_type` - The runtime value type to convert
///
/// # Returns
///
/// A Result containing the converted core value type, or an error if
/// conversion is not possible
///
/// # Examples
///
/// ```
/// use wrt_component::type_conversion::bidirectional::types_valtype_to_valuetype;
/// use wrt_foundation::component_value::ValType;
///
/// let s32_type = ValType::S32;
/// let core_type = types_valtype_to_valuetype(&s32_type).unwrap();
/// assert!(matches!(core_type, wrt_foundation::types::ValueType::I32));
/// ```
pub fn types_valtype_to_valuetype(types_val_type: &TypesValType) -> Result<ValueType> {
    convert_types_valtype_to_valuetype(types_val_type)
}

/// Convert ValueType to TypesValType
///
/// Converts a core WebAssembly value type to the runtime component value type.
///
/// # Arguments
///
/// * `value_type` - The core value type to convert
///
/// # Returns
///
/// The corresponding runtime component value type
///
/// # Examples
///
/// ```
/// use wrt_component::type_conversion::bidirectional::value_type_to_types_valtype;
/// use wrt_foundation::types::ValueType;
///
/// let i32_type = ValueType::I32;
/// let runtime_type = value_type_to_types_valtype(&i32_type);
/// assert!(matches!(runtime_type, wrt_foundation::component_value::ValType::S32));
/// ```
pub fn value_type_to_types_valtype(value_type: &ValueType) -> TypesValType {
    match value_type {
        ValueType::I32 => TypesValType::S32,
        ValueType::I64 => TypesValType::S64,
        ValueType::F32 => TypesValType::F32,
        ValueType::F64 => TypesValType::F64,
        ValueType::FuncRef => TypesValType::Own(0), // Default to resource type 0
        ValueType::ExternRef => TypesValType::Ref(0), // Default to type index 0
    }
}

/// Convert FormatValType to TypesValType
///
/// Comprehensive conversion from format value type to runtime component value
/// type.
///
/// # Arguments
///
/// * `format_val_type` - The format value type to convert
///
/// # Returns
///
/// The corresponding runtime component value type
///
/// # Examples
///
/// ```
/// use wrt_component::type_conversion::bidirectional::format_valtype_to_types_valtype;
/// use wrt_format::component::ValType;
///
/// let string_type = ValType::String;
/// let runtime_type = format_valtype_to_types_valtype(&string_type);
/// assert!(matches!(runtime_type, wrt_foundation::component_value::ValType::String));
/// ```
pub fn format_valtype_to_types_valtype(format_val_type: &FormatValType) -> TypesValType {
    convert_format_to_types_valtype(format_val_type)
}

/// Format type to types ValType helper function
///
/// This is a public entry point for the helper function to ensure
/// compatibility.
///
/// # Arguments
///
/// * `val_type` - The ValType to convert
///
/// # Returns
///
/// The corresponding TypesValType
pub fn format_to_types_valtype(val_type: &TypesValType) -> TypesValType {
    convert_types_valtype_identity(val_type)
}

/// Convert TypesValType to FormatValType
///
/// Comprehensive conversion from runtime component value type to format value
/// type.
///
/// # Arguments
///
/// * `types_val_type` - The runtime component value type to convert
///
/// # Returns
///
/// The corresponding format value type
///
/// # Examples
///
/// ```
/// use wrt_component::type_conversion::bidirectional::types_valtype_to_format_valtype;
/// use wrt_foundation::component_value::ValType;
///
/// let string_type = ValType::String;
/// let format_type = types_valtype_to_format_valtype(&string_type);
/// assert!(matches!(format_type, wrt_format::component::ValType::String));
/// ```
pub fn types_valtype_to_format_valtype(types_val_type: &TypesValType) -> FormatValType {
    match types_val_type {
        TypesValType::Bool => FormatValType::Bool,
        TypesValType::S8 => FormatValType::S8,
        TypesValType::U8 => FormatValType::U8,
        TypesValType::S16 => FormatValType::S16,
        TypesValType::U16 => FormatValType::U16,
        TypesValType::S32 => FormatValType::S32,
        TypesValType::U32 => FormatValType::U32,
        TypesValType::S64 => FormatValType::S64,
        TypesValType::U64 => FormatValType::U64,
        TypesValType::F32 => FormatValType::F32,
        TypesValType::F64 => FormatValType::F64,
        TypesValType::Char => FormatValType::Char,
        TypesValType::String => FormatValType::String,
        TypesValType::Ref(idx) => FormatValType::Ref(*idx),
        TypesValType::Record(fields) => {
            let converted_fields = fields
                .iter()
                .map(|(name, val_type)| (name.clone(), types_valtype_to_format_valtype(val_type)))
                .collect();
            FormatValType::Record(converted_fields)
        }
        TypesValType::Variant(cases) => {
            let converted_cases = cases
                .iter()
                .map(|(name, opt_type)| {
                    (
                        name.clone(),
                        opt_type.as_ref().map(|val_type| types_valtype_to_format_valtype(val_type)),
                    )
                })
                .collect();
            FormatValType::Variant(converted_cases)
        }
        TypesValType::List(elem_type) => {
            FormatValType::List(Box::new(types_valtype_to_format_valtype(elem_type)))
        }
        TypesValType::FixedList(elem_type, size) => {
            FormatValType::FixedList(Box::new(types_valtype_to_format_valtype(elem_type)), *size)
        }
        TypesValType::Tuple(types) => {
            let converted_types =
                types.iter().map(|val_type| types_valtype_to_format_valtype(val_type)).collect();
            FormatValType::Tuple(converted_types)
        }
        TypesValType::Flags(names) => FormatValType::Flags(names.clone()),
        TypesValType::Enum(variants) => FormatValType::Enum(variants.clone()),
        TypesValType::Option(inner_type) => {
            FormatValType::Option(Box::new(types_valtype_to_format_valtype(inner_type)))
        }
        TypesValType::Result(result_type) => {
            FormatValType::Result(Box::new(types_valtype_to_format_valtype(result_type)))
        }
        TypesValType::Own(idx) => FormatValType::Own(*idx),
        TypesValType::Borrow(idx) => FormatValType::Borrow(*idx),
        TypesValType::Void => {
            // Map void to a default type (this is a simplification)
            FormatValType::Bool
        }
        TypesValType::ErrorContext => FormatValType::ErrorContext,
        TypesValType::Result { ok: _, err: _ } => {
            // Map to FormatValType::Result with a placeholder type
            FormatValType::Result(Box::new(FormatValType::Unit))
        } // All enums handled above
    }
}

/// Convert FormatExternType to TypesExternType
///
/// Comprehensive conversion from format external type to runtime external type.
///
/// # Arguments
///
/// * `format_extern_type` - The format external type to convert
///
/// # Returns
///
/// Result containing the corresponding runtime external type, or an error if
/// conversion is not possible
///
/// # Examples
///
/// ```
/// use wrt_component::type_conversion::bidirectional::format_to_runtime_extern_type;
/// use wrt_format::component::ExternType as FormatExternType;
/// use wrt_format::component::ValType as FormatValType;
///
/// let format_func = FormatExternType::Function {
///     params: vec![("param".to_string(), FormatValType::S32)],
///     results: vec![FormatValType::S32],
/// };
///
/// let runtime_func = format_to_runtime_extern_type(&format_func).unwrap();
/// ```
pub fn format_to_runtime_extern_type(
    format_extern_type: &FormatExternType,
) -> Result<TypesExternType> {
    match format_extern_type {
        FormatExternType::Function { params, results } => {
            // Convert all parameter types to core ValueType
            let converted_params = params
                .iter()
                .map(|(name, val_type)| format_val_type_to_value_type(val_type))
                .collect::<Result<Vec<_>>>()?;

            // Convert all result types to core ValueType
            let converted_results = results
                .iter()
                .map(|val_type| format_val_type_to_value_type(val_type))
                .collect::<Result<Vec<_>>>()?;

            Ok(TypesExternType::Function(TypesFuncType::new(converted_params, converted_results)))
        }
        FormatExternType::Value(val_type) => {
            // Convert to most appropriate TypesExternType - likely Function with no
            // params/results Could be mapped as constant global in the future
            let value_type = format_val_type_to_value_type(val_type).unwrap_or(ValueType::I32);
            Ok(TypesExternType::Global(wrt_foundation::component::GlobalType {
                value_type,
                mutable: false,
            }))
        }
        FormatExternType::Type(type_idx) => {
            // Type reference - this would need context from the component
            // For now, provide a sensible default
            Ok(TypesExternType::Function(TypesFuncType::new(vec![], vec![])))
        }
        FormatExternType::Instance { exports } => {
            // Convert each export to a TypesExternType
            let converted_exports: core::result::Result<Vec<(String, TypesExternType)>> = exports
                .iter()
                .map(|(name, ext_type)| {
                    Ok((name.clone(), format_to_runtime_extern_type(ext_type)?))
                })
                .collect();

            Ok(TypesExternType::Instance(InstanceType { exports: converted_exports? }))
        }
        FormatExternType::Component { imports, exports } => {
            // Convert imports to TypesExternType
            let converted_imports: core::result::Result<Vec<(String, String, TypesExternType)>> = imports
                .iter()
                .map(|(ns, name, ext_type)| {
                    Ok((ns.clone(), name.clone(), format_to_runtime_extern_type(ext_type)?))
                })
                .collect();

            // Convert exports to TypesExternType
            let converted_exports: core::result::Result<Vec<(String, TypesExternType)>> = exports
                .iter()
                .map(|(name, ext_type)| {
                    Ok((name.clone(), format_to_runtime_extern_type(ext_type)?))
                })
                .collect();

            Ok(TypesExternType::Component(ComponentType::new(
                converted_imports?,
                converted_exports?,
            )))
        }
    }
}

/// Convert TypesExternType to FormatExternType
///
/// Comprehensive conversion from runtime external type to format external type.
///
/// # Arguments
///
/// * `types_extern_type` - The runtime external type to convert
///
/// # Returns
///
/// Result containing the corresponding format external type, or an error if
/// conversion is not possible
///
/// # Examples
///
/// ```
/// use wrt_component::type_conversion::bidirectional::runtime_to_format_extern_type;
/// use wrt_foundation::{ExternType, component::FuncType};
/// use wrt_foundation::types::ValueType;
///
/// let func_type = FuncType {
///     params: vec![ValueType::I32],
///     results: vec![ValueType::I32],
/// };
///
/// let runtime_func = ExternType::Function(func_type);
/// let format_func = runtime_to_format_extern_type(&runtime_func).unwrap();
/// ```
pub fn runtime_to_format_extern_type(
    types_extern_type: &TypesExternType,
) -> Result<FormatExternType> {
    match types_extern_type {
        ExternType::Function(func_type) => {
            // Convert parameter types
            let param_names: Vec<String> =
                (0..func_type.params.len()).map(|i| "Component not found").collect();

            // Create param_types manually to handle errors gracefully
            let mut param_types = Vec::new();
            for (i, value_type) in func_type.params.iter().enumerate() {
                match value_type_to_format_val_type(value_type) {
                    Ok(format_val_type) => {
                        param_types.push((param_names[i].clone(), format_val_type))
                    }
                    Err(e) => return Err(e),
                }
            }

            // Create result_types manually to handle errors gracefully
            let mut result_types = Vec::new();
            for value_type in &func_type.results {
                match value_type_to_format_val_type(value_type) {
                    Ok(format_val_type) => result_types.push(format_val_type),
                    Err(e) => return Err(e),
                }
            }

            Ok(FormatExternType::Function { params: param_types, results: result_types })
        }
        ExternType::Table(table_type) => Err(Error::new(
            ErrorCategory::System,
            codes::NOT_IMPLEMENTED,
            "Table ExternType not supported in wrt_format::component".to_string(),
        )),
        ExternType::Memory(memory_type) => Err(Error::new(
            ErrorCategory::System,
            codes::NOT_IMPLEMENTED,
            "Memory ExternType not supported in wrt_format::component".to_string(),
        )),
        ExternType::Global(global_type) => Err(Error::new(
            ErrorCategory::System,
            codes::NOT_IMPLEMENTED,
            "Global ExternType not supported in wrt_format::component".to_string(),
        )),
        ExternType::Instance(instance_type) => {
            // Convert exports to FormatExternType
            let exports_format: core::result::Result<Vec<(String, FormatExternType)>> = instance_type
                .exports
                .iter()
                .map(|(name, ext_type)| {
                    Ok((name.clone(), runtime_to_format_extern_type(ext_type)?))
                })
                .collect();

            Ok(FormatExternType::Instance { exports: exports_format? })
        }
        ExternType::Component(component_type) => {
            // Convert imports to FormatExternType
            let imports_format: core::result::Result<Vec<(String, String, FormatExternType)>> = component_type
                .imports
                .iter()
                .map(|(ns, name, ext_type)| {
                    Ok((ns.clone(), name.clone(), runtime_to_format_extern_type(ext_type)?))
                })
                .collect();

            // Convert exports to FormatExternType
            let exports_format: core::result::Result<Vec<(String, FormatExternType)>> = component_type
                .exports
                .iter()
                .map(|(name, ext_type)| {
                    Ok((name.clone(), runtime_to_format_extern_type(ext_type)?))
                })
                .collect();

            Ok(FormatExternType::Component { imports: imports_format?, exports: exports_format? })
        }
        ExternType::Resource(resource_type) => {
            // Note: Since FormatExternType doesn't have a direct Resource variant,
            // we map it to a Value type with the appropriate representation
            let val_type = match resource_type.rep_type {
                ValueType::I32 => FormatValType::Own(0), // Use type index 0 as default
                ValueType::I64 => FormatValType::Own(1), // Use type index 1 as default
                _ => FormatValType::Own(0),              // Default to type index 0
            };

            Ok(FormatExternType::Value(convert_types_to_format_valtype(&val_type)))
        }
    }
}

/// Convert the format ValType to the common ValueType used in the runtime
///
/// # Arguments
///
/// * `val_type` - The format value type to convert
///
/// # Returns
///
/// Result containing the converted core value type, or an error if
/// conversion is not possible
pub fn format_to_common_val_type(val_type: &FormatValType) -> Result<ValueType> {
    match val_type {
        FormatValType::S32 => Ok(ValueType::I32),
        FormatValType::S64 => Ok(ValueType::I64),
        FormatValType::F32 => Ok(ValueType::F32),
        FormatValType::F64 => Ok(ValueType::F64),
        _ => Err(Error::new(
            ErrorCategory::Type,
            codes::NOT_IMPLEMENTED,
            NotImplementedError("Component not found"),
        )),
    }
}

/// Convert the common ValueType to a format ValType
///
/// # Arguments
///
/// * `value_type` - The core value type to convert
///
/// # Returns
///
/// Result containing the converted format value type, or an error if
/// conversion is not possible
pub fn common_to_format_val_type(value_type: &ValueType) -> Result<FormatValType> {
    match value_type {
        ValueType::I32 => Ok(FormatValType::S32),
        ValueType::I64 => Ok(FormatValType::S64),
        ValueType::F32 => Ok(FormatValType::F32),
        ValueType::F64 => Ok(FormatValType::F64),
        _ => Err(Error::new(
            ErrorCategory::Type,
            codes::NOT_IMPLEMENTED,
            NotImplementedError(format!(
                "Value type {:?} cannot be directly mapped to component format",
                value_type
            )),
        )),
    }
}

/// Convert an ExternType to a FuncType if it represents a function
///
/// # Arguments
///
/// * `extern_type` - The external type to convert
///
/// # Returns
///
/// The function type if the extern type is a function, or an error otherwise
pub fn extern_type_to_func_type(extern_type: &ExternType) -> Result<TypesFuncType> {
    match extern_type {
        ExternType::Function(func_type) => Ok(func_type.clone()),
        _ => Err(Error::new(
            ErrorCategory::Type,
            codes::INVALID_TYPE,
            InvalidArgumentError("Component not found"),
        )),
    }
}

/// Trait for types that can be converted to runtime types
pub trait IntoRuntimeType<T> {
    /// Convert to runtime type
    fn into_runtime_type(self) -> Result<T>;
}

/// Trait for types that can be converted to format types
pub trait IntoFormatType<T> {
    /// Convert to format type
    fn into_format_type(self) -> Result<T>;
}

impl IntoRuntimeType<TypesExternType> for FormatExternType {
    fn into_runtime_type(self) -> Result<TypesExternType> {
        format_to_runtime_extern_type(&self)
    }
}

impl IntoFormatType<FormatExternType> for TypesExternType {
    fn into_format_type(self) -> Result<FormatExternType> {
        runtime_to_format_extern_type(&self)
    }
}

impl IntoRuntimeType<TypesValType> for FormatValType {
    fn into_runtime_type(self) -> Result<TypesValType> {
        Ok(format_valtype_to_types_valtype(&self))
    }
}

impl IntoFormatType<FormatValType> for TypesValType {
    fn into_format_type(self) -> Result<FormatValType> {
        Ok(types_valtype_to_format_valtype(&self))
    }
}

/// Convert FormatConstValue to TypesComponentValue
///
/// Comprehensive conversion from format constant value to runtime component
/// value.
///
/// # Arguments
///
/// * `format_const_value` - The format constant value to convert
///
/// # Returns
///
/// The corresponding runtime component value
///
/// # Examples
///
/// ```
/// use wrt_component::type_conversion::bidirectional::format_constvalue_to_types_componentvalue;
/// use wrt_format::component::ConstValue;
///
/// let s32_val = ConstValue::S32(42);
/// let runtime_val = format_constvalue_to_types_componentvalue(&s32_val).unwrap();
/// assert!(matches!(runtime_val, wrt_foundation::component_value::ComponentValue::S32(42)));
/// ```
pub fn format_constvalue_to_types_componentvalue(
    format_const_value: &FormatConstValue,
) -> Result<ComponentValue> {
    match format_const_value {
        FormatConstValue::Bool(v) => Ok(ComponentValue::Bool(*v)),
        FormatConstValue::S8(v) => Ok(ComponentValue::S8(*v)),
        FormatConstValue::U8(v) => Ok(ComponentValue::U8(*v)),
        FormatConstValue::S16(v) => Ok(ComponentValue::S16(*v)),
        FormatConstValue::U16(v) => Ok(ComponentValue::U16(*v)),
        FormatConstValue::S32(v) => Ok(ComponentValue::S32(*v)),
        FormatConstValue::U32(v) => Ok(ComponentValue::U32(*v)),
        FormatConstValue::S64(v) => Ok(ComponentValue::S64(*v)),
        FormatConstValue::U64(v) => Ok(ComponentValue::U64(*v)),
        FormatConstValue::F32(v) => Ok(ComponentValue::F32(*v)),
        FormatConstValue::F64(v) => Ok(ComponentValue::F64(*v)),
        FormatConstValue::Char(v) => Ok(ComponentValue::Char(*v)),
        FormatConstValue::String(v) => Ok(ComponentValue::String(v.clone())),
        FormatConstValue::Null => Ok(ComponentValue::Void),
    }
}

/// Convert TypesComponentValue to FormatConstValue
///
/// Comprehensive conversion from runtime component value to format constant
/// value.
///
/// # Arguments
///
/// * `types_component_value` - The runtime component value to convert
///
/// # Returns
///
/// Result containing the corresponding format constant value, or an error if
/// conversion is not possible
///
/// # Examples
///
/// ```
/// use wrt_component::type_conversion::bidirectional::types_componentvalue_to_format_constvalue;
/// use wrt_foundation::component_value::ComponentValue;
///
/// let s32_val = ComponentValue::S32(42);
/// let format_val = types_componentvalue_to_format_constvalue(&s32_val).unwrap();
/// assert!(matches!(format_val, wrt_format::component::ConstValue::S32(42)));
/// ```
pub fn types_componentvalue_to_format_constvalue(
    types_component_value: &ComponentValue,
) -> Result<FormatConstValue> {
    match types_component_value {
        ComponentValue::Bool(v) => Ok(FormatConstValue::Bool(*v)),
        ComponentValue::S8(v) => Ok(FormatConstValue::S8(*v)),
        ComponentValue::U8(v) => Ok(FormatConstValue::U8(*v)),
        ComponentValue::S16(v) => Ok(FormatConstValue::S16(*v)),
        ComponentValue::U16(v) => Ok(FormatConstValue::U16(*v)),
        ComponentValue::S32(v) => Ok(FormatConstValue::S32(*v)),
        ComponentValue::U32(v) => Ok(FormatConstValue::U32(*v)),
        ComponentValue::S64(v) => Ok(FormatConstValue::S64(*v)),
        ComponentValue::U64(v) => Ok(FormatConstValue::U64(*v)),
        ComponentValue::F32(v) => Ok(FormatConstValue::F32(*v)),
        ComponentValue::F64(v) => Ok(FormatConstValue::F64(*v)),
        ComponentValue::Char(v) => Ok(FormatConstValue::Char(*v)),
        ComponentValue::String(v) => Ok(FormatConstValue::String(v.clone())),
        ComponentValue::Void => Ok(FormatConstValue::Null),
        _ => Err(Error::new(
            ErrorCategory::Type,
            codes::CONVERSION_ERROR,
            NotImplementedError(format!(
                "Cannot convert {:?} to format ConstValue",
                types_component_value
            )),
        )),
    }
}

/// Convert a core WebAssembly value to a runtime component value
///
/// This replaces the existing functionality in
/// wrt-foundation/src/component_value.rs to consolidate value conversions in
/// the same crate as type conversions.
///
/// # Arguments
///
/// * `value` - The core value to convert
///
/// # Returns
///
/// Result containing the converted component value, or an error if
/// conversion is not possible
pub fn core_value_to_types_componentvalue(
    value: &wrt_foundation::values::Value,
) -> Result<ComponentValue> {
    match value {
        wrt_foundation::values::Value::I32(v) => Ok(ComponentValue::S32(*v)),
        wrt_foundation::values::Value::I64(v) => Ok(ComponentValue::S64(*v)),
        wrt_foundation::values::Value::F32(v) => Ok(ComponentValue::F32(*v)),
        wrt_foundation::values::Value::F64(v) => Ok(ComponentValue::F64(*v)),
        wrt_foundation::values::Value::Ref(v) => Ok(ComponentValue::U32(*v)), // Map reference
        // to U32
        _ => Err(Error::new(
            ErrorCategory::Type,
            codes::CONVERSION_ERROR,
            NotImplementedError(
                "Unsupported value type for conversion to component value".to_string(),
            ),
        )),
    }
}

/// Convert a runtime component value to a core WebAssembly value
///
/// This replaces the existing functionality in
/// wrt-foundation/src/component_value.rs to consolidate value conversions in
/// the same crate as type conversions.
///
/// # Arguments
///
/// * `component_value` - The component value to convert
///
/// # Returns
///
/// Result containing the converted core value, or an error if
/// conversion is not possible
pub fn types_componentvalue_to_core_value(
    component_value: &ComponentValue,
) -> Result<wrt_foundation::values::Value> {
    match component_value {
        ComponentValue::Bool(v) => Ok(wrt_foundation::values::Value::I32(if *v { 1 } else { 0 })),
        ComponentValue::S8(v) => Ok(wrt_foundation::values::Value::I32(*v as i32)),
        ComponentValue::U8(v) => Ok(wrt_foundation::values::Value::I32(*v as i32)),
        ComponentValue::S16(v) => Ok(wrt_foundation::values::Value::I32(*v as i32)),
        ComponentValue::U16(v) => Ok(wrt_foundation::values::Value::I32(*v as i32)),
        ComponentValue::S32(v) => Ok(wrt_foundation::values::Value::I32(*v)),
        ComponentValue::U32(v) => {
            // For U32, check if it represents a reference value (e.g., resource handle)
            // For now, we'll treat all U32 as potential references to maintain
            // compatibility A more sophisticated approach might involve
            // checking the context
            if let Some(resource_index) = is_resource_reference(*v) {
                Ok(wrt_foundation::values::Value::Ref(resource_index))
            } else {
                Ok(wrt_foundation::values::Value::I32(*v as i32))
            }
        }
        ComponentValue::S64(v) => Ok(wrt_foundation::values::Value::I64(*v)),
        ComponentValue::U64(v) => Ok(wrt_foundation::values::Value::I64(*v as i64)),
        ComponentValue::F32(v) => Ok(wrt_foundation::values::Value::F32(*v)),
        ComponentValue::F64(v) => Ok(wrt_foundation::values::Value::F64(*v)),
        _ => Err(Error::new(
            ErrorCategory::Type,
            codes::CONVERSION_ERROR,
            NotImplementedError(
                "Unsupported component value type for conversion to core value".to_string(),
            ),
        )),
    }
}

/// Helper function to determine if a U32 value represents a resource reference
/// This is a placeholder - in a real implementation, this might check against
/// a registry of resource handles or use contextual information.
fn is_resource_reference(value: u32) -> Option<u32> {
    // For now, we'll always return None, defaulting to treating U32 as I32
    // In a more complete implementation, this would check if the value is a valid
    // resource handle
    None
}

// Aliases for backward compatibility
pub use format_to_runtime_extern_type as format_to_types_extern_type;
pub use runtime_to_format_extern_type as types_to_format_extern_type;

/// Complete bidirectional conversion between wrt_foundation::ExternType and
/// wrt_format::component::ExternType
///
/// This function handles all ExternType variants comprehensively, fixing
/// previous compatibility issues.
///
/// # Arguments
///
/// * `types_extern_type` - The wrt_foundation::ExternType to convert
///
/// # Returns
///
/// * Result containing the converted wrt_format::component::ExternType or an
///   error
pub fn complete_types_to_format_extern_type(
    types_extern_type: &wrt_foundation::ExternType,
) -> Result<wrt_format::component::ExternType> {
    match types_extern_type {
        wrt_foundation::ExternType::Function(func_type) => {
            // Convert parameter types
            let param_names: Vec<String> =
                (0..func_type.params.len()).map(|i| "Component not found").collect();

            // Create param_types manually to handle errors gracefully
            let mut param_types = Vec::new();
            for (i, value_type) in func_type.params.iter().enumerate() {
                match value_type_to_format_val_type(value_type) {
                    Ok(format_val_type) => {
                        param_types.push((param_names[i].clone(), format_val_type))
                    }
                    Err(e) => return Err(e),
                }
            }

            // Create result_types manually to handle errors gracefully
            let mut result_types = Vec::new();
            for value_type in &func_type.results {
                match value_type_to_format_val_type(value_type) {
                    Ok(format_val_type) => result_types.push(format_val_type),
                    Err(e) => return Err(e),
                }
            }

            Ok(FormatExternType::Function { params: param_types, results: result_types })
        }
        wrt_foundation::ExternType::Table(table_type) => Err(Error::new(
            ErrorCategory::Type,
            codes::CONVERSION_ERROR,
            "Table ExternType not supported in component model format".to_string(),
        )),
        wrt_foundation::ExternType::Memory(memory_type) => Err(Error::new(
            ErrorCategory::Type,
            codes::CONVERSION_ERROR,
            "Memory ExternType not supported in component model format".to_string(),
        )),
        wrt_foundation::ExternType::Global(global_type) => Err(Error::new(
            ErrorCategory::Type,
            codes::CONVERSION_ERROR,
            "Global ExternType not supported in component model format".to_string(),
        )),
        wrt_foundation::ExternType::Resource(resource_type) => {
            // For resources, we convert to a Type reference for now
            // In the future, this could be expanded to include full resource types
            Ok(FormatExternType::Type(0))
        }
        wrt_foundation::ExternType::Instance(instance_type) => {
            // Convert instance exports
            let exports_result: core::result::Result<Vec<(String, FormatExternType)>> = instance_type
                .exports
                .iter()
                .map(|(name, extern_type)| {
                    let format_extern = complete_types_to_format_extern_type(extern_type)?;
                    Ok((name.clone(), format_extern))
                })
                .collect();

            Ok(FormatExternType::Instance { exports: exports_result? })
        }
        wrt_foundation::ExternType::Component(component_type) => {
            // Convert component imports
            let imports_result: core::result::Result<Vec<(String, String, FormatExternType)>> = component_type
                .imports
                .iter()
                .map(|(namespace, name, extern_type)| {
                    let format_extern = complete_types_to_format_extern_type(extern_type)?;
                    Ok((namespace.clone(), name.clone(), format_extern))
                })
                .collect();

            // Convert component exports
            let exports_result: core::result::Result<Vec<(String, FormatExternType)>> = component_type
                .exports
                .iter()
                .map(|(name, extern_type)| {
                    let format_extern = complete_types_to_format_extern_type(extern_type)?;
                    Ok((name.clone(), format_extern))
                })
                .collect();

            Ok(FormatExternType::Component { imports: imports_result?, exports: exports_result? })
        }
    }
}

/// Complete bidirectional conversion from wrt_format::component::ExternType to
/// wrt_foundation::ExternType
///
/// This function handles all ExternType variants comprehensively, fixing
/// previous compatibility issues.
///
/// # Arguments
///
/// * `format_extern_type` - The wrt_format::component::ExternType to convert
///
/// # Returns
///
/// * Result containing the converted wrt_foundation::ExternType or an error
pub fn complete_format_to_types_extern_type(
    format_extern_type: &wrt_format::component::ExternType,
) -> Result<wrt_foundation::ExternType> {
    match format_extern_type {
        FormatExternType::Function { params, results } => {
            // Convert parameter types - create an empty vector and then convert and add
            // each parameter
            let mut param_types = Vec::new();
            for (_, format_val_type) in params {
                // First convert to TypesValType, then to ValueType if needed
                let types_val_type = convert_format_to_types_valtype(format_val_type);
                match convert_format_valtype_to_valuetype(format_val_type) {
                    Ok(value_type) => param_types.push(value_type),
                    Err(_) => {
                        return Err(Error::new(
                            ErrorCategory::Type,
                            codes::CONVERSION_ERROR,
                            NotImplementedError(format!(
                                "Cannot convert {:?} to core ValueType",
                                format_val_type
                            )),
                        ))
                    }
                }
            }

            // Convert result types - create an empty vector and then convert and add each
            // result
            let mut result_types = Vec::new();
            for format_val_type in results {
                // First convert to TypesValType, then to ValueType if needed
                let types_val_type = convert_format_to_types_valtype(format_val_type);
                match convert_format_valtype_to_valuetype(format_val_type) {
                    Ok(value_type) => result_types.push(value_type),
                    Err(_) => {
                        return Err(Error::new(
                            ErrorCategory::Type,
                            codes::CONVERSION_ERROR,
                            NotImplementedError(format!(
                                "Cannot convert {:?} to core ValueType",
                                format_val_type
                            )),
                        ))
                    }
                }
            }

            // Create a new FuncType properly
            Ok(wrt_foundation::ExternType::Function(wrt_foundation::FuncType::new(
                param_types,
                result_types,
            )))
        }
        FormatExternType::Value(format_val_type) => {
            // Value types typically map to globals in the runtime
            // First convert to TypesValType, then to ValueType if needed
            let types_val_type = convert_format_to_types_valtype(format_val_type);
            let value_type = match convert_format_valtype_to_valuetype(format_val_type) {
                Ok(vt) => vt,
                Err(_) => {
                    return Err(Error::new(
                        ErrorCategory::Type,
                        codes::CONVERSION_ERROR,
                        NotImplementedError(format!(
                            "Cannot convert {:?} to core ValueType",
                            format_val_type
                        )),
                    ))
                }
            };
            Ok(wrt_foundation::ExternType::Global(wrt_foundation::GlobalType {
                value_type,
                mutable: false, // Values are typically immutable
            }))
        }
        FormatExternType::Type(type_idx) => {
            // Type references typically map to resources for now
            // In the future, this could be expanded to include more complex type mappings
            Ok(wrt_foundation::ExternType::Resource(wrt_foundation::ResourceType {
                name: "Component not found",
                rep_type: wrt_foundation::ValueType::I32, // Default representation
            }))
        }
        FormatExternType::Instance { exports } => {
            // Convert instance exports
            let export_types: core::result::Result<Vec<(String, wrt_foundation::ExternType)>> = exports
                .iter()
                .map(|(name, extern_type)| {
                    let types_extern = complete_format_to_types_extern_type(extern_type)?;
                    Ok((name.clone(), types_extern))
                })
                .collect();

            Ok(wrt_foundation::ExternType::Instance(wrt_foundation::InstanceType {
                exports: export_types?,
            }))
        }
        FormatExternType::Component { imports, exports } => {
            // Convert component imports
            let import_types: core::result::Result<Vec<(String, String, wrt_foundation::ExternType)>> = imports
                .iter()
                .map(|(namespace, name, extern_type)| {
                    let types_extern = complete_format_to_types_extern_type(extern_type)?;
                    Ok((namespace.clone(), name.clone(), types_extern))
                })
                .collect();

            // Convert component exports
            let export_types: core::result::Result<Vec<(String, wrt_foundation::ExternType)>> = exports
                .iter()
                .map(|(name, extern_type)| {
                    let types_extern = complete_format_to_types_extern_type(extern_type)?;
                    Ok((name.clone(), types_extern))
                })
                .collect();

            Ok(wrt_foundation::ExternType::Component(wrt_foundation::ComponentType {
                imports: import_types?,
                exports: export_types?,
                instances: Vec::new(), // Instances are handled separately in format types
            }))
        }
    }
}
