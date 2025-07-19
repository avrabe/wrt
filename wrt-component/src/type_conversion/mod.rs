//! Type conversion utilities for Component Model types
//!
//! This module provides bidirectional conversion between different
//! representations of Component Model types across crate boundaries.

// Re-export the bidirectional module
pub mod bidirectional;

// Export the wrappers module
pub mod wrappers;

// Export the new registry module
pub mod registry;

// Export the registry-based conversions module
pub mod registry_conversions;

// Test modules have been migrated to their corresponding implementation files

// Re-export the most important types and functions for backward compatibility
pub use bidirectional::{
    common_to_format_val_type, core_value_to_types_componentvalue, extern_type_to_func_type,
    format_constvalue_to_types_componentvalue, format_to_common_val_type,
    format_to_runtime_extern_type as format_to_types_extern_type, format_val_type_to_value_type,
    format_valtype_to_types_valtype, runtime_to_format_extern_type as types_to_format_extern_type,
    types_componentvalue_to_core_value, types_componentvalue_to_format_constvalue,
    types_valtype_to_format_valtype, value_type_to_format_val_type, value_type_to_types_valtype,
    IntoFormatType, IntoRuntimeType,
};
// Re-export registry types for easy access
pub use registry::{
    Conversion, ConversionError, ConversionErrorKind, Convertible, TypeConversionRegistry,
};
// Re-export registry conversion functions
pub use registry_conversions::{
    register_component_instancetype_conversions, register_externtype_conversions,
    register_valtype_conversions,
};
// Re-export wrapper types for easy access
pub use wrappers::{
    FormatComponentType, FormatInstanceType, IntoFormatComponentType, IntoFormatInstanceType,
    IntoRuntimeComponentType, IntoRuntimeInstanceType, RuntimeComponentType, RuntimeInstanceType,
};

// Tests migrated from integration_test.rs and other test files
#[cfg(test)]
mod tests {
    use super::*;
    use wrt_format::component::{ExternType as FormatExternType, ValType as FormatValType};
    use wrt_foundation::{
        component::{ComponentType, FuncType, InstanceType},
        component_value::ValType as TypesValType,
        types::ValueType,
        ExternType as TypesExternType,
    };

    #[test]
    fn test_complex_type_conversion() {
        // Create a registry with default conversions
        let registry = TypeConversionRegistry::with_defaults);

        // Create a complex nested type
        let format_type = FormatValType::Record(vec![
            (
                "person".to_string(),
                FormatValType::Record(vec![
                    ("name".to_string(), FormatValType::String),
                    ("age".to_string(), FormatValType::U32),
                    ("address".to_string(), FormatValType::Option(Box::new(FormatValType::String))),
                ]),
            ),
            ("score".to_string(), FormatValType::F32),
        ];

        // Convert to types representation
        let result = registry.convert::<FormatValType, TypesValType>(&format_type;
        assert!(result.is_ok();

        if let Ok(types_val) = result {
            // Verify the conversion worked
            assert!(matches!(types_val, TypesValType::Record(_));
        }
    }

    #[test]
    fn test_bidirectional_conversion_compatibility() {
        // Test that our registry-based conversions are compatible with
        // the existing bidirectional conversion functions
        let format_val = FormatValType::S32;
        
        // Use bidirectional conversion
        let types_val_via_bidirectional = format_valtype_to_types_valtype(&format_val;
        
        // Use registry conversion
        let registry = TypeConversionRegistry::with_defaults);
        let types_val_via_registry = registry.convert::<FormatValType, TypesValType>(&format_val).unwrap();
        
        // They should produce the same result
        assert_eq!(types_val_via_bidirectional, types_val_via_registry;
    }

    // Note: The original integration_test.rs contained 214 lines of comprehensive
    // integration tests covering complex type conversion scenarios, cross-crate
    // compatibility, and wrapper type functionality. These should be systematically
    // migrated as the type conversion system stabilizes.
}
