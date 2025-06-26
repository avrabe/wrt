//! Capability-aware allocators
//!
//! This module provides safe wrappers for heap allocations that integrate
//! with the capability system to ensure all allocations are tracked and verified.

use core::marker::PhantomData;
use crate::{
    capabilities::{MemoryCapabilityContext, MemoryOperation},
    budget_aware_provider::CrateId,
    codes, Error, ErrorCategory, Result,
};

#[cfg(any(feature = "std", feature = "alloc"))]
extern crate alloc;

#[cfg(feature = "std")]
use std::boxed::Box;
#[cfg(all(not(feature = "std"), feature = "alloc"))]
use alloc::boxed::Box;

#[cfg(any(feature = "std", feature = "alloc"))]
use alloc::vec::Vec;

/// Capability-aware Box allocator
pub struct CapabilityBox<T> {
    _phantom: PhantomData<T>,
}

#[cfg(any(feature = "std", feature = "alloc"))]
impl<T> CapabilityBox<T> {
    /// Allocate a Box with capability verification
    pub fn new(
        value: T,
        context: &MemoryCapabilityContext,
        crate_id: CrateId,
    ) -> Result<Box<T>> {
        let size = core::mem::size_of::<T>();
        let operation = MemoryOperation::Allocate { size };
        context.verify_operation(crate_id, &operation)?;
        
        Ok(Box::new(value))
    }
    
    /// Allocate a Box with default value and capability verification
    pub fn new_default(
        context: &MemoryCapabilityContext,
        crate_id: CrateId,
    ) -> Result<Box<T>>
    where
        T: Default,
    {
        Self::new(T::default(), context, crate_id)
    }
}

/// Capability-aware Vec allocator
pub struct CapabilityVec<T> {
    _phantom: PhantomData<T>,
}

#[cfg(any(feature = "std", feature = "alloc"))]
impl<T> CapabilityVec<T> {
    /// Create a new Vec with capability verification
    pub fn new(
        context: &MemoryCapabilityContext,
        crate_id: CrateId,
        capacity: usize,
    ) -> Result<Vec<T>> {
        let size = capacity * core::mem::size_of::<T>();
        let operation = MemoryOperation::Allocate { size };
        context.verify_operation(crate_id, &operation)?;
        
        Ok(Vec::with_capacity(capacity))
    }
    
    /// Create a Vec from elements with capability verification
    pub fn from_vec(
        elements: Vec<T>,
        context: &MemoryCapabilityContext,
        crate_id: CrateId,
    ) -> Result<Vec<T>> {
        let size = elements.capacity() * core::mem::size_of::<T>();
        let operation = MemoryOperation::Allocate { size };
        context.verify_operation(crate_id, &operation)?;
        
        Ok(elements)
    }
    
    /// Create a Vec from slice with capability verification
    pub fn from_slice(
        slice: &[T],
        context: &MemoryCapabilityContext,
        crate_id: CrateId,
    ) -> Result<Vec<T>>
    where
        T: Clone,
    {
        let size = slice.len() * core::mem::size_of::<T>();
        let operation = MemoryOperation::Allocate { size };
        context.verify_operation(crate_id, &operation)?;
        
        Ok(slice.to_vec())
    }
}

/// Capability-aware allocator trait for any type
pub trait CapabilityAlloc<T> {
    /// Allocate with capability verification
    fn capability_alloc(
        &self,
        context: &MemoryCapabilityContext,
        crate_id: CrateId,
    ) -> Result<T>;
}

/// Global capability allocation functions for convenience
pub mod capability_alloc {
    use super::*;
    
    /// Allocate a Box with capability verification
    #[cfg(any(feature = "std", feature = "alloc"))]
    pub fn capability_box<T>(
        value: T,
        context: &MemoryCapabilityContext,
        crate_id: CrateId,
    ) -> Result<Box<T>> {
        CapabilityBox::new(value, context, crate_id)
    }
    
    /// Allocate a Vec with capability verification
    #[cfg(any(feature = "std", feature = "alloc"))]
    pub fn capability_vec<T>(
        context: &MemoryCapabilityContext,
        crate_id: CrateId,
        capacity: usize,
    ) -> Result<Vec<T>> {
        CapabilityVec::new(context, crate_id, capacity)
    }
    
    /// Allocate a Vec from elements with capability verification
    #[cfg(any(feature = "std", feature = "alloc"))]
    pub fn capability_vec_from<T>(
        elements: Vec<T>,
        context: &MemoryCapabilityContext,
        crate_id: CrateId,
    ) -> Result<Vec<T>> {
        CapabilityVec::from_vec(elements, context, crate_id)
    }
}

/// Macros for capability-aware allocation
#[macro_export]
macro_rules! capability_box {
    ($value:expr, $context:expr, $crate_id:expr) => {
        $crate::capability_allocators::capability_alloc::capability_box($value, $context, $crate_id)
    };
}

#[macro_export]
macro_rules! capability_vec {
    (with_capacity($capacity:expr), $context:expr, $crate_id:expr) => {
        $crate::capability_allocators::capability_alloc::capability_vec($context, $crate_id, $capacity)
    };
    ([$($element:expr),*], $context:expr, $crate_id:expr) => {
        $crate::capability_allocators::capability_alloc::capability_vec_from(
            vec![$($element),*], $context, $crate_id
        )
    };
}

/// No-std fallback implementations
#[cfg(not(any(feature = "std", feature = "alloc")))]
mod no_std_impl {
    use super::*;
    
    impl<T> CapabilityBox<T> {
        pub fn new(
            _value: T,
            _context: &MemoryCapabilityContext,
            _crate_id: CrateId,
        ) -> Result<()> {
            Err(Error::runtime_execution_error("
            ))
        }
    }
    
    impl<T> CapabilityVec<T> {
        pub fn new(
            _context: &MemoryCapabilityContext,
            _crate_id: CrateId,
            _capacity: usize,
        ) -> Result<()> {
            Err(Error::new(
                ErrorCategory::Runtime,
                codes::UNSUPPORTED_OPERATION,
                "))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capabilities::{DynamicMemoryCapability, CapabilityMask};
    
    #[cfg(any(feature = "std", feature = "alloc"))]
    #[test]
    fn test_capability_box_allocation() {
        let mut context = MemoryCapabilityContext::new(
            crate::verification::VerificationLevel::Standard,
            false
        );
        
        // Register capability for testing
        let _ = context.register_dynamic_capability(CrateId::Runtime, 1024);
        
        let boxed_value = CapabilityBox::new(42u32, &context, CrateId::Runtime);
        assert!(boxed_value.is_ok());
        assert_eq!(*boxed_value.unwrap(), 42);
    }
    
    #[cfg(any(feature = "std", feature = "alloc"))]
    #[test]
    fn test_capability_vec_allocation() {
        let mut context = MemoryCapabilityContext::new(
            crate::verification::VerificationLevel::Standard,
            false
        );
        
        // Register capability for testing
        let _ = context.register_dynamic_capability(CrateId::Runtime, 1024);
        
        let vec_result = CapabilityVec::<u32>::new(&context, CrateId::Runtime, 10);
        assert!(vec_result.is_ok());
        
        let vec = vec_result.unwrap();
        assert_eq!(vec.capacity(), 10);
    }
    
    #[cfg(any(feature = "std", feature = "alloc"))]
    #[test]
    fn test_capability_allocation_without_permission() {
        let context = MemoryCapabilityContext::new(
            crate::verification::VerificationLevel::Standard,
            false
        );
        
        // Don't register any capabilities
        let boxed_value = CapabilityBox::new(42u32, &context, CrateId::Runtime);
        assert!(boxed_value.is_err());
    }
}