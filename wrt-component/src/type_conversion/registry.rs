/// Type Conversion Registry
///
/// This module implements a central registry for type conversions between
/// different representations across the WebAssembly Runtime.

#[cfg(feature = "std")]
use std::{
    any::{Any, TypeId},
    boxed::Box,
    collections::HashMap,
    fmt,
    marker::PhantomData,
    sync::Arc,
};

#[cfg(not(feature = "std"))]
use alloc::{
    boxed::Box,
    collections::BTreeMap as HashMap,
    sync::Arc,
};
#[cfg(not(feature = "std"))]
use core::{
    any::{Any, TypeId},
    fmt,
    marker::PhantomData,
};

/// Error type for conversion operations
#[derive(Debug, Clone)]
pub struct ConversionError {
    /// The specific kind of conversion error
    pub kind: ConversionErrorKind,
    /// Source type being converted from
    pub source_type: &'static str,
    /// Target type being converted to
    pub target_type: &'static str,
    /// Additional context information
    pub context: Option<String>,
    /// Source error (for chaining)
    pub source: Option<Box<ConversionError>>,
}

impl fmt::Display for ConversionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Failed to convert from {} to {}: {:?}",
            self.source_type, self.target_type, self.kind
        )?;

        if let Some(context) = &self.context {
            write!(f, " - {}", context)?;
        }

        if let Some(source) = &self.source {
            write!(f, "\nCaused by: {}", source)?;
        }

        Ok(())
    }
}

#[cfg(feature = "std")]
impl std::error::Error for ConversionError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.source.as_ref().map(|e| e.as_ref() as &(dyn std::error::Error + 'static))
    }
}

/// Specific kinds of conversion errors
#[derive(Debug, Clone)]
pub enum ConversionErrorKind {
    /// Type conversion not implemented
    NotImplemented,
    /// Invalid arguments provided
    InvalidArgument,
    /// Invalid type variant encountered
    InvalidVariant,
    /// Value out of range for target type
    OutOfRange,
    /// Unexpected null value
    UnexpectedNull,
    /// General conversion failure
    ConversionFailed,
    /// No registered converter found
    NoConverterFound,
}

/// Trait for any convertible type
pub trait Convertible: Any + Sized + Send + Sync {
    fn type_name(&self) -> &'static str;
}

impl<T: Any + Sized + Send + Sync> Convertible for T {
    fn type_name(&self) -> &'static str {
        std::any::type_name::<T>()
    }
}

/// Trait for type conversion functions
pub trait Conversion<From, To>: Send + Sync
where
    From: Convertible,
    To: Convertible,
{
    fn convert(&self, from: &From) -> core::result::Result<To, ConversionError>;
}

/// Implementation for function-based converters
impl<From, To, F> Conversion<From, To> for F
where
    From: Convertible,
    To: Convertible,
    F: Fn(&From) -> core::result::Result<To, ConversionError> + Send + Sync,
{
    fn convert(&self, from: &From) -> core::result::Result<To, ConversionError> {
        self(from)
    }
}

/// Type-erased conversion trait object
trait AnyConversion: Send + Sync {
    fn convert_any(&self, from: &dyn Any) -> core::result::Result<Box<dyn Any>, ConversionError>;
    fn source_type_id(&self) -> TypeId;
    fn target_type_id(&self) -> TypeId;
    fn source_type_name(&self) -> &'static str;
    fn target_type_name(&self) -> &'static str;
}

/// Adapter to implement AnyConversion for specific converters
struct ConversionAdapter<From, To, C>
where
    From: Convertible + 'static,
    To: Convertible + 'static,
    C: Conversion<From, To> + 'static,
{
    converter: C,
    source_type_name: &'static str,
    target_type_name: &'static str,
    _phantom_from: PhantomData<From>,
    _phantom_to: PhantomData<To>,
}

impl<From, To, C> AnyConversion for ConversionAdapter<From, To, C>
where
    From: Convertible + 'static,
    To: Convertible + 'static,
    C: Conversion<From, To> + 'static,
{
    fn convert_any(&self, from: &dyn Any) -> core::result::Result<Box<dyn Any>, ConversionError> {
        // Try to downcast to the expected input type
        let from = from.downcast_ref::<From>().ok_or_else(|| ConversionError {
            kind: ConversionErrorKind::InvalidArgument,
            source_type: self.source_type_name,
            target_type: self.target_type_name,
            context: Some("Source value doesn't match expected type".to_string()),
            source: None,
        })?;

        // Perform the conversion
        let result = self.converter.convert(from)?;

        // Box the result as Any
        Ok(Box::new(result))
    }

    fn source_type_id(&self) -> TypeId {
        TypeId::of::<From>()
    }

    fn target_type_id(&self) -> TypeId {
        TypeId::of::<To>()
    }

    fn source_type_name(&self) -> &'static str {
        self.source_type_name
    }

    fn target_type_name(&self) -> &'static str {
        self.target_type_name
    }
}

/// Central registry for type conversions
pub struct TypeConversionRegistry {
    // Maps source type ID and target type ID to conversion function
    conversions: HashMap<(TypeId, TypeId), Box<dyn AnyConversion>>,

    // Feature flags status
    #[cfg(feature = "std")]
    std_enabled: bool,

    #[cfg(not(feature = "std"))]
    alloc_enabled: bool,
}

impl TypeConversionRegistry {
    /// Create a new, empty type conversion registry
    #[cfg(feature = "std")]
    pub fn new() -> Self {
        Self { conversions: HashMap::new(), std_enabled: true }
    }

    /// Create a new, empty type conversion registry (no_std version)
    #[cfg(not(feature = "std"))]
    pub fn new() -> Self {
        Self { conversions: HashMap::new(), alloc_enabled: true }
    }

    /// Register a conversion function from type From to type To
    pub fn register<From, To, F>(&mut self, converter: F) -> &mut Self
    where
        From: Convertible + 'static,
        To: Convertible + 'static,
        F: Fn(&From) -> core::result::Result<To, ConversionError> + Send + Sync + 'static,
    {
        let adapter = ConversionAdapter {
            converter,
            source_type_name: std::any::type_name::<From>(),
            target_type_name: std::any::type_name::<To>(),
            _phantom_from: PhantomData,
            _phantom_to: PhantomData,
        };

        let key = (TypeId::of::<From>(), TypeId::of::<To>);
        self.conversions.insert(key, Box::new(adapter;
        self
    }

    /// Check if a conversion exists between the given types
    pub fn can_convert<From, To>(&self) -> bool
    where
        From: Convertible + 'static,
        To: Convertible + 'static,
    {
        let key = (TypeId::of::<From>(), TypeId::of::<To>);
        self.conversions.contains_key(&key)
    }

    /// Convert from one type to another
    pub fn convert<From, To>(&self, from: &From) -> core::result::Result<To, ConversionError>
    where
        From: Convertible + 'static,
        To: Convertible + 'static,
    {
        let key = (TypeId::of::<From>(), TypeId::of::<To>);

        // Look up the converter in the registry
        let converter = self.conversions.get(&key).ok_or_else(|| ConversionError {
            kind: ConversionErrorKind::NoConverterFound,
            source_type: std::any::type_name::<From>(),
            target_type: std::any::type_name::<To>(),
            context: Some("No converter registered for this type pair".to_string()),
            source: None,
        })?;

        // Perform the conversion
        let result = converter.convert_any(from)?;

        // Downcast the result to the expected output type
        let result = result.downcast::<To>().map_err(|_| ConversionError {
            kind: ConversionErrorKind::ConversionFailed,
            source_type: std::any::type_name::<From>(),
            target_type: std::any::type_name::<To>(),
            context: Some("Failed to downcast conversion result".to_string()),
            source: None,
        })?;

        Ok(*result)
    }

    /// Create a registry populated with default conversions
    pub fn with_defaults() -> Self {
        let mut registry = Self::new);
        registry.register_defaults);
        registry
    }

    /// Register all default conversions
    pub fn register_defaults(&mut self) -> &mut Self {
        use super::registry_conversions::{
            register_component_instancetype_conversions, register_externtype_conversions,
            register_valtype_conversions,
        };

        register_valtype_conversions(self;
        register_externtype_conversions(self;
        register_component_instancetype_conversions(self;

        self
    }

    /// Create a new empty registry with the same configuration
    pub fn new_empty(&self) -> Self {
        #[cfg(feature = "std")]
        {
            Self { conversions: HashMap::new(), std_enabled: self.std_enabled }
        }

        #[cfg(not(feature = "std"))]
        {
            Self { conversions: HashMap::new(), alloc_enabled: self.alloc_enabled }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Simple test types
    #[derive(Debug, PartialEq)]
    struct TestSourceType(i32;

    #[derive(Debug, PartialEq)]
    struct TestTargetType(i32;

    #[test]
    fn test_registry_basic_conversion() {
        // Create a registry
        let mut registry = TypeConversionRegistry::new);

        // Register a simple conversion function
        registry.register(|src: &TestSourceType| -> core::result::Result<TestTargetType, ConversionError> {
            Ok(TestTargetType(src.0 * 2))
        };

        // Test the conversion
        let source = TestSourceType(21;
        let target = registry.convert::<TestSourceType, TestTargetType>(&source).unwrap();

        assert_eq!(target, TestTargetType(42;
    }

    #[test]
    fn test_registry_missing_conversion() {
        #[derive(Debug)]
        struct AnotherType(f64;

        let registry = TypeConversionRegistry::new);

        // Try a conversion that doesn't exist
        let source = TestSourceType(42;
        let result = registry.convert::<TestSourceType, AnotherType>(&source;

        assert!(result.is_err();
        if let Err(err) = result {
            assert!(matches!(err.kind, ConversionErrorKind::NoConverterFound);
        }
    }

    #[test]
    fn test_can_convert() {
        let mut registry = TypeConversionRegistry::new);

        // Register a conversion
        registry.register(|src: &TestSourceType| -> core::result::Result<TestTargetType, ConversionError> {
            Ok(TestTargetType(src.0))
        };

        // Check conversions
        assert!(registry.can_convert::<TestSourceType, TestTargetType>();
        assert!(!registry.can_convert::<TestTargetType, TestSourceType>();
    }

    #[test]
    fn test_conversion_error_handling() {
        let mut registry = TypeConversionRegistry::new);

        // Register a conversion that may fail
        registry.register(|src: &TestSourceType| -> core::result::Result<TestTargetType, ConversionError> {
            if src.0 < 0 {
                return Err(ConversionError {
                    kind: ConversionErrorKind::OutOfRange,
                    source_type: std::any::type_name::<TestSourceType>(),
                    target_type: std::any::type_name::<TestTargetType>(),
                    context: Some("Value must be non-negative".to_string()),
                    source: None,
                };
            }
            Ok(TestTargetType(src.0))
        };

        // Test successful conversion
        let good_source = TestSourceType(42;
        assert!(registry.convert::<TestSourceType, TestTargetType>(&good_source).is_ok();

        // Test failed conversion
        let bad_source = TestSourceType(-1;
        let result = registry.convert::<TestSourceType, TestTargetType>(&bad_source;

        assert!(result.is_err();
        if let Err(err) = result {
            assert!(matches!(err.kind, ConversionErrorKind::OutOfRange);
            assert!(err.context.unwrap().contains("must be non-negative");
        }
    }

    // ====== TESTS MIGRATED FROM SIMPLE_TEST.RS ======

    #[test]
    fn test_simple_format_to_types_conversion() {
        use wrt_format::component::ValType as FormatValType;
        use wrt_foundation::component_value::ValType as TypesValType;

        let registry = TypeConversionRegistry::with_defaults);

        // Test primitive types
        let bool_type = FormatValType::Bool;
        let result = registry.convert::<FormatValType, TypesValType>(&bool_type).unwrap();
        assert!(matches!(result, TypesValType::Bool);

        let s32_type = FormatValType::S32;
        let result = registry.convert::<FormatValType, TypesValType>(&s32_type).unwrap();
        assert!(matches!(result, TypesValType::S32);
    }

    #[test]
    fn test_simple_types_to_format_conversion() {
        use wrt_format::component::ValType as FormatValType;
        use wrt_foundation::component_value::ValType as TypesValType;

        let registry = TypeConversionRegistry::with_defaults);

        // Test primitive types
        let bool_type = TypesValType::Bool;
        let result = registry.convert::<TypesValType, FormatValType>(&bool_type).unwrap();
        assert!(matches!(result, FormatValType::Bool);

        let s32_type = TypesValType::S32;
        let result = registry.convert::<TypesValType, FormatValType>(&s32_type).unwrap();
        assert!(matches!(result, FormatValType::S32);
    }

    // ====== TESTS MIGRATED FROM REGISTRY_TEST.RS ======

    // Additional test types for registry testing
    #[derive(Debug, PartialEq)]
    struct SimpleSource(i32;

    #[derive(Debug, PartialEq)]
    struct SimpleTarget(i32;

    #[test]
    fn test_basic_registry_functionality() {
        // Create a registry
        let mut registry = TypeConversionRegistry::new);

        // Register a simple conversion
        registry.register(|src: &SimpleSource| -> core::result::Result<SimpleTarget, ConversionError> {
            Ok(SimpleTarget(src.0 * 2))
        };

        // Test the conversion
        let source = SimpleSource(21;
        let result = registry.convert::<SimpleSource, SimpleTarget>(&source).unwrap();
        assert_eq!(result, SimpleTarget(42;
    }
}
