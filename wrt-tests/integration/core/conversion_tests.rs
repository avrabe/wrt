//! Validation test for the conversion architecture design
//! This ensures the design can be implemented and validates the approach

#![cfg(test)]

// In a real implementation, this would be in wrt-component/src/type_conversion/registry.rs
// and other appropriate files, but this is just for validation of the design

use std::{any::{Any, TypeId}, collections::HashMap, sync::Arc};

/// Error kind for conversion errors
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
}

/// Error for conversion operations
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

/// Trait for any convertible type
pub trait Convertible: Any + Sized + Send + Sync {
    fn type_name(&self) -> &'static str;
}

/// Trait for type conversion functions
pub trait Conversion<From, To>: Send + Sync
where
    From: Convertible,
    To: Convertible,
{
    fn convert(&self, from: &From) -> Result<To, ConversionError>;
}

/// Type-erased conversion trait object
trait AnyConversion: Send + Sync {
    fn convert_any(&self, from: &dyn Any) -> Result<Box<dyn Any>, ConversionError>;
    fn source_type_id(&self) -> TypeId;
    fn target_type_id(&self) -> TypeId;
}

/// Implementation of AnyConversion for specific types
struct TypedConversion<From, To, F>
where
    From: Convertible + 'static,
    To: Convertible + 'static,
    F: Fn(&From) -> Result<To, ConversionError> + Send + Sync + 'static,
{
    convert_fn: F,
    _phantom: std::marker::PhantomData<(From, To)>,
}

impl<From, To, F> AnyConversion for TypedConversion<From, To, F>
where
    From: Convertible + 'static,
    To: Convertible + 'static,
    F: Fn(&From) -> Result<To, ConversionError> + Send + Sync + 'static,
{
    fn convert_any(&self, from: &dyn Any) -> Result<Box<dyn Any>, ConversionError> {
        let from = from.downcast_ref::<From>().ok_or_else(|| ConversionError {
            kind: ConversionErrorKind::InvalidArgument,
            source_type: std::any::type_name::<From>(),
            target_type: std::any::type_name::<To>(),
            context: Some("Source value is not of the expected type".to_string()),
            source: None,
        })?;
        
        let result = (self.convert_fn)(from)?;
        Ok(Box::new(result))
    }
    
    fn source_type_id(&self) -> TypeId {
        TypeId::of::<From>()
    }
    
    fn target_type_id(&self) -> TypeId {
        TypeId::of::<To>()
    }
}

/// Central registry for type conversions
pub struct TypeConversionRegistry {
    // Maps source type ID and target type ID to conversion function
    conversions: HashMap<(TypeId, TypeId), Box<dyn AnyConversion>>,
}

impl TypeConversionRegistry {
    /// Creates a new empty registry
    pub fn new() -> Self {
        Self {
            conversions: HashMap::new(),
        }
    }
    
    /// Registers a conversion function from type From to type To
    pub fn register<From, To, F>(&mut self, converter: F) -> &mut Self
    where
        From: Convertible + 'static,
        To: Convertible + 'static,
        F: Fn(&From) -> Result<To, ConversionError> + Send + Sync + 'static,
    {
        let key = (TypeId::of::<From>(), TypeId::of::<To>);
        let conversion = TypedConversion {
            convert_fn: converter,
            _phantom: std::marker::PhantomData,
        };
        
        self.conversions.insert(key, Box::new(conversion;
        self
    }
    
    /// Converts from one type to another
    pub fn convert<From, To>(&self, from: &From) -> Result<To, ConversionError>
    where
        From: Convertible + 'static,
        To: Convertible + 'static,
    {
        let key = (TypeId::of::<From>(), TypeId::of::<To>);
        
        let conversion = self.conversions.get(&key).ok_or_else(|| ConversionError {
            kind: ConversionErrorKind::NotImplemented,
            source_type: std::any::type_name::<From>(),
            target_type: std::any::type_name::<To>(),
            context: Some("No conversion registered for these types".to_string()),
            source: None,
        })?;
        
        let result = conversion.convert_any(from)?;
        let result = result.downcast::<To>().map_err(|_| ConversionError {
            kind: ConversionErrorKind::ConversionFailed,
            source_type: std::any::type_name::<From>(),
            target_type: std::any::type_name::<To>(),
            context: Some("Failed to downcast conversion result".to_string()),
            source: None,
        })?;
        
        Ok(*result)
    }
    
    /// Check if a conversion exists
    pub fn can_convert<From, To>(&self) -> bool
    where
        From: Convertible + 'static,
        To: Convertible + 'static,
    {
        let key = (TypeId::of::<From>(), TypeId::of::<To>);
        self.conversions.contains_key(&key)
    }
}

/// Example types for testing the conversion system
#[derive(Debug, PartialEq, Clone)]
struct FormatValType(String;

#[derive(Debug, PartialEq, Clone)]
struct RuntimeValType(String;

impl Convertible for FormatValType {
    fn type_name(&self) -> &'static str {
        "FormatValType"
    }
}

impl Convertible for RuntimeValType {
    fn type_name(&self) -> &'static str {
        "RuntimeValType"
    }
}

/// ComponentLoader for test validation
pub struct ComponentLoader {
    registry: Arc<TypeConversionRegistry>,
}

impl ComponentLoader {
    pub fn new() -> Self {
        let mut registry = TypeConversionRegistry::new();
        
        // Register default conversions for testing
        registry.register(|format: &FormatValType| -> Result<RuntimeValType, ConversionError> {
            Ok(RuntimeValType(format.0.clone()))
        };
        
        Self {
            registry: Arc::new(registry),
        }
    }
    
    pub fn load_component(&self, format_type: &FormatValType) -> Result<RuntimeValType, ConversionError> {
        self.registry.convert(format_type)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_conversion_registry() {
        let mut registry = TypeConversionRegistry::new();
        
        // Register a conversion from FormatValType to RuntimeValType
        registry.register(|format: &FormatValType| -> Result<RuntimeValType, ConversionError> {
            Ok(RuntimeValType(format.0.clone()))
        };
        
        // Test conversion
        let format_type = FormatValType("i32".to_string());
        let runtime_type: RuntimeValType = registry.convert(&format_type).unwrap();
        
        assert_eq!(runtime_type, RuntimeValType("i32".to_string());
    }
    
    #[test]
    fn test_component_loader() {
        let loader = ComponentLoader::new();
        
        // Test loading a component
        let format_type = FormatValType("i32".to_string());
        let runtime_type = loader.load_component(&format_type).unwrap();
        
        assert_eq!(runtime_type, RuntimeValType("i32".to_string());
    }
    
    #[test]
    fn test_missing_conversion() {
        let registry = TypeConversionRegistry::new();
        
        // Try to convert without registering a conversion
        let format_type = FormatValType("i32".to_string());
        let result = registry.convert::<FormatValType, RuntimeValType>(&format_type;
        
        assert!(result.is_err();
        assert!(matches!(result.unwrap_err().kind, ConversionErrorKind::NotImplemented);
    }
} 