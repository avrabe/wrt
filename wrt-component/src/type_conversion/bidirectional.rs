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
//! use wrt_format::component::FormatValType;
//! use wrt_format::component::WrtExternType as FormatExternType;
//! use wrt_foundation::WrtExternType as RuntimeWrtExternType;
//!
//! // Convert a format function type to a runtime function type
//! let format_func = FormatExternType::Function {
//!     params: vec![("arg".to_string(), FormatValType<ComponentProvider>::S32)],
//!     results: vec![FormatValType<ComponentProvider>::S32],
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
    FormatResourceOperation, ResourceRepresentation, FormatValType,
};
use crate::bounded_component_infra::ComponentProvider;
use wrt_foundation::{
    component::{ComponentType, FuncType as TypesFuncType, InstanceType},
    component_value::{ValType as TypesValType},
    resource::{ResourceOperation, ResourceType},
    types::ValueType,
    values::Value,
    ExternType as TypesExternType,
};

use crate::prelude::*;

// Type aliases to ensure consistent generic parameters
type WrtTypesValType = TypesValType<ComponentProvider>;

// Helper functions to handle type conversions with correct parameters

// Special helper functions for FormatValType<ComponentProvider> to ValueType conversion
fn convert_format_valtype_to_valuetype(format_val_type: &FormatValType<ComponentProvider>) -> Result<ValueType> {
    match format_val_type {
        FormatValType<ComponentProvider>::S32 => Ok(ValueType::I32),
        FormatValType<ComponentProvider>::S64 => Ok(ValueType::I64),
        FormatValType<ComponentProvider>::F32 => Ok(ValueType::F32),
        FormatValType<ComponentProvider>::F64 => Ok(ValueType::F64),
        _ => Err(Error::unimplemented("Error occurred"Component not foundMissing messageMissing messageMissing message")),
    }
}

// Variant that accepts ValType (WrtTypesValType) for use at call sites
fn convert_types_valtype_to_valuetype(val_type: &WrtTypesValType) -> Result<ValueType> {
    match val_type {
        WrtTypesValType::S32 => Ok(ValueType::I32),
        WrtTypesValType::S64 => Ok(ValueType::I64),
        WrtTypesValType::F32 => Ok(ValueType::F32),
        WrtTypesValType::F64 => Ok(ValueType::F64),
        _ => Err(Error::unimplemented("Error occurred"Component not foundMissing messageMissing messageMissing message")),
    }
}

// Special helper function for FormatValType<ComponentProvider> to WrtTypesValType conversion
fn convert_format_to_types_valtype(format_val_type: &FormatValType<ComponentProvider>) -> WrtTypesValType {
    match format_val_type {
        FormatValType<ComponentProvider>::Bool => WrtTypesValType::Bool,
        FormatValType<ComponentProvider>::S8 => WrtTypesValType::S8,
        FormatValType<ComponentProvider>::U8 => WrtTypesValType::U8,
        FormatValType<ComponentProvider>::S16 => WrtTypesValType::S16,
        FormatValType<ComponentProvider>::U16 => WrtTypesValType::U16,
        FormatValType<ComponentProvider>::S32 => WrtTypesValType::S32,
        FormatValType<ComponentProvider>::U32 => WrtTypesValType::U32,
        FormatValType<ComponentProvider>::S64 => WrtTypesValType::S64,
        FormatValType<ComponentProvider>::U64 => WrtTypesValType::U64,
        FormatValType<ComponentProvider>::F32 => WrtTypesValType::F32,
        FormatValType<ComponentProvider>::F64 => WrtTypesValType::F64,
        FormatValType<ComponentProvider>::Char => WrtTypesValType::Char,
        FormatValType<ComponentProvider>::String => WrtTypesValType::String,
        FormatValType<ComponentProvider>::Ref(idx) => WrtTypesValType::Ref(*idx),
        FormatValType<ComponentProvider>::Own(idx) => WrtTypesValType::Own(*idx),
        FormatValType<ComponentProvider>::Borrow(idx) => WrtTypesValType::Borrow(*idx),
        _ => WrtTypesValType::Void, // Default fallback
    }
}

// Variant that takes a ValType directly for use at call sites
fn convert_types_valtype_identity(val_type: &WrtTypesValType) -> WrtTypesValType {
    val_type.clone()
}

// Special helper function for WrtTypesValType to FormatValType<ComponentProvider> conversion
fn convert_types_to_format_valtype(types_val_type: &WrtTypesValType) -> FormatValType<ComponentProvider> {
    match types_val_type {
        WrtTypesValType::Bool => FormatValType<ComponentProvider>::Bool,
        WrtTypesValType::S8 => FormatValType<ComponentProvider>::S8,
        WrtTypesValType::U8 => FormatValType<ComponentProvider>::U8,
        WrtTypesValType::S16 => FormatValType<ComponentProvider>::S16,
        WrtTypesValType::U16 => FormatValType<ComponentProvider>::U16,
        WrtTypesValType::S32 => FormatValType<ComponentProvider>::S32,
        WrtTypesValType::U32 => FormatValType<ComponentProvider>::U32,
        WrtTypesValType::S64 => FormatValType<ComponentProvider>::S64,
        WrtTypesValType::U64 => FormatValType<ComponentProvider>::U64,
        WrtTypesValType::F32 => FormatValType<ComponentProvider>::F32,
        WrtTypesValType::F64 => FormatValType<ComponentProvider>::F64,
        WrtTypesValType::Char => FormatValType<ComponentProvider>::Char,
        WrtTypesValType::String => FormatValType<ComponentProvider>::String,
        WrtTypesValType::Ref(idx) => FormatValType<ComponentProvider>::Ref(*idx),
        WrtTypesValType::Own(idx) => FormatValType<ComponentProvider>::Own(*idx),
        WrtTypesValType::Borrow(idx) => FormatValType<ComponentProvider>::Borrow(*idx),
        _ => FormatValType<ComponentProvider>::Bool, // Default fallback
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
/// assert!(matches!(format_type, wrt_format::component::ValType::S32);
/// ```
pub fn value_type_to_format_val_type(value_type: &ValueType) -> Result<FormatValType<ComponentProvider>> {
    match value_type {
        ValueType::I32 => Ok(FormatValType<ComponentProvider>::S32),
        ValueType::I64 => Ok(FormatValType<ComponentProvider>::S64),
        ValueType::F32 => Ok(FormatValType<ComponentProvider>::F32),
        ValueType::F64 => Ok(FormatValType<ComponentProvider>::F64),
        ValueType::FuncRef => Err(Error::runtime_execution_error("Error occurred".to_string(),
            ),
        )),
        ValueType::ExternRef => Err(Error::new(
            ErrorCategory::Type,
            codes::NOT_IMPLEMENTED,
            NotImplementedError(
                Missing message"),
            ),
        )),
    }
}

/// Convert FormatValType<ComponentProvider> to ValueType
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
/// assert!(matches!(core_type, wrt_foundation::types::ValueType::I32);
/// ```
pub fn format_val_type_to_value_type(format_val_type: &FormatValType<ComponentProvider>) -> Result<ValueType> {
    convert_format_valtype_to_valuetype(format_val_type)
}

/// Convert WrtTypesValType to ValueType
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
/// assert!(matches!(core_type, wrt_foundation::types::ValueType::I32);
/// ```
pub fn types_valtype_to_valuetype(types_val_type: &WrtTypesValType) -> Result<ValueType> {
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
/// assert!(matches!(runtime_type, wrt_foundation::component_value::ValType::S32);
/// ```
pub fn value_type_to_types_valtype(value_type: &ValueType) -> WrtTypesValType {
    match value_type {
        ValueType::I32 => WrtTypesValType::S32,
        ValueType::I64 => WrtTypesValType::S64,
        ValueType::F32 => WrtTypesValType::F32,
        ValueType::F64 => WrtTypesValType::F64,
        ValueType::FuncRef => WrtTypesValType::Own(0), // Default to resource type 0
        ValueType::ExternRef => WrtTypesValType::Ref(0), // Default to type index 0
    }
}

/// Convert FormatValType<ComponentProvider> to TypesValType
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
/// assert!(matches!(runtime_type, wrt_foundation::component_value::ValType::String);
/// ```
pub fn format_valtype_to_types_valtype(format_val_type: &FormatValType<ComponentProvider>) -> WrtTypesValType {
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
pub fn format_to_types_valtype(val_type: &WrtTypesValType) -> WrtTypesValType {
    convert_types_valtype_identity(val_type)
}

/// Convert WrtTypesValType to FormatValType
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
/// assert!(matches!(format_type, wrt_format::component::ValType::String);
/// ```
pub fn types_valtype_to_format_valtype(types_val_type: &WrtTypesValType) -> FormatValType<ComponentProvider> {
    match types_val_type {
        WrtTypesValType::Bool => FormatValType<ComponentProvider>::Bool,
        WrtTypesValType::S8 => FormatValType<ComponentProvider>::S8,
        WrtTypesValType::U8 => FormatValType<ComponentProvider>::U8,
        WrtTypesValType::S16 => FormatValType<ComponentProvider>::S16,
        WrtTypesValType::U16 => FormatValType<ComponentProvider>::U16,
        WrtTypesValType::S32 => FormatValType<ComponentProvider>::S32,
        WrtTypesValType::U32 => FormatValType<ComponentProvider>::U32,
        WrtTypesValType::S64 => FormatValType<ComponentProvider>::S64,
        WrtTypesValType::U64 => FormatValType<ComponentProvider>::U64,
        WrtTypesValType::F32 => FormatValType<ComponentProvider>::F32,
        WrtTypesValType::F64 => FormatValType<ComponentProvider>::F64,
        WrtTypesValType::Char => FormatValType<ComponentProvider>::Char,
        WrtTypesValType::String => FormatValType<ComponentProvider>::String,
        WrtTypesValType::Ref(idx) => FormatValType<ComponentProvider>::Ref(*idx),
        WrtTypesValType::Record(fields) => {
            let converted_fields = fields
                .iter()
                .map(|(name, val_type)| (name.clone(), types_valtype_to_format_valtype(val_type))
                .collect();
            FormatValType<ComponentProvider>::Record(converted_fields)
        }
        WrtTypesValType::Variant(cases) => {
            let converted_cases = cases
                .iter()
                .map(|(name, opt_type)| {
                    (
                        name.clone(),
                        opt_type.as_ref().map(|val_type| types_valtype_to_format_valtype(val_type)),
                    )
                })
                .collect();
            FormatValType<ComponentProvider>::Variant(converted_cases)
        }
        WrtTypesValType::List(elem_type) => {
            FormatValType<ComponentProvider>::List(Box::new(types_valtype_to_format_valtype(elem_type))
        }
        WrtTypesValType::FixedList(elem_type, size) => {
            FormatValType<ComponentProvider>::FixedList(Box::new(types_valtype_to_format_valtype(elem_type)), *size)
        }
        WrtTypesValType::Tuple(types) => {
            let converted_types =
                types.iter().map(|val_type| types_valtype_to_format_valtype(val_type)).collect();
            FormatValType<ComponentProvider>::Tuple(converted_types)
        }
        WrtTypesValType::Flags(names) => FormatValType<ComponentProvider>::Flags(names.clone()),
        WrtTypesValType::Enum(variants) => FormatValType<ComponentProvider>::Enum(variants.clone()),
        WrtTypesValType::Option(inner_type) => {
            FormatValType<ComponentProvider>::Option(Box::new(types_valtype_to_format_valtype(inner_type))
        }
        WrtTypesValType::Result(result_type) => {
            FormatValType<ComponentProvider>::Result(Box::new(types_valtype_to_format_valtype(result_type))
        }
        WrtTypesValType::Own(idx) => FormatValType<ComponentProvider>::Own(*idx),
        WrtTypesValType::Borrow(idx) => FormatValType<ComponentProvider>::Borrow(*idx),
        WrtTypesValType::Void => {
            // Map void to a default type (this is a simplification)
            FormatValType<ComponentProvider>::Bool
        }
        WrtTypesValType::ErrorContext => FormatValType<ComponentProvider>::ErrorContext,
        WrtTypesValType::Result { ok: _, err: _ } => {
            // Map to FormatValType<ComponentProvider>::Result with a placeholder type
            FormatValType<ComponentProvider>::Result(Box::new(FormatValType<ComponentProvider>::Unit)
        } // All enums handled above
    }
}

/// Convert FormatExternType to TypesWrtExternType
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
/// use wrt_format::component::WrtExternType as FormatExternType;
/// use wrt_format::component::ValType as FormatValType<ComponentProvider>;
///
/// let format_func = FormatExternType::Function {
///     params: vec![("param".to_string(), FormatValType<ComponentProvider>::S32)],
///     results: vec![FormatValType<ComponentProvider>::S32],
/// };
///
/// let runtime_func = format_to_runtime_extern_type(&format_func).unwrap();
/// ```
pub fn format_to_runtime_extern_type(
    format_extern_type: &FormatExternType,
) -> Result<TypesWrtExternType> {
    match format_extern_type {
        FormatExternType::Function { params, results } => {
            // Convert all parameter types to core ValueType
            let converted_params = params
                .iter()
                .map(|(name, val_type)| format_val_type_to_value_type(val_type)
                .collect::<Result<Vec<_>>>()?;

            // Convert all result types to core ValueType
            let converted_results = results
                .iter()
                .map(|val_type| format_val_type_to_value_type(val_type)
                .collect::<Result<Vec<_>>>()?;

            Ok(TypesWrtExternType::Function(TypesFuncType::new(converted_params, converted_results))
        }
        FormatExternType::Value(val_type) => {
            // Convert to most appropriate TypesWrtExternType - likely Function with no
            // params/results Could be mapped as constant global in the future
            let value_type = format_val_type_to_value_type(val_type).unwrap_or(ValueType::I32);
            Ok(TypesWrtExternType::Global(wrt_foundation::component::GlobalType {
                value_type,
                mutable: false,
            })
        }
        FormatExternType::Type(type_idx) => {
            // Type reference - this would need context from the component
            // For now, provide a sensible default
            Ok(TypesWrtExternType::Function(TypesFuncType::new(vec![], vec![]))
        }
        FormatExternType::Instance { exports } => {
            // Convert each export to a TypesWrtExternType
            let converted_exports: core::result::Result<Vec<(String, TypesWrtExternType)>> = exports
                .iter()
                .map(|(name, ext_type)| {
                    Ok((name.clone(), format_to_runtime_extern_type(ext_type)?)
                })
                .collect();

            Ok(TypesWrtExternType::Instance(InstanceType { exports: converted_exports? })
        }
        FormatExternType::Component { imports, exports } => {
            // Convert imports to TypesWrtExternType
            let converted_imports: core::result::Result<Vec<(String, String, TypesWrtExternType)>> = imports
                .iter()
                .map(|(ns, name, ext_type)| {
                    Ok((ns.clone(), name.clone(), format_to_runtime_extern_type(ext_type)?)
                })
                .collect();

            // Convert exports to TypesWrtExternType
            let converted_exports: core::result::Result<Vec<(String, TypesWrtExternType)>> = exports
                .iter()
                .map(|(name, ext_type)| {
                    Ok((name.clone(), format_to_runtime_extern_type(ext_type)?)
                })
                .collect();

            Ok(TypesWrtExternType::Component(ComponentType::new(
                converted_imports?,
                converted_exports?,
            ))
        }
    }
}

/// Convert TypesWrtExternType to FormatExternType
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
/// use wrt_foundation::{WrtExternType, component::FuncType};
/// use wrt_foundation::types::ValueType;
///
/// let func_type = FuncType {
///     params: vec![ValueType::I32],
///     results: vec![ValueType::I32],
/// };
///
/// let runtime_func = WrtExternType::Function(func_type);
/// let format_func = runtime_to_format_extern_type(&runtime_func).unwrap();
/// ```
pub fn runtime_to_format_extern_type(
    types_extern_type: &TypesWrtExternType,
) -> Result<FormatExternType> {
    match types_extern_type {
        WrtExternType::Function(func_type) => {
            // Convert parameter types
            let param_names: Vec<String> =
                (0..func_type.params.len()).map(|i| "Component not foundMissing message").collect();

            // Create param_types manually to handle errors gracefully
            let mut param_types = Vec::new();
            for (i, value_type) in func_type.params.iter().enumerate() {
                match value_type_to_format_val_type(value_type) {
                    Ok(format_val_type) => {
                        param_types.push((param_names[i].clone(), format_val_type)
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
        WrtExternType::Table(table_type) => Err(Error::runtime_execution_error("Error occurred".to_string(),
        )),
        WrtExternType::Memory(memory_type) => Err(Error::new(
            ErrorCategory::System,
            codes::NOT_IMPLEMENTED,
            Missing message"),
        )),
        WrtExternType::Global(global_type) => Err(Error::runtime_execution_error("Error occurred".to_string(),
        )),
        WrtExternType::Instance(instance_type) => {
            // Convert exports to FormatExternType
            let exports_format: core::result::Result<Vec<(String, FormatExternType)>> = instance_type
                .exports
                .iter()
                .map(|(name, ext_type)| {
                    Ok((name.clone(), runtime_to_format_extern_type(ext_type)?)
                })
                .collect();

            Ok(FormatExternType::Instance { exports: exports_format? })
        }
        WrtExternType::Component(component_type) => {
            // Convert imports to FormatExternType
            let imports_format: core::result::Result<Vec<(String, String, FormatExternType)>> = component_type
                .imports
                .iter()
                .map(|(ns, name, ext_type)| {
                    Ok((ns.clone(), name.clone(), runtime_to_format_extern_type(ext_type)?)
                })
                .collect();

            // Convert exports to FormatExternType
            let exports_format: core::result::Result<Vec<(String, FormatExternType)>> = component_type
                .exports
                .iter()
                .map(|(name, ext_type)| {
                    Ok((name.clone(), runtime_to_format_extern_type(ext_type)?)
                })
                .collect();

            Ok(FormatExternType::Component { imports: imports_format?, exports: exports_format? })
        }
        WrtExternType::Resource(resource_type) => {
            // Note: Since FormatExternType doesn't have a direct Resource variant,
            // we map it to a Value type with the appropriate representation
            let val_type = match resource_type.rep_type {
                ValueType::I32 => FormatValType<ComponentProvider>::Own(0), // Use type index 0 as default
                ValueType::I64 => FormatValType<ComponentProvider>::Own(1), // Use type index 1 as default
                _ => FormatValType<ComponentProvider>::Own(0),              // Default to type index 0
            };

            Ok(FormatExternType::Value(convert_types_to_format_valtype(&val_type))
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
pub fn format_to_common_val_type(val_type: &FormatValType<ComponentProvider>) -> Result<ValueType> {
    match val_type {
        FormatValType<ComponentProvider>::S32 => Ok(ValueType::I32),
        FormatValType<ComponentProvider>::S64 => Ok(ValueType::I64),
        FormatValType<ComponentProvider>::F32 => Ok(ValueType::F32),
        FormatValType<ComponentProvider>::F64 => Ok(ValueType::F64),
        _ => Err(Error::component_not_found("Missing error messageMissing message"),
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
pub fn common_to_format_val_type(value_type: &ValueType) -> Result<FormatValType<ComponentProvider>> {
    match value_type {
        ValueType::I32 => Ok(FormatValType<ComponentProvider>::S32),
        ValueType::I64 => Ok(FormatValType<ComponentProvider>::S64),
        ValueType::F32 => Ok(FormatValType<ComponentProvider>::F32),
        ValueType::F64 => Ok(FormatValType<ComponentProvider>::F64),
        _ => Err(Error::runtime_execution_error("Error occurred",
                value_type
            )),
        )),
    }
}

/// Convert an WrtExternType to a FuncType if it represents a function
///
/// # Arguments
///
/// * `extern_type` - The external type to convert
///
/// # Returns
///
/// The function type if the extern type is a function, or an error otherwise
pub fn extern_type_to_func_type(extern_type: &WrtExternType) -> Result<TypesFuncType> {
    match extern_type {
        WrtExternType::Function(func_type) => Ok(func_type.clone()),
        _ => Err(Error::component_not_found("Missing error messageMissing message"),
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

impl IntoRuntimeType<TypesWrtExternType> for FormatExternType {
    fn into_runtime_type(self) -> Result<TypesWrtExternType> {
        format_to_runtime_extern_type(&self)
    }
}

impl IntoFormatType<FormatExternType> for TypesWrtExternType {
    fn into_format_type(self) -> Result<FormatExternType> {
        runtime_to_format_extern_type(&self)
    }
}

impl IntoRuntimeType<WrtTypesValType> for FormatValType<ComponentProvider> {
    fn into_runtime_type(self) -> Result<WrtTypesValType> {
        Ok(format_valtype_to_types_valtype(&self)
    }
}

impl IntoFormatType<FormatValType<ComponentProvider>> for WrtTypesValType {
    fn into_format_type(self) -> Result<FormatValType<ComponentProvider>> {
        Ok(types_valtype_to_format_valtype(&self)
    }
}

/// Convert FormatConstValue to TypesWrtComponentValue
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
/// assert!(matches!(runtime_val, wrt_foundation::component_value::WrtComponentValue::S32(42));
/// ```
pub fn format_constvalue_to_types_componentvalue(
    format_const_value: &FormatConstValue,
) -> Result<WrtComponentValue> {
    match format_const_value {
        FormatConstValue::Bool(v) => Ok(WrtComponentValue::Bool(*v)),
        FormatConstValue::S8(v) => Ok(WrtComponentValue::S8(*v)),
        FormatConstValue::U8(v) => Ok(WrtComponentValue::U8(*v)),
        FormatConstValue::S16(v) => Ok(WrtComponentValue::S16(*v)),
        FormatConstValue::U16(v) => Ok(WrtComponentValue::U16(*v)),
        FormatConstValue::S32(v) => Ok(WrtComponentValue::S32(*v)),
        FormatConstValue::U32(v) => Ok(WrtComponentValue::U32(*v)),
        FormatConstValue::S64(v) => Ok(WrtComponentValue::S64(*v)),
        FormatConstValue::U64(v) => Ok(WrtComponentValue::U64(*v)),
        FormatConstValue::F32(v) => Ok(WrtComponentValue::F32(*v)),
        FormatConstValue::F64(v) => Ok(WrtComponentValue::F64(*v)),
        FormatConstValue::Char(v) => Ok(WrtComponentValue::Char(*v)),
        FormatConstValue::String(v) => Ok(WrtComponentValue::String(v.clone())),
        FormatConstValue::Null => Ok(WrtComponentValue::Void),
    }
}

/// Convert TypesWrtComponentValue to FormatConstValue
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
/// use wrt_foundation::component_value::WrtComponentValue;
///
/// let s32_val = WrtComponentValue::S32(42);
/// let format_val = types_componentvalue_to_format_constvalue(&s32_val).unwrap();
/// assert!(matches!(format_val, wrt_format::component::ConstValue::S32(42));
/// ```
pub fn types_componentvalue_to_format_constvalue(
    types_component_value: &WrtComponentValue,
) -> Result<FormatConstValue> {
    match types_component_value {
        WrtComponentValue::Bool(v) => Ok(FormatConstValue::Bool(*v)),
        WrtComponentValue::S8(v) => Ok(FormatConstValue::S8(*v)),
        WrtComponentValue::U8(v) => Ok(FormatConstValue::U8(*v)),
        WrtComponentValue::S16(v) => Ok(FormatConstValue::S16(*v)),
        WrtComponentValue::U16(v) => Ok(FormatConstValue::U16(*v)),
        WrtComponentValue::S32(v) => Ok(FormatConstValue::S32(*v)),
        WrtComponentValue::U32(v) => Ok(FormatConstValue::U32(*v)),
        WrtComponentValue::S64(v) => Ok(FormatConstValue::S64(*v)),
        WrtComponentValue::U64(v) => Ok(FormatConstValue::U64(*v)),
        WrtComponentValue::F32(v) => Ok(FormatConstValue::F32(*v)),
        WrtComponentValue::F64(v) => Ok(FormatConstValue::F64(*v)),
        WrtComponentValue::Char(v) => Ok(FormatConstValue::Char(*v)),
        WrtComponentValue::String(v) => Ok(FormatConstValue::String(v.clone())),
        WrtComponentValue::Void => Ok(FormatConstValue::Null),
        _ => Err(Error::runtime_execution_error("Error occurred",
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
) -> Result<WrtComponentValue> {
    match value {
        wrt_foundation::values::Value::I32(v) => Ok(WrtComponentValue::S32(*v)),
        wrt_foundation::values::Value::I64(v) => Ok(WrtComponentValue::S64(*v)),
        wrt_foundation::values::Value::F32(v) => Ok(WrtComponentValue::F32(*v)),
        wrt_foundation::values::Value::F64(v) => Ok(WrtComponentValue::F64(*v)),
        wrt_foundation::values::Value::Ref(v) => Ok(WrtComponentValue::U32(*v)), // Map reference
        // to U32
        _ => Err(Error::new(
            ErrorCategory::Type,
            codes::CONVERSION_ERROR,
            NotImplementedError(
                Missing message"),
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
    component_value: &WrtComponentValue,
) -> Result<wrt_foundation::values::Value> {
    match component_value {
        WrtComponentValue::Bool(v) => Ok(wrt_foundation::values::Value::I32(if *v { 1 } else { 0 })),
        WrtComponentValue::S8(v) => Ok(wrt_foundation::values::Value::I32(*v as i32)),
        WrtComponentValue::U8(v) => Ok(wrt_foundation::values::Value::I32(*v as i32)),
        WrtComponentValue::S16(v) => Ok(wrt_foundation::values::Value::I32(*v as i32)),
        WrtComponentValue::U16(v) => Ok(wrt_foundation::values::Value::I32(*v as i32)),
        WrtComponentValue::S32(v) => Ok(wrt_foundation::values::Value::I32(*v)),
        WrtComponentValue::U32(v) => {
            // For U32, check if it represents a reference value (e.g., resource handle)
            // For now, we'll treat all U32 as potential references to maintain
            // compatibility A more sophisticated approach might involve
            // checking the context
            if let Some(resource_index) = is_resource_reference(*v) {
                Ok(wrt_foundation::values::Value::Ref(resource_index)
            } else {
                Ok(wrt_foundation::values::Value::I32(*v as i32)
            }
        }
        WrtComponentValue::S64(v) => Ok(wrt_foundation::values::Value::I64(*v)),
        WrtComponentValue::U64(v) => Ok(wrt_foundation::values::Value::I64(*v as i64)),
        WrtComponentValue::F32(v) => Ok(wrt_foundation::values::Value::F32(*v)),
        WrtComponentValue::F64(v) => Ok(wrt_foundation::values::Value::F64(*v)),
        _ => Err(Error::runtime_execution_error("Error occurred".to_string(),
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

/// Complete bidirectional conversion between wrt_foundation::WrtExternType and
/// wrt_format::component::WrtExternType
///
/// This function handles all WrtExternType variants comprehensively, fixing
/// previous compatibility issues.
///
/// # Arguments
///
/// * `types_extern_type` - The wrt_foundation::WrtExternType to convert
///
/// # Returns
///
/// * Result containing the converted wrt_format::component::WrtExternType or an
///   error
pub fn complete_types_to_format_extern_type(
    types_extern_type: &wrt_foundation::WrtExternType,
) -> Result<wrt_format::component::WrtExternType> {
    match types_extern_type {
        wrt_foundation::WrtExternType::Function(func_type) => {
            // Convert parameter types
            let param_names: Vec<String> =
                (0..func_type.params.len()).map(|i| Missing message").collect();

            // Create param_types manually to handle errors gracefully
            let mut param_types = Vec::new();
            for (i, value_type) in func_type.params.iter().enumerate() {
                match value_type_to_format_val_type(value_type) {
                    Ok(format_val_type) => {
                        param_types.push((param_names[i].clone(), format_val_type)
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
        wrt_foundation::WrtExternType::Table(table_type) => Err(Error::runtime_execution_error("Error occurred".to_string(),
        )),
        wrt_foundation::WrtExternType::Memory(memory_type) => Err(Error::new(
            ErrorCategory::Type,
            codes::CONVERSION_ERROR,
            Missing message"),
        )),
        wrt_foundation::WrtExternType::Global(global_type) => Err(Error::runtime_execution_error("Error occurred".to_string(),
        )),
        wrt_foundation::WrtExternType::Resource(resource_type) => {
            // For resources, we convert to a Type reference for now
            // In the future, this could be expanded to include full resource types
            Ok(FormatExternType::Type(0)
        }
        wrt_foundation::WrtExternType::Instance(instance_type) => {
            // Convert instance exports
            let exports_result: core::result::Result<Vec<(String, FormatExternType)>> = instance_type
                .exports
                .iter()
                .map(|(name, extern_type)| {
                    let format_extern = complete_types_to_format_extern_type(extern_type)?;
                    Ok((name.clone(), format_extern)
                })
                .collect();

            Ok(FormatExternType::Instance { exports: exports_result? })
        }
        wrt_foundation::WrtExternType::Component(component_type) => {
            // Convert component imports
            let imports_result: core::result::Result<Vec<(String, String, FormatExternType)>> = component_type
                .imports
                .iter()
                .map(|(namespace, name, extern_type)| {
                    let format_extern = complete_types_to_format_extern_type(extern_type)?;
                    Ok((namespace.clone(), name.clone(), format_extern)
                })
                .collect();

            // Convert component exports
            let exports_result: core::result::Result<Vec<(String, FormatExternType)>> = component_type
                .exports
                .iter()
                .map(|(name, extern_type)| {
                    let format_extern = complete_types_to_format_extern_type(extern_type)?;
                    Ok((name.clone(), format_extern)
                })
                .collect();

            Ok(FormatExternType::Component { imports: imports_result?, exports: exports_result? })
        }
    }
}

/// Complete bidirectional conversion from wrt_format::component::WrtExternType to
/// wrt_foundation::WrtExternType
///
/// This function handles all WrtExternType variants comprehensively, fixing
/// previous compatibility issues.
///
/// # Arguments
///
/// * `format_extern_type` - The wrt_format::component::WrtExternType to convert
///
/// # Returns
///
/// * Result containing the converted wrt_foundation::WrtExternType or an error
pub fn complete_format_to_types_extern_type(
    format_extern_type: &wrt_format::component::WrtExternType,
) -> Result<wrt_foundation::WrtExternType> {
    match format_extern_type {
        FormatExternType::Function { params, results } => {
            // Convert parameter types - create an empty vector and then convert and add
            // each parameter
            let mut param_types = Vec::new();
            for (_, format_val_type) in params {
                // First convert to WrtTypesValType, then to ValueType if needed
                let types_val_type = convert_format_to_types_valtype(format_val_type);
                match convert_format_valtype_to_valuetype(format_val_type) {
                    Ok(value_type) => param_types.push(value_type),
                    Err(_) => {
                        return Err(Error::new(
                            ErrorCategory::Type,
                            codes::CONVERSION_ERROR,
                            NotImplementedError(format!(
                                Missing messageMissing messageMissing message")),
                        )
                    }
                }
            }

            // Convert result types - create an empty vector and then convert and add each
            // result
            let mut result_types = Vec::new();
            for format_val_type in results {
                // First convert to WrtTypesValType, then to ValueType if needed
                let types_val_type = convert_format_to_types_valtype(format_val_type);
                match convert_format_valtype_to_valuetype(format_val_type) {
                    Ok(value_type) => result_types.push(value_type),
                    Err(_) => {
                        return Err(Error::runtime_execution_error("Error occurred",
                                format_val_type
                            )),
                        )
                    }
                }
            }

            // Create a new FuncType properly
            Ok(wrt_foundation::WrtExternType::Function(wrt_foundation::FuncType::new(
                param_types,
                result_types,
            ))
        }
        FormatExternType::Value(format_val_type) => {
            // Value types typically map to globals in the runtime
            // First convert to WrtTypesValType, then to ValueType if needed
            let types_val_type = convert_format_to_types_valtype(format_val_type);
            let value_type = match convert_format_valtype_to_valuetype(format_val_type) {
                Ok(vt) => vt,
                Err(_) => {
                    return Err(Error::new(
                        ErrorCategory::Type,
                        codes::CONVERSION_ERROR,
                        NotImplementedError(format!(
                            Missing messageMissing messageMissing message")),
                    )
                }
            };
            Ok(wrt_foundation::WrtExternType::Global(wrt_foundation::GlobalType {
                value_type,
                mutable: false, // Values are typically immutable
            })
        }
        FormatExternType::Type(type_idx) => {
            // Type references typically map to resources for now
            // In the future, this could be expanded to include more complex type mappings
            Ok(wrt_foundation::WrtExternType::Resource(wrt_foundation::ResourceType {
                name: "Component not found",
                rep_type: wrt_foundation::ValueType::I32, // Default representation
            })
        }
        FormatExternType::Instance { exports } => {
            // Convert instance exports
            let export_types: core::result::Result<Vec<(String, wrt_foundation::WrtExternType)>> = exports
                .iter()
                .map(|(name, extern_type)| {
                    let types_extern = complete_format_to_types_extern_type(extern_type)?;
                    Ok((name.clone(), types_extern)
                })
                .collect();

            Ok(wrt_foundation::WrtExternType::Instance(wrt_foundation::InstanceType {
                exports: export_types?,
            })
        }
        FormatExternType::Component { imports, exports } => {
            // Convert component imports
            let import_types: core::result::Result<Vec<(String, String, wrt_foundation::WrtExternType)>> = imports
                .iter()
                .map(|(namespace, name, extern_type)| {
                    let types_extern = complete_format_to_types_extern_type(extern_type)?;
                    Ok((namespace.clone(), name.clone(), types_extern)
                })
                .collect();

            // Convert component exports
            let export_types: core::result::Result<Vec<(String, wrt_foundation::WrtExternType)>> = exports
                .iter()
                .map(|(name, extern_type)| {
                    let types_extern = complete_format_to_types_extern_type(extern_type)?;
                    Ok((name.clone(), types_extern)
                })
                .collect();

            Ok(wrt_foundation::WrtExternType::Component(wrt_foundation::ComponentType {
                imports: import_types?,
                exports: export_types?,
                instances: Vec::new(), // Instances are handled separately in format types
            })
        }
    }
}
