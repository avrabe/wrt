#![cfg(test)]

//! Comprehensive verification test for no_std without alloc functionality
//!
//! This file provides comprehensive testing of functionality that must work
//! in the most restrictive no_std without alloc environment. This ensures
//! that core WRT features work correctly on embedded and bare-metal systems.

// Binary std/no_std choice

#[cfg(test)]
mod tests {
    // Import from wrt-error
    use wrt_error::{ErrorCategory, Result};
    
    // Import from wrt-foundation
    use wrt_foundation::{
        bounded::{BoundedVec, BoundedStack},
        safe_memory::{SafeSlice, SafeStack},
        resource::ResourceId,
        values::Value,
        ValueType,
    };
    
    // Import from wrt-sync
    use wrt_sync::{
        mutex::RawMutex,
        rwlock::RawRwLock,
    };
    
    // Import from wrt-math
    use wrt_math::float_bits::{transmute_f32_to_u32, transmute_u32_to_f32};
    
    // Import from wrt-platform
    use wrt_platform::memory::{page_size, protect_memory, MemoryProtection};
    
    #[test]
    fn test_bounded_vec_without_alloc() {
        // Binary std/no_std choice
        let mut vec = BoundedVec::<u32, 10>::new);
        
        // Fill it with values
        for i in 0..5 {
            assert!(vec.push(i).is_ok();
        }
        
        // Test operations
        assert_eq!(vec.len(), 5;
        assert_eq!(vec.get(2), Some(&2;
        
        // Test bounds checking
        assert_eq!(vec.get(10), None;
        
        // Test full capacity
        for i in 5..10 {
            assert!(vec.push(i).is_ok();
        }
        
        // Test overflow prevention
        assert!(vec.push(100).is_err();
    }
    
    #[test]
    fn test_bounded_stack_without_alloc() {
        // Binary std/no_std choice
        let mut stack = BoundedStack::<u32, 5>::new);
        
        // Push values
        assert!(stack.push(1).is_ok();
        assert!(stack.push(2).is_ok();
        assert!(stack.push(3).is_ok();
        
        // Test LIFO behavior
        assert_eq!(stack.pop(), Some(3;
        assert_eq!(stack.pop(), Some(2;
        assert_eq!(stack.top(), Some(&1;
        
        // Test overflow prevention
        stack.push(2).unwrap();
        stack.push(3).unwrap();
        stack.push(4).unwrap();
        assert!(stack.push(5).is_err();
    }
    
    #[test]
    fn test_resource_id_without_alloc() {
        // Binary std/no_std choice
        let id1 = ResourceId::new(42;
        let id2 = ResourceId::new(43;
        
        assert_eq!(id1.get(), 42;
        assert_ne!(id1, id2;
    }
    
    #[test]
    fn test_safe_slice_without_alloc() {
        // Create a static array
        let data = [1, 2, 3, 4, 5];
        
        // Binary std/no_std choice
        let slice = SafeSlice::new(&data;
        
        // Test operations
        assert_eq!(slice.len(), 5;
        assert_eq!(slice.read_u8(0).unwrap(), 1;
        assert_eq!(slice.read_u8(4).unwrap(), 5;
        
        // Test bounds checking
        assert!(slice.read_u8(5).is_err();
    }
    
    #[test]
    fn test_value_operations_without_alloc() {
        // Binary std/no_std choice
        let i32_val = Value::I32(42;
        let i64_val = Value::I64(84;
        let f32_val = Value::F32(3.14;
        
        assert_eq!(i32_val.get_type(), ValueType::I32;
        assert_eq!(i64_val.get_type(), ValueType::I64;
        assert_eq!(f32_val.get_type(), ValueType::F32;
        
        // Test value comparisons
        assert_ne!(i32_val, i64_val;
        
        // Test value extraction
        if let Value::I32(value) = i32_val {
            assert_eq!(value, 42;
        } else {
            panic!("Incorrect value type";
        }
    }
    
    #[test]
    fn test_math_operations_without_alloc() {
        // Binary std/no_std choice
        let bits: u32 = transmute_f32_to_u32(3.14;
        let float: f32 = transmute_u32_to_f32(bits;
        
        // Due to floating point precision, use approximate comparison
        assert!((float - 3.14).abs() < 0.0001);
    }
    
    #[test]
    fn test_mutex_without_alloc() {
        // Binary std/no_std choice
        let mutex = RawMutex::new);
        
        // Test lock/unlock
        unsafe {
            mutex.lock);
            
            // Critical section here
            let value = 42;
            assert_eq!(value, 42;
            
            mutex.unlock);
        }
    }
    
    #[test]
    fn test_rwlock_without_alloc() {
        // Binary std/no_std choice
        let rwlock = RawRwLock::new);
        
        // Test read lock
        unsafe {
            rwlock.read_lock);
            
            // Read operation here
            let value = 42;
            assert_eq!(value, 42;
            
            rwlock.read_unlock);
        }
        
        // Test write lock
        unsafe {
            rwlock.write_lock);
            
            // Write operation here
            let mut value = 42;
            value += 1;
            assert_eq!(value, 43;
            
            rwlock.write_unlock);
        }
    }
    
    #[test]
    fn test_platform_page_size() {
        // Binary std/no_std choice
        let size = page_size);
        
        // Page size should be a power of 2 and greater than 0
        assert!(size > 0);
        assert!(size & (size - 1) == 0, "Page size must be a power of 2");
    }
    
    #[test]
    fn test_safe_stack_operations() {
        // Binary std/no_std choice
        let mut stack = SafeStack::<u32, 5>::new);
        
        // Test stack operations
        assert!(stack.push(1).is_ok();
        assert!(stack.push(2).is_ok();
        
        assert_eq!(stack.pop(), Some(2;
        assert_eq!(stack.peek(), Some(&1;
        
        // Fill to capacity
        stack.push(2).unwrap();
        stack.push(3).unwrap();
        stack.push(4).unwrap();
        stack.push(5).unwrap();
        
        // Test overflow protection
        assert!(stack.push(6).is_err();
    }
}