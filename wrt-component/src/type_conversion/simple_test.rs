#[cfg(test)]
mod simple_tests {
    use wrt_format::component::ValType as FormatValType;
    use wrt_foundation::component_value::ValType as TypesValType;

    use super::super::registry::TypeConversionRegistry;

    #[test]
    fn test_simple_format_to_types_conversion() {
        let registry = TypeConversionRegistry::with_defaults();

        // Test primitive types
        let bool_type = FormatValType::Bool;
        let result = registry.convert::<FormatValType, TypesValType(&bool_type).unwrap();
        assert!(matches!(result, TypesValType::Bool));

        let s32_type = FormatValType::S32;
        let result = registry.convert::<FormatValType, TypesValType(&s32_type).unwrap();
        assert!(matches!(result, TypesValType::S32));
    }

    #[test]
    fn test_simple_types_to_format_conversion() {
        let registry = TypeConversionRegistry::with_defaults();

        // Test primitive types
        let bool_type = TypesValType::Bool;
        let result = registry.convert::<TypesValType, FormatValType(&bool_type).unwrap();
        assert!(matches!(result, FormatValType::Bool));

        let s32_type = TypesValType::S32;
        let result = registry.convert::<TypesValType, FormatValType(&s32_type).unwrap();
        assert!(matches!(result, FormatValType::S32));
    }
}
