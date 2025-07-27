use crate::prelude::*;

/// Registry-based type conversions
///
/// This module implements conversions between format and runtime types using
/// the TypeConversionRegistry, providing a consistent and extensible approach
/// to type conversion.

use wrt_format::component::{
    ComponentTypeDefinition, ExternType as FormatExternType, ValType as FormatValType,
};
use wrt_foundation::{
    component::{ComponentType, FuncType, InstanceType},
    component_value::ValType,
    types::ValueType,
    ExternType as TypesExternType,
};

use super::{
    registry::{ConversionError, ConversionErrorKind, TypeConversionRegistry},
    wrappers::{
        FormatComponentType, FormatInstanceType, RuntimeComponentType, RuntimeInstanceType,
    },
};

/// Register ValType conversions in the TypeConversionRegistry
pub fn register_valtype_conversions(registry: &mut TypeConversionRegistry) {
    // Format ValType to Types ValType - primitive types
    registry.register(|format_val_type: &FormatValType| -> core::result::Result<ValType, ConversionError> {
        match format_val_type {
            FormatValType::Bool => Ok(ValType::Bool),
            FormatValType::S8 => Ok(ValType::S8),
            FormatValType::U8 => Ok(ValType::U8),
            FormatValType::S16 => Ok(ValType::S16),
            FormatValType::U16 => Ok(ValType::U16),
            FormatValType::S32 => Ok(ValType::S32),
            FormatValType::U32 => Ok(ValType::U32),
            FormatValType::S64 => Ok(ValType::S64),
            FormatValType::U64 => Ok(ValType::U64),
            FormatValType::F32 => Ok(ValType::F32),
            FormatValType::F64 => Ok(ValType::F64),
            FormatValType::Char => Ok(ValType::Char),
            FormatValType::String => Ok(ValType::String),
            FormatValType::Ref(idx) => Ok(ValType::Ref(*idx)),
            FormatValType::Flags(names) => Ok(ValType::Flags(names.clone())),
            FormatValType::Enum(cases) => Ok(ValType::Enum(cases.clone())),
            FormatValType::Own(idx) => Ok(ValType::Own(*idx)),
            FormatValType::Borrow(idx) => Ok(ValType::Borrow(*idx)),
            // Complex types handled elsewhere or not supported
            _ => Err(ConversionError {
                kind: ConversionErrorKind::NotImplemented,
                source_type: "FormatValType",
                target_type: "ValType",
                context: Some("Complex type conversion requires registry capabilities".to_string()),
                source: None,
            }),
        }
    };

    // Types ValType to Format ValType - primitive types
    registry.register(|types_val_type: &ValType| -> core::result::Result<FormatValType, ConversionError> {
        match types_val_type {
            ValType::Bool => Ok(FormatValType::Bool),
            ValType::S8 => Ok(FormatValType::S8),
            ValType::U8 => Ok(FormatValType::U8),
            ValType::S16 => Ok(FormatValType::S16),
            ValType::U16 => Ok(FormatValType::U16),
            ValType::S32 => Ok(FormatValType::S32),
            ValType::U32 => Ok(FormatValType::U32),
            ValType::S64 => Ok(FormatValType::S64),
            ValType::U64 => Ok(FormatValType::U64),
            ValType::F32 => Ok(FormatValType::F32),
            ValType::F64 => Ok(FormatValType::F64),
            ValType::Char => Ok(FormatValType::Char),
            ValType::String => Ok(FormatValType::String),
            ValType::Ref(idx) => Ok(FormatValType::Ref(*idx)),
            ValType::Flags(names) => Ok(FormatValType::Flags(names.clone())),
            ValType::Enum(cases) => Ok(FormatValType::Enum(cases.clone())),
            ValType::Own(idx) => Ok(FormatValType::Own(*idx)),
            ValType::Borrow(idx) => Ok(FormatValType::Borrow(*idx)),
            // Complex types handled elsewhere or not supported
            _ => Err(ConversionError {
                kind: ConversionErrorKind::NotImplemented,
                source_type: "ValType",
                target_type: "FormatValType",
                context: Some("Complex type conversion requires registry capabilities".to_string()),
                source: None,
            }),
        }
    };

    // ValueType to FormatValType conversion
    registry.register(|value_type: &ValueType| -> core::result::Result<FormatValType, ConversionError> {
        match value_type {
            ValueType::I32 => Ok(FormatValType::S32),
            ValueType::I64 => Ok(FormatValType::S64),
            ValueType::F32 => Ok(FormatValType::F32),
            ValueType::F64 => Ok(FormatValType::F64),
            ValueType::FuncRef | ValueType::ExternRef => Err(ConversionError {
                kind: ConversionErrorKind::InvalidVariant,
                source_type: "ValueType::FuncRef/ExternRef",
                target_type: "FormatValType",
                context: Some(
                    "Reference types cannot be directly converted to component format types"
                        .to_string(),
                ),
                source: None,
            }),
        }
    };

    // FormatValType to ValueType conversion
    registry.register(|format_val_type: &FormatValType| -> core::result::Result<ValueType, ConversionError> {
        match format_val_type {
            FormatValType::S32 => Ok(ValueType::I32),
            FormatValType::S64 => Ok(ValueType::I64),
            FormatValType::F32 => Ok(ValueType::F32),
            FormatValType::F64 => Ok(ValueType::F64),
            _ => Err(ConversionError {
                kind: ConversionErrorKind::InvalidVariant,
                source_type: "FormatValType",
                target_type: "ValueType",
                context: Some("Component not found"),
                source: None,
            }),
        }
    };

    // ValueType to ValType conversion
    registry.register(|value_type: &ValueType| -> core::result::Result<ValType, ConversionError> {
        match value_type {
            ValueType::I32 => Ok(ValType::S32),
            ValueType::I64 => Ok(ValType::S64),
            ValueType::F32 => Ok(ValType::F32),
            ValueType::F64 => Ok(ValType::F64),
            ValueType::FuncRef => Ok(ValType::Own(0)), // Default to resource type 0
            ValueType::ExternRef => Ok(ValType::Ref(0)), // Default to type index 0
        }
    };
}

/// Register ExternType conversions in the TypeConversionRegistry
pub fn register_externtype_conversions(registry: &mut TypeConversionRegistry) {
    // Minimal implementation to support our simple tests
}

/// Register ComponentType and InstanceType conversions in the
/// TypeConversionRegistry
pub fn register_component_instancetype_conversions(registry: &mut TypeConversionRegistry) {
    // Minimal implementation to support our simple tests
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_primitive_valtype_conversions() {
        let mut registry = TypeConversionRegistry::new();
        register_valtype_conversions(&mut registry;

        // Test Format to Types ValType
        let format_val_type = FormatValType::S32;
        let types_val_type = registry.convert::<FormatValType, ValType>(&format_val_type).unwrap();
        assert!(matches!(types_val_type, ValType::S32);

        // Test Types to Format ValType
        let types_val_type = ValType::Bool;
        let format_val_type = registry.convert::<ValType, FormatValType>(&types_val_type).unwrap();
        assert!(matches!(format_val_type, FormatValType::Bool);

        // Test ValueType to FormatValType
        let value_type = ValueType::I32;
        let format_val_type = registry.convert::<ValueType, FormatValType>(&value_type).unwrap();
        assert!(matches!(format_val_type, FormatValType::S32);
    }
}
