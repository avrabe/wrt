//! Component Model support for no_std environments
//!
//! This module provides basic Component Model functionality using bounded
//! collections, enabling component model usage in pure no_std environments
//! without allocation.

use wrt_foundation::{BoundedVec, BoundedString, MemoryProvider, NoStdProvider};
use crate::{
    MAX_COMPONENT_TYPES, MAX_COMPONENT_IMPORTS, MAX_COMPONENT_EXPORTS,
    MAX_WASM_STRING_SIZE, MAX_STATIC_TYPES
};

/// Component name in no_std environment
pub type ComponentName<P> = BoundedString<MAX_WASM_STRING_SIZE, P>;

/// Bounded component type for no_std environments
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BoundedComponentType<P: MemoryProvider + Default + Clone + PartialEq + Eq> {
    /// Core module type
    CoreModule,
    /// Core function type with bounded parameters
    CoreFunc {
        params: BoundedVec<CoreValueType, 16, P>,
        results: BoundedVec<CoreValueType, 16, P>,
    },
    /// Component function type
    ComponentFunc {
        params: BoundedVec<ComponentValueType<P>, 16, P>,
        results: BoundedVec<ComponentValueType<P>, 16, P>,
    },
    /// Component instance type
    Instance {
        exports: BoundedVec<(ComponentName<P>, BoundedComponentType<P>), 32, P>,
    },
    /// Component type
    Component {
        imports: BoundedVec<(ComponentName<P>, BoundedComponentType<P>), 32, P>,
        exports: BoundedVec<(ComponentName<P>, BoundedComponentType<P>), 32, P>,
    },
}

/// Core WebAssembly value types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoreValueType {
    I32,
    I64,
    F32,
    F64,
    V128,
    FuncRef,
    ExternRef,
}

/// Component Model value types for no_std
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ComponentValueType<P: MemoryProvider + Default + Clone + PartialEq + Eq> {
    /// Primitive types
    Bool,
    S8, U8, S16, U16, S32, U32, S64, U64,
    Float32, Float64,
    Char,
    String,
    
    /// Composite types with bounded collections
    List(Box<ComponentValueType<P>>),
    Record(BoundedVec<(ComponentName<P>, ComponentValueType<P>), 16, P>),
    Variant(BoundedVec<(ComponentName<P>, Option<ComponentValueType<P>>), 16, P>),
    Tuple(BoundedVec<ComponentValueType<P>, 16, P>),
    Flags(BoundedVec<ComponentName<P>, 32, P>),
    Enum(BoundedVec<ComponentName<P>, 32, P>),
    Option(Box<ComponentValueType<P>>),
    Result {
        ok: Option<Box<ComponentValueType<P>>>,
        err: Option<Box<ComponentValueType<P>>>,
    },
    
    /// Resource types
    Own(u32),
    Borrow(u32),
}

/// Import declaration for components
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoundedImport<P: MemoryProvider + Default + Clone + PartialEq + Eq> {
    /// Import name
    pub name: ComponentName<P>,
    /// Import type
    pub ty: BoundedComponentType<P>,
}

/// Export declaration for components
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoundedExport<P: MemoryProvider + Default + Clone + PartialEq + Eq> {
    /// Export name
    pub name: ComponentName<P>,
    /// Export type
    pub ty: BoundedComponentType<P>,
}

/// Bounded component for no_std environments
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoundedComponent<P: MemoryProvider + Default + Clone + PartialEq + Eq = NoStdProvider<4096>> {
    /// Component types
    pub types: BoundedVec<BoundedComponentType<P>, MAX_COMPONENT_TYPES, P>,
    /// Component imports
    pub imports: BoundedVec<BoundedImport<P>, MAX_COMPONENT_IMPORTS, P>,
    /// Component exports
    pub exports: BoundedVec<BoundedExport<P>, MAX_COMPONENT_EXPORTS, P>,
    /// Memory provider
    provider: P,
}

impl<P: MemoryProvider + Default + Clone + PartialEq + Eq> BoundedComponent<P> {
    /// Create a new bounded component
    pub fn new(provider: P) -> Result<Self, wrt_foundation::bounded::CapacityError> {
        Ok(Self {
            types: BoundedVec::new(provider.clone())?,
            imports: BoundedVec::new(provider.clone())?,
            exports: BoundedVec::new(provider.clone())?,
            provider,
        })
    }

    /// Add a type to the component
    pub fn add_type(&mut self, ty: BoundedComponentType<P>) -> Result<u32, wrt_foundation::bounded::CapacityError> {
        let index = self.types.len() as u32;
        self.types.push(ty)?;
        Ok(index)
    }

    /// Add an import to the component
    pub fn add_import(&mut self, import: BoundedImport<P>) -> Result<u32, wrt_foundation::bounded::CapacityError> {
        let index = self.imports.len() as u32;
        self.imports.push(import)?;
        Ok(index)
    }

    /// Add an export to the component
    pub fn add_export(&mut self, export: BoundedExport<P>) -> Result<u32, wrt_foundation::bounded::CapacityError> {
        let index = self.exports.len() as u32;
        self.exports.push(export)?;
        Ok(index)
    }

    /// Get a type by index
    pub fn get_type(&self, index: u32) -> Option<&BoundedComponentType<P>> {
        self.types.get(index as usize).ok()
    }

    /// Get an import by index
    pub fn get_import(&self, index: u32) -> Option<&BoundedImport<P>> {
        self.imports.get(index as usize).ok()
    }

    /// Get an export by index
    pub fn get_export(&self, index: u32) -> Option<&BoundedExport<P>> {
        self.exports.get(index as usize).ok()
    }

    /// Get the number of types
    pub fn type_count(&self) -> u32 {
        self.types.len() as u32
    }

    /// Get the number of imports
    pub fn import_count(&self) -> u32 {
        self.imports.len() as u32
    }

    /// Get the number of exports
    pub fn export_count(&self) -> u32 {
        self.exports.len() as u32
    }
}

impl<P: MemoryProvider + Default + Clone + PartialEq + Eq> Default for BoundedComponent<P> {
    fn default() -> Self {
        Self::new(P::default()).unwrap_or_else(|_| {
            // Fallback to empty component if creation fails
            Self {
                types: BoundedVec::new(P::default()).unwrap(),
                imports: BoundedVec::new(P::default()).unwrap(),
                exports: BoundedVec::new(P::default()).unwrap(),
                provider: P::default(),
            }
        })
    }
}

/// Static type store for compile-time type registration
pub struct StaticTypeStore<const N: usize> {
    types: [Option<CoreValueType>; N],
    count: usize,
}

impl<const N: usize> StaticTypeStore<N> {
    /// Create a new static type store
    pub const fn new() -> Self {
        Self {
            types: [None; N],
            count: 0,
        }
    }

    /// Add a type at compile time
    pub const fn add_type(mut self, ty: CoreValueType) -> Self {
        if self.count < N {
            self.types[self.count] = Some(ty;
            self.count += 1;
        }
        self
    }

    /// Get a type by index
    pub const fn get_type(&self, index: usize) -> Option<CoreValueType> {
        if index < N {
            self.types[index]
        } else {
            None
        }
    }

    /// Get the number of types
    pub const fn len(&self) -> usize {
        self.count
    }

    /// Check if the store is empty
    pub const fn is_empty(&self) -> bool {
        self.count == 0
    }
}

/// Feature detection for no_std component model
pub const HAS_COMPONENT_MODEL_NO_STD: bool = true;
pub const HAS_WIT_PARSING_NO_STD: bool = true; // Now implemented with bounded parser

/// Const-friendly function type constructor
pub const fn const_core_func_type(
    params: &'static [CoreValueType],
    results: &'static [CoreValueType],
) -> (&'static [CoreValueType], &'static [CoreValueType]) {
    (params, results)
}

#[cfg(test)]
mod tests {
    use super::*;
    use wrt_foundation::NoStdProvider;

    type TestProvider = NoStdProvider<4096>;

    #[test]
    fn test_bounded_component_creation() {
        let provider = TestProvider::default());
        let component = BoundedComponent::new(provider;
        assert!(component.is_ok());

        let component = component.unwrap();
        assert_eq!(component.type_count(), 0);
        assert_eq!(component.import_count(), 0);
        assert_eq!(component.export_count(), 0);
    }

    #[test]
    fn test_static_type_store() {
        const STORE: StaticTypeStore<4> = StaticTypeStore::new()
            .add_type(CoreValueType::I32)
            .add_type(CoreValueType::F64;

        assert_eq!(STORE.len(), 2;
        assert_eq!(STORE.get_type(0), Some(CoreValueType::I32;
        assert_eq!(STORE.get_type(1), Some(CoreValueType::F64;
        assert_eq!(STORE.get_type(2), None;
    }

    #[test]
    fn test_const_func_type() {
        const FUNC_TYPE: (&[CoreValueType], &[CoreValueType]) = const_core_func_type(
            &[CoreValueType::I32, CoreValueType::I32],
            &[CoreValueType::I64]
        ;

        assert_eq!(FUNC_TYPE.0.len(), 2;
        assert_eq!(FUNC_TYPE.1.len(), 1);
        assert_eq!(FUNC_TYPE.0[0], CoreValueType::I32;
        assert_eq!(FUNC_TYPE.1[0], CoreValueType::I64;
    }
}