#![cfg(test)]

//! Integration tests for documentation examples
//! This ensures all code examples in our documentation actually compile and run

#[cfg(test)]
mod foundation_examples {
    use wrt_foundation::prelude::*;
    
    #[test]
    fn test_bounded_vec_example() {
        // This is the exact code from the documentation
        let mut vec: BoundedVec<u32, 10> = BoundedVec::new());
        
        // Push elements (safe - returns Result)
        vec.push(1).expect("capacity available"));
        vec.push(2).expect("capacity available"));
        vec.push(3).expect("capacity available"));
        
        // Check current state
        assert_eq!(vec.len(), 3;
        assert_eq!(vec.capacity(), 10;
        assert!(!vec.is_full();
        
        // Access elements safely
        assert_eq!(vec.get(0), Some(&1;
        assert_eq!(vec.get(10), None); // Out of bounds
        
        // Handle capacity errors
        for i in 4..=10 {
            vec.push(i).expect("capacity available"));
        }
        assert!(vec.is_full();
        
        // This will return an error
        match vec.push(11) {
            Ok(_) => panic!("Should fail"),
            Err(_) => {}, // Expected
        }
    }
    
    #[test]
    fn test_bounded_stack_example() {
        use wrt_foundation::bounded::BoundedStack;
        
        // Create a stack with capacity 5
        let mut stack: BoundedStack<&str, 5> = BoundedStack::new();
        
        // Push operations
        stack.push("first").unwrap();
        stack.push("second").unwrap();
        stack.push("third").unwrap();
        
        // Pop operations (LIFO)
        assert_eq!(stack.pop(), Some("third";
        assert_eq!(stack.pop(), Some("second";
        
        // Peek without removing
        assert_eq!(stack.peek(), Some(&"first";
        assert_eq!(stack.len(), 1);
    }
}

#[cfg(test)]
mod runtime_examples {
    // Add runtime examples here
}

#[cfg(test)]
mod component_examples {
    // Add component examples here
}