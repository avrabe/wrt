#[cfg(test)]
mod tests {
    use std::{
        any::{Any, TypeId},
        boxed::Box,
        collections::HashMap,
        fmt,
        marker::PhantomData,
        sync::Arc,
    };

    #[derive(Debug, Clone)]
    struct TestConversionError {
        message: String,
    }

    impl fmt::Display for TestConversionError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "Conversion error: {}", self.message)
        }
    }

    trait TestConvertible: Any + Sized + Send + Sync {
        fn test_type_name(&self) -> &'static str;
    }

    impl<T: Any + Sized + Send + Sync> TestConvertible for T {
        fn test_type_name(&self) -> &'static str {
            std::any::type_name::<T>()
        }
    }

    trait TestConversion<From, To>: Send + Sync
    where
        From: TestConvertible,
        To: TestConvertible,
    {
        fn convert(&self, from: &From) -> Result<To, TestConversionError>;
    }

    impl<From, To, F> TestConversion<From, To> for F
    where
        From: TestConvertible,
        To: TestConvertible,
        F: Fn(&From) -> Result<To, TestConversionError> + Send + Sync,
    {
        fn convert(&self, from: &From) -> Result<To, TestConversionError> {
            self(from)
        }
    }

    trait TestAnyConversion: Send + Sync {
        fn convert_any(&self, from: &dyn Any) -> Result<Box<dyn Any>, TestConversionError>;
        fn source_type_id(&self) -> TypeId;
        fn target_type_id(&self) -> TypeId;
    }

    struct TestConversionAdapter<From, To, C>
    where
        From: TestConvertible + 'static,
        To: TestConvertible + 'static,
        C: TestConversion<From, To> + 'static,
    {
        converter: C,
        _phantom_from: PhantomData<From>,
        _phantom_to: PhantomData<To>,
    }

    impl<From, To, C> TestAnyConversion for TestConversionAdapter<From, To, C>
    where
        From: TestConvertible + 'static,
        To: TestConvertible + 'static,
        C: TestConversion<From, To> + 'static,
    {
        fn convert_any(&self, from: &dyn Any) -> Result<Box<dyn Any>, TestConversionError> {
            let from = from.downcast_ref::<From>().ok_or_else(|| TestConversionError {
                message: "Source value doesn't match expected type".to_string(),
            })?;

            let result = self.converter.convert(from)?;
            Ok(Box::new(result))
        }

        fn source_type_id(&self) -> TypeId {
            TypeId::of::<From>()
        }

        fn target_type_id(&self) -> TypeId {
            TypeId::of::<To>()
        }
    }

    struct TestRegistry {
        conversions: HashMap<(TypeId, TypeId), Box<dyn TestAnyConversion>>,
    }

    impl TestRegistry {
        fn new() -> Self {
            Self { conversions: HashMap::new() }
        }

        fn register<From, To, F>(&mut self, converter: F) -> &mut Self
        where
            From: TestConvertible + 'static,
            To: TestConvertible + 'static,
            F: Fn(&From) -> Result<To, TestConversionError> + Send + Sync + 'static,
        {
            let adapter = TestConversionAdapter {
                converter,
                _phantom_from: PhantomData,
                _phantom_to: PhantomData,
            };

            let key = (TypeId::of::<From>(), TypeId::of::<To>());
            self.conversions.insert(key, Box::new(adapter));
            self
        }

        fn can_convert<From, To>(&self) -> bool
        where
            From: TestConvertible + 'static,
            To: TestConvertible + 'static,
        {
            let key = (TypeId::of::<From>(), TypeId::of::<To>());
            self.conversions.contains_key(&key)
        }

        fn convert<From, To>(&self, from: &From) -> Result<To, TestConversionError>
        where
            From: TestConvertible + 'static,
            To: TestConvertible + 'static,
        {
            let key = (TypeId::of::<From>(), TypeId::of::<To>());

            let converter = self.conversions.get(&key).ok_or_else(|| TestConversionError {
                message: "No converter registered for this type pair".to_string(),
            })?;

            let result = converter.convert_any(from)?;

            let result = result.downcast::<To>().map_err(|_| TestConversionError {
                message: "Failed to downcast conversion result".to_string(),
            })?;

            Ok(*result)
        }
    }

    // Test types
    #[derive(Debug, PartialEq)]
    struct TestSource(i32);

    #[derive(Debug, PartialEq)]
    struct TestTarget(i32);

    #[test]
    fn test_registry_functionality() {
        // Create a registry
        let mut registry = TestRegistry::new();

        // Register a simple conversion
        registry.register(|src: &TestSource| -> Result<TestTarget, TestConversionError> {
            Ok(TestTarget(src.0 * 2))
        });

        // Test the conversion
        let source = TestSource(21);
        let target = registry.convert::<TestSource, TestTarget>(&source).unwrap();

        assert_eq!(target, TestTarget(42));
    }

    #[test]
    fn test_missing_conversion() {
        let registry = TestRegistry::new();

        // Try a conversion that doesn't exist
        let source = TestSource(42);
        let result = registry.convert::<TestSource, TestTarget>(&source);

        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("No converter registered"));
    }

    #[test]
    fn test_can_convert_check() {
        let mut registry = TestRegistry::new();

        // Initially no conversions are registered
        assert!(!registry.can_convert::<TestSource, TestTarget>());
        assert!(!registry.can_convert::<TestTarget, TestSource>());

        // Register one conversion
        registry.register(|src: &TestSource| -> Result<TestTarget, TestConversionError> {
            Ok(TestTarget(src.0))
        });

        // Now one direction should work but not the other
        assert!(registry.can_convert::<TestSource, TestTarget>());
        assert!(!registry.can_convert::<TestTarget, TestSource>());
    }
}
