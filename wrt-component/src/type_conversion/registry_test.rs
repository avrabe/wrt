#[cfg(test)]
mod tests {
    use crate::type_conversion::registry::*;

    // Simple test types for basic validation
    #[derive(Debug, PartialEq)]
    struct SimpleSource(i32);

    #[derive(Debug, PartialEq)]
    struct SimpleTarget(i32);

    #[test]
    fn test_basic_registry_functionality() {
        // Create a registry
        let mut registry = TypeConversionRegistry::new();

        // Register a simple conversion
        registry.register(|src: &SimpleSource| -> core::result::Result<SimpleTarget, ConversionError> {
            Ok(SimpleTarget(src.0 * 2))
        });

        // Test the conversion
        let source = SimpleSource(21);
        let target = registry.convert::<SimpleSource, SimpleTarget>(&source).unwrap();

        assert_eq!(target, SimpleTarget(42));
    }

    #[test]
    fn test_missing_conversion() {
        let registry = TypeConversionRegistry::new();

        // Try a conversion that doesn't exist
        let source = SimpleSource(42);
        let result = registry.convert::<SimpleSource, SimpleTarget>(&source);

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(matches!(error.kind, ConversionErrorKind::NoConverterFound));
    }

    #[test]
    fn test_can_convert_check() {
        let mut registry = TypeConversionRegistry::new();

        // Initially no conversions are registered
        assert!(!registry.can_convert::<SimpleSource, SimpleTarget>());
        assert!(!registry.can_convert::<SimpleTarget, SimpleSource>());

        // Register one conversion
        registry.register(|src: &SimpleSource| -> core::result::Result<SimpleTarget, ConversionError> {
            Ok(SimpleTarget(src.0))
        });

        // Now one direction should work but not the other
        assert!(registry.can_convert::<SimpleSource, SimpleTarget>());
        assert!(!registry.can_convert::<SimpleTarget, SimpleSource>());
    }

    #[test]
    fn test_conversion_error_handling() {
        let mut registry = TypeConversionRegistry::new();

        // Register a conversion that can fail
        registry.register(|src: &SimpleSource| -> core::result::Result<SimpleTarget, ConversionError> {
            if src.0 < 0 {
                return Err(ConversionError {
                    kind: ConversionErrorKind::OutOfRange,
                    source_type: std::any::type_name::<SimpleSource>(),
                    target_type: std::any::type_name::<SimpleTarget>(),
                    context: Some("Value must be non-negative".to_string()),
                    source: None,
                });
            }
            Ok(SimpleTarget(src.0))
        });

        // Test successful conversion
        let good_source = SimpleSource(42);
        let success_result = registry.convert::<SimpleSource, SimpleTarget>(&good_source);
        assert!(success_result.is_ok());
        assert_eq!(success_result.unwrap(), SimpleTarget(42));

        // Test error case
        let bad_source = SimpleSource(-1);
        let error_result = registry.convert::<SimpleSource, SimpleTarget>(&bad_source);

        assert!(error_result.is_err());
        let error = error_result.unwrap_err();
        assert!(matches!(error.kind, ConversionErrorKind::OutOfRange));
        assert!(error.context.unwrap().contains("non-negative"));
    }
}
