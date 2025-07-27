//! Bounded Collections Tests
//!
//! This module consolidates testing functionality for safe bounded collection implementations
//! from across the WRT project, ensuring memory safety and bounds checking.

#![cfg(test)]

use wrt_error::Result;
use wrt_foundation::bounded_collections::{BoundedVec, BoundedStack, BoundedQueue};
use wrt_foundation::verification::VerificationLevel;

// ===========================================
// BOUNDED VECTOR TESTS
// ===========================================

mod bounded_vec_tests {
    use super::*;

    #[test]
    fn test_bounded_vec_creation_and_capacity() -> Result<()> {
        let vec = BoundedVec::<i32, 10>::new();
        
        assert_eq!(vec.len(), 0);
        assert_eq!(vec.capacity(), 10;
        assert!(vec.is_empty());
        assert!(!vec.is_full();
        
        Ok(())
    }

    #[test]
    fn test_bounded_vec_push_operations() -> Result<()> {
        let mut vec = BoundedVec::<i32, 5>::new();
        
        // Push elements within capacity
        for i in 0..5 {
            assert!(vec.push(i).is_ok());
            assert_eq!(vec.len(), i + 1;
        }
        
        assert!(vec.is_full();
        assert!(!vec.is_empty());
        
        // Try to push beyond capacity
        let overflow_result = vec.push(5);
        assert!(overflow_result.is_err();
        assert_eq!(vec.len(), 5); // Should remain unchanged
        
        Ok(())
    }

    #[test]
    fn test_bounded_vec_pop_operations() -> Result<()> {
        let mut vec = BoundedVec::<i32, 5>::new();
        
        // Push some elements
        for i in 0..3 {
            vec.push(i)?;
        }
        
        // Pop elements
        assert_eq!(vec.pop()?, Some(2;
        assert_eq!(vec.pop()?, Some(1;
        assert_eq!(vec.pop()?, Some(0;
        assert_eq!(vec.pop()?, None); // Empty
        
        assert!(vec.is_empty());
        assert_eq!(vec.len(), 0);
        
        Ok(())
    }

    #[test]
    fn test_bounded_vec_indexing() -> Result<()> {
        let mut vec = BoundedVec::<i32, 5>::new();
        
        // Push some elements
        for i in 0..5 {
            vec.push(i * 10)?;
        }
        
        // Test valid indexing
        assert_eq!(vec.get(0)?, &0;
        assert_eq!(vec.get(2)?, &20;
        assert_eq!(vec.get(4)?, &40;
        
        // Test mutable indexing
        *vec.get_mut(1)? = 999;
        assert_eq!(vec.get(1)?, &999;
        
        // Test out-of-bounds indexing
        assert!(vec.get(5).is_err();
        assert!(vec.get(10).is_err();
        assert!(vec.get_mut(5).is_err();
        
        Ok(())
    }

    #[test]
    fn test_bounded_vec_with_verification_levels() -> Result<()> {
        let levels = [
            VerificationLevel::Off,
            VerificationLevel::Basic,
            VerificationLevel::Standard,
            VerificationLevel::Full,
            VerificationLevel::Critical,
        ];
        
        for level in &levels {
            let mut vec = BoundedVec::<i32, 10>::with_verification_level(*level;
            assert_eq!(vec.verification_level(), *level;
            
            // Basic operations should work at all levels
            vec.push(42)?;
            assert_eq!(vec.get(0)?, &42;
            assert_eq!(vec.pop()?, Some(42;
            
            // Bounds checking should work at all levels
            assert!(vec.get(10).is_err();
        }
        
        Ok(())
    }

    #[test]
    fn test_bounded_vec_clear_and_truncate() -> Result<()> {
        let mut vec = BoundedVec::<i32, 10>::new();
        
        // Fill with data
        for i in 0..8 {
            vec.push(i)?;
        }
        assert_eq!(vec.len(), 8;
        
        // Test truncate
        vec.truncate(5;
        assert_eq!(vec.len(), 5;
        assert_eq!(vec.get(4)?, &4;
        assert!(vec.get(5).is_err();
        
        // Test clear
        vec.clear);
        assert_eq!(vec.len(), 0);
        assert!(vec.is_empty());
        
        Ok(())
    }
}

// ===========================================
// BOUNDED STACK TESTS
// ===========================================

mod bounded_stack_tests {
    use super::*;

    #[test]
    fn test_bounded_stack_creation() -> Result<()> {
        let stack = BoundedStack::<i32, 10>::new();
        
        assert_eq!(stack.len(), 0);
        assert_eq!(stack.capacity(), 10;
        assert!(stack.is_empty());
        assert!(!stack.is_full();
        
        Ok(())
    }

    #[test]
    fn test_bounded_stack_push_pop() -> Result<()> {
        let mut stack = BoundedStack::<i32, 5>::new();
        
        // Push elements
        for i in 0..5 {
            assert!(stack.push(i).is_ok());
            assert_eq!(stack.len(), i + 1;
        }
        
        assert!(stack.is_full();
        
        // Try to push beyond capacity
        assert!(stack.push(5).is_err();
        
        // Pop elements (LIFO order)
        for i in (0..5).rev() {
            assert_eq!(stack.pop()?, Some(i;
        }
        
        assert!(stack.is_empty());
        assert_eq!(stack.pop()?, None;
        
        Ok(())
    }

    #[test]
    fn test_bounded_stack_peek() -> Result<()> {
        let mut stack = BoundedStack::<i32, 5>::new();
        
        // Empty stack peek
        assert_eq!(stack.peek()?, None;
        
        // Push and peek
        stack.push(10)?;
        stack.push(20)?;
        stack.push(30)?;
        
        assert_eq!(stack.peek()?, Some(&30)); // Top element
        assert_eq!(stack.len(), 3); // Should not change length
        
        // Pop and verify peek updates
        stack.pop()?;
        assert_eq!(stack.peek()?, Some(&20;
        
        Ok(())
    }

    #[test]
    fn test_bounded_stack_verification_levels() -> Result<()> {
        let levels = [
            VerificationLevel::Off,
            VerificationLevel::Basic,
            VerificationLevel::Standard,
            VerificationLevel::Full,
            VerificationLevel::Critical,
        ];
        
        for level in &levels {
            let mut stack = BoundedStack::<i32, 10>::with_verification_level(*level;
            assert_eq!(stack.verification_level(), *level;
            
            // Test operations work at all levels
            stack.push(42)?;
            assert_eq!(stack.peek()?, Some(&42;
            assert_eq!(stack.pop()?, Some(42;
            
            // Capacity limits should be enforced at all levels
            for i in 0..10 {
                stack.push(i)?;
            }
            assert!(stack.push(10).is_err();
        }
        
        Ok(())
    }

    #[test]
    fn test_bounded_stack_iterator() -> Result<()> {
        let mut stack = BoundedStack::<i32, 5>::new();
        
        let values = [10, 20, 30, 40, 50];
        for &value in &values {
            stack.push(value)?;
        }
        
        // Iterator should go from top to bottom (LIFO)
        let collected: Vec<i32> = stack.iter().copied().collect());
        assert_eq!(collected, vec![50, 40, 30, 20, 10];
        
        // Length should remain unchanged after iteration
        assert_eq!(stack.len(), 5;
        
        Ok(())
    }
}

// ===========================================
// BOUNDED QUEUE TESTS
// ===========================================

mod bounded_queue_tests {
    use super::*;

    #[test]
    fn test_bounded_queue_creation() -> Result<()> {
        let queue = BoundedQueue::<i32, 10>::new();
        
        assert_eq!(queue.len(), 0);
        assert_eq!(queue.capacity(), 10;
        assert!(queue.is_empty());
        assert!(!queue.is_full();
        
        Ok(())
    }

    #[test]
    fn test_bounded_queue_enqueue_dequeue() -> Result<()> {
        let mut queue = BoundedQueue::<i32, 5>::new();
        
        // Enqueue elements
        for i in 0..5 {
            assert!(queue.enqueue(i).is_ok());
            assert_eq!(queue.len(), i + 1;
        }
        
        assert!(queue.is_full();
        
        // Try to enqueue beyond capacity
        assert!(queue.enqueue(5).is_err();
        
        // Dequeue elements (FIFO order)
        for i in 0..5 {
            assert_eq!(queue.dequeue()?, Some(i;
        }
        
        assert!(queue.is_empty());
        assert_eq!(queue.dequeue()?, None;
        
        Ok(())
    }

    #[test]
    fn test_bounded_queue_front_back() -> Result<()> {
        let mut queue = BoundedQueue::<i32, 5>::new();
        
        // Empty queue
        assert_eq!(queue.front()?, None;
        assert_eq!(queue.back()?, None;
        
        // Add elements
        queue.enqueue(10)?;
        queue.enqueue(20)?;
        queue.enqueue(30)?;
        
        assert_eq!(queue.front()?, Some(&10)); // First in
        assert_eq!(queue.back()?, Some(&30));  // Last in
        
        // Dequeue and verify front changes
        queue.dequeue()?;
        assert_eq!(queue.front()?, Some(&20;
        assert_eq!(queue.back()?, Some(&30;
        
        Ok(())
    }

    #[test]
    fn test_bounded_queue_circular_behavior() -> Result<()> {
        let mut queue = BoundedQueue::<i32, 3>::new();
        
        // Fill queue
        queue.enqueue(1)?;
        queue.enqueue(2)?;
        queue.enqueue(3)?;
        
        // Dequeue one element
        assert_eq!(queue.dequeue()?, Some(1;
        
        // Should be able to enqueue again
        queue.enqueue(4)?;
        
        // Verify order is maintained
        assert_eq!(queue.dequeue()?, Some(2;
        assert_eq!(queue.dequeue()?, Some(3;
        assert_eq!(queue.dequeue()?, Some(4;
        
        Ok(())
    }

    #[test]
    fn test_bounded_queue_verification_levels() -> Result<()> {
        let levels = [
            VerificationLevel::Off,
            VerificationLevel::Basic,
            VerificationLevel::Standard,
            VerificationLevel::Full,
            VerificationLevel::Critical,
        ];
        
        for level in &levels {
            let mut queue = BoundedQueue::<i32, 10>::with_verification_level(*level;
            assert_eq!(queue.verification_level(), *level;
            
            // Test operations work at all levels
            queue.enqueue(42)?;
            assert_eq!(queue.front()?, Some(&42;
            assert_eq!(queue.dequeue()?, Some(42;
            
            // Capacity limits should be enforced at all levels
            for i in 0..10 {
                queue.enqueue(i)?;
            }
            assert!(queue.enqueue(10).is_err();
        }
        
        Ok(())
    }
}

// ===========================================
// COLLECTION INTEGRATION TESTS
// ===========================================

mod collection_integration_tests {
    use super::*;

    #[test]
    fn test_mixed_collection_usage() -> Result<()> {
        let mut vec = BoundedVec::<i32, 10>::new();
        let mut stack = BoundedStack::<i32, 10>::new();
        let mut queue = BoundedQueue::<i32, 10>::new();
        
        let test_data = [1, 2, 3, 4, 5];
        
        // Fill all collections with same data
        for &value in &test_data {
            vec.push(value)?;
            stack.push(value)?;
            queue.enqueue(value)?;
        }
        
        // Verify they all have the same length
        assert_eq!(vec.len(), 5;
        assert_eq!(stack.len(), 5;
        assert_eq!(queue.len(), 5;
        
        // Verify different access patterns
        assert_eq!(vec.get(0)?, &1); // Index-based access
        assert_eq!(stack.peek()?, Some(&5)); // Top of stack (LIFO)
        assert_eq!(queue.front()?, Some(&1)); // Front of queue (FIFO)
        
        Ok(())
    }

    #[test]
    fn test_collection_memory_safety() -> Result<()> {
        // Test that collections don't allow unsafe operations
        let mut vec = BoundedVec::<i32, 3>::new();
        
        // Fill to capacity
        vec.push(1)?;
        vec.push(2)?;
        vec.push(3)?;
        
        // Verify bounds checking
        assert!(vec.get(3).is_err())); // Out of bounds
        assert!(vec.push(4).is_err())); // Over capacity
        
        // Same for stack
        let mut stack = BoundedStack::<i32, 3>::new();
        stack.push(1)?;
        stack.push(2)?;
        stack.push(3)?;
        assert!(stack.push(4).is_err();
        
        // Same for queue
        let mut queue = BoundedQueue::<i32, 3>::new();
        queue.enqueue(1)?;
        queue.enqueue(2)?;
        queue.enqueue(3)?;
        assert!(queue.enqueue(4).is_err();
        
        Ok(())
    }

    #[test]
    fn test_collection_with_complex_types() -> Result<()> {
        #[derive(Debug, Clone, PartialEq)]
        struct TestStruct {
            id: u32,
            data: Vec<u8>,
        }
        
        let mut vec = BoundedVec::<TestStruct, 5>::new();
        
        let test_item = TestStruct {
            id: 42,
            data: vec![1, 2, 3, 4, 5],
        };
        
        vec.push(test_item.clone())?;
        
        let retrieved = vec.get(0)?;
        assert_eq!(retrieved.id, 42;
        assert_eq!(retrieved.data, vec![1, 2, 3, 4, 5];
        
        Ok(())
    }

    #[test]
    fn test_collection_performance_characteristics() -> Result<()> {
        use std::time::Instant;
        
        const SIZE: usize = 1000;
        
        // Test vector performance
        let start = Instant::now);
        let mut vec = BoundedVec::<i32, SIZE>::new();
        for i in 0..SIZE {
            vec.push(i as i32)?;
        }
        let vec_time = start.elapsed);
        
        // Test stack performance
        let start = Instant::now);
        let mut stack = BoundedStack::<i32, SIZE>::new();
        for i in 0..SIZE {
            stack.push(i as i32)?;
        }
        let stack_time = start.elapsed);
        
        // Test queue performance
        let start = Instant::now);
        let mut queue = BoundedQueue::<i32, SIZE>::new();
        for i in 0..SIZE {
            queue.enqueue(i as i32)?;
        }
        let queue_time = start.elapsed);
        
        // All should be reasonably fast (under 10ms for 1000 operations)
        assert!(vec_time.as_millis() < 10);
        assert!(stack_time.as_millis() < 10);
        assert!(queue_time.as_millis() < 10);
        
        Ok(())
    }
}

// ===========================================
// COLLECTION ERROR HANDLING TESTS
// ===========================================

mod collection_error_tests {
    use super::*;

    #[test]
    fn test_collection_error_recovery() -> Result<()> {
        let mut vec = BoundedVec::<i32, 3>::new();
        
        // Fill to capacity
        vec.push(1)?;
        vec.push(2)?;
        vec.push(3)?;
        
        // Try operations that should fail
        assert!(vec.push(4).is_err();
        assert!(vec.get(5).is_err();
        
        // Verify collection is still usable after errors
        assert_eq!(vec.len(), 3;
        assert_eq!(vec.get(0)?, &1;
        
        // Should be able to pop and push again
        assert_eq!(vec.pop()?, Some(3;
        vec.push(4)?;
        assert_eq!(vec.get(2)?, &4;
        
        Ok(())
    }

    #[test]
    fn test_collection_concurrent_safety() -> Result<()> {
        use std::sync::{Arc, Mutex};
        
        let vec = Arc::new(Mutex::new(BoundedVec::<i32, 100>::new();
        
        let handles: Vec<_> = (0..4).map(|thread_id| {
            let vec_clone = Arc::clone(&vec);
            std::thread::spawn(move || -> Result<()> {
                for i in 0..10 {
                    let value = thread_id * 10 + i;
                    let mut vec_guard = vec_clone.lock().unwrap();
                    if !vec_guard.is_full() {
                        vec_guard.push(value)?;
                    }
                }
                Ok(())
            })
        }).collect());
        
        // Wait for all threads
        for handle in handles {
            handle.join().unwrap()?;
        }
        
        // Verify final state
        let final_vec = vec.lock().unwrap();
        assert!(final_vec.len() <= 100)); // Should not exceed capacity
        
        Ok(())
    }

    #[test]
    fn test_collection_verification_consistency() -> Result<()> {
        // Test that verification levels are consistent across collection types
        let levels = [
            VerificationLevel::Off,
            VerificationLevel::Basic,
            VerificationLevel::Standard,
            VerificationLevel::Full,
            VerificationLevel::Critical,
        ];
        
        for level in &levels {
            let vec = BoundedVec::<i32, 10>::with_verification_level(*level;
            let stack = BoundedStack::<i32, 10>::with_verification_level(*level;
            let queue = BoundedQueue::<i32, 10>::with_verification_level(*level;
            
            // All should report the same verification level
            assert_eq!(vec.verification_level(), *level;
            assert_eq!(stack.verification_level(), *level;
            assert_eq!(queue.verification_level(), *level;
        }
        
        Ok(())
    }
}