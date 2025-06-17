//! Example ASIL-Tagged Tests
//!
//! This module demonstrates the usage of ASIL test macros and shows how
//! to categorize tests by safety level and requirements.

use wrt_foundation::{
    asil_testing::TestCategory,
    safety_system::AsilLevel,
    BoundedVec, BoundedDeque, SafeSlice, SafeMemoryHandler,
    VerificationLevel,
    asil_test, asil_d_test, asil_c_test, memory_safety_test, resource_safety_test,
, safe_managed_alloc};

// Example ASIL-D tests for highest safety integrity
asil_d_test! {
    name: memory_bounds_checking_asil_d,
    requirement: "REQ_MEM_001",
    category: TestCategory::Memory,
    description: "Verify memory bounds checking prevents buffer overflows (ASIL-D)",
    test: {
        // Create a bounded vector with strict capacity
        let guard = safe_managed_alloc!(64, CrateId::Foundation)?;

        let provider = unsafe { guard.release() };
        let mut vec = BoundedVec::<u32, 4, _>::new(provider).unwrap();
        
        // Test that we can add up to capacity
        for i in 0..4 {
            assert!(vec.push(i).is_ok(), "Should be able to push within capacity");
        }
        
        // Test that exceeding capacity fails safely
        assert!(vec.push(4).is_err(), "Should fail when exceeding capacity");
        assert_eq!(vec.len(), 4, "Length should remain at capacity");
        
        // Verify no memory corruption occurred
        for (i, &value) in vec.iter().enumerate() {
            assert_eq!(value, i as u32, "Values should be intact after capacity exceeded");
        }
    }
}

asil_d_test! {
    name: safe_slice_bounds_verification_asil_d,
    requirement: "REQ_MEM_002", 
    category: TestCategory::Memory,
    description: "Verify SafeSlice prevents out-of-bounds memory access (ASIL-D)",
    test: {
        let data = [1u32, 2, 3, 4, 5];
        let safe_slice = SafeSlice::new(&data);
        
        // Test valid access
        assert_eq!(safe_slice.get(0), Some(&1));
        assert_eq!(safe_slice.get(4), Some(&5));
        
        // Test invalid access fails safely
        assert_eq!(safe_slice.get(5), None, "Out-of-bounds access should return None");
        assert_eq!(safe_slice.get(100), None, "Large out-of-bounds access should return None");
        
        // Test slice operation bounds
        let sub_slice = safe_slice.get_slice(1, 3);
        assert!(sub_slice.is_some(), "Valid slice should succeed");
        assert_eq!(sub_slice.unwrap().len(), 2, "Slice should have correct length");
        
        let invalid_slice = safe_slice.get_slice(3, 10);
        assert!(invalid_slice.is_none(), "Invalid slice should fail safely");
    }
}

// Example ASIL-C tests for moderate safety integrity
asil_c_test! {
    name: resource_exhaustion_handling_asil_c,
    requirement: "REQ_RES_001",
    category: TestCategory::Resource,
    description: "Verify graceful handling of resource exhaustion (ASIL-C)",
    test: {
        let guard = safe_managed_alloc!(32, CrateId::Foundation)?;

        let provider = unsafe { guard.release() };
        let mut deque = BoundedDeque::<u64, 2, _>::new(provider).unwrap();
        
        // Fill to capacity
        assert!(deque.push_back(100).is_ok());
        assert!(deque.push_back(200).is_ok());
        
        // Verify resource exhaustion is handled gracefully
        let result = deque.push_back(300);
        assert!(result.is_err(), "Resource exhaustion should be detected");
        
        // Verify system remains stable after resource exhaustion
        assert_eq!(deque.len(), 2, "Container should maintain integrity");
        assert_eq!(deque.front(), Some(&100), "Data should remain intact");
        assert_eq!(deque.back(), Some(&200), "Data should remain intact");
        
        // Verify recovery after freeing resources
        let _freed = deque.pop_front();
        assert!(deque.push_back(300).is_ok(), "Should succeed after freeing space");
    }
}

memory_safety_test! {
    name: safe_memory_handler_verification,
    asil: AsilLevel::AsilC,
    requirement: "REQ_MEM_003",
    description: "Verify SafeMemoryHandler prevents unsafe memory operations",
    test: {
        let mut data = vec![0u8; 100];
        let handler = SafeMemoryHandler::new(&mut data);
        
        // Test safe read operations
        let read_result = handler.read_bytes(10, 5);
        assert!(read_result.is_ok(), "Safe read should succeed");
        
        // Test bounds checking on read
        let invalid_read = handler.read_bytes(95, 10);
        assert!(invalid_read.is_err(), "Out-of-bounds read should fail");
        
        // Test safe write operations
        let write_data = [1, 2, 3, 4, 5];
        let write_result = handler.write_bytes(20, &write_data);
        assert!(write_result.is_ok(), "Safe write should succeed");
        
        // Test bounds checking on write
        let large_write_data = [0u8; 50];
        let invalid_write = handler.write_bytes(80, &large_write_data);
        assert!(invalid_write.is_err(), "Out-of-bounds write should fail");
        
        // Verify data integrity after failed operations
        let verify_read = handler.read_bytes(20, 5);
        assert!(verify_read.is_ok(), "Verification read should succeed");
        assert_eq!(verify_read.unwrap(), write_data, "Data should match written values");
    }
}

resource_safety_test! {
    name: stack_overflow_prevention,
    asil: AsilLevel::AsilB,
    requirement: "REQ_RES_002", 
    description: "Verify stack overflow prevention mechanisms",
    test: {
        use wrt_foundation::SafeStack;
        
        let guard = safe_managed_alloc!(128, CrateId::Foundation)?;

        
        let provider = unsafe { guard.release() };
        let mut stack = SafeStack::<u32, 8, _>::new(provider).unwrap();
        
        // Test normal stack operations
        for i in 0..8 {
            assert!(stack.push(i).is_ok(), "Normal push should succeed");
        }
        
        // Test stack overflow prevention
        let overflow_result = stack.push(999);
        assert!(overflow_result.is_err(), "Stack overflow should be prevented");
        
        // Verify stack integrity after overflow attempt
        assert_eq!(stack.len(), 8, "Stack size should remain at capacity");
        assert_eq!(stack.peek(), Some(&7), "Top element should be unchanged");
        
        // Test recovery after popping elements
        let _popped = stack.pop();
        assert!(stack.push(999).is_ok(), "Push should succeed after pop");
        assert_eq!(stack.peek(), Some(&999), "New element should be on top");
    }
}

// Example of testing verification levels
asil_test! {
    name: verification_level_enforcement,
    asil: AsilLevel::AsilC,
    requirement: "REQ_VER_001",
    category: TestCategory::Safety,
    description: "Verify that verification levels are properly enforced",
    test: {
        // Test that different verification levels behave correctly
        let full_verification = VerificationLevel::Full;
        let standard_verification = VerificationLevel::Standard;
        let none_verification = VerificationLevel::None;
        
        // These would test actual verification behavior in a real implementation
        assert_ne!(full_verification, none_verification, "Verification levels should differ");
        assert_ne!(standard_verification, none_verification, "Verification levels should differ");
        
        // In a real test, we would verify that Full verification catches more issues
        // than Standard, and Standard catches more than None
    }
}

// Example integration test combining multiple safety features
asil_test! {
    name: integrated_safety_systems_test,
    asil: AsilLevel::AsilD,
    requirement: "REQ_INT_001",
    category: TestCategory::Integration,
    description: "Verify integration of multiple safety systems (ASIL-D)",
    test: {
        // This test would verify that multiple safety systems work together
        // For example: memory safety + resource limits + verification
        
        let guard = safe_managed_alloc!(256, CrateId::Foundation)?;

        
        let provider = unsafe { guard.release() };
        let mut bounded_vec = BoundedVec::<u32, 10, _>::new(provider).unwrap();
        
        // Test that resource limits and memory safety work together
        for i in 0..10 {
            let result = bounded_vec.push(i);
            assert!(result.is_ok(), "Should succeed within limits");
        }
        
        // Test that overflow is handled safely
        assert!(bounded_vec.push(10).is_err(), "Should fail when exceeding limits");
        
        // Test that the system maintains integrity under stress
        for _ in 0..100 {
            let _ = bounded_vec.push(999); // This should consistently fail
            assert_eq!(bounded_vec.len(), 10, "Length should remain stable");
        }
        
        // Verify data integrity
        for (i, &value) in bounded_vec.iter().enumerate() {
            assert_eq!(value, i as u32, "Data should remain intact");
        }
    }
}

#[cfg(test)]
mod framework_tests {
    use super::*;
    use wrt_foundation::asil_testing::*;

    #[test]
    fn test_asil_test_categorization() {
        // Get all ASIL-D tests (should include our examples above)
        let asil_d_tests = get_tests_by_asil(AsilLevel::AsilD);
        
        // Should have at least the ASIL-D tests we defined
        assert!(asil_d_tests.len() >= 3, 
            "Should have multiple ASIL-D tests, found: {}", asil_d_tests.len());
        
        // Check that memory tests are properly categorized
        let memory_tests = get_tests_by_category(TestCategory::Memory);
        assert!(memory_tests.len() >= 2, 
            "Should have multiple memory tests, found: {}", memory_tests.len());
        
        // Check that we have tests for different requirements
        let mut requirement_ids = std::collections::HashSet::new();
        for test in get_asil_tests() {
            requirement_ids.insert(test.requirement_id);
        }
        
        assert!(requirement_ids.contains("REQ_MEM_001"));
        assert!(requirement_ids.contains("REQ_MEM_002"));
        assert!(requirement_ids.contains("REQ_RES_001"));
    }

    #[test]
    fn test_statistics_accuracy() {
        let stats = get_test_statistics();
        
        // Should have a reasonable number of tests
        assert!(stats.total_count >= 6, 
            "Should have multiple ASIL tests, found: {}", stats.total_count);
        
        // Should have ASIL-D tests (highest safety level)
        assert!(stats.asil_d_count >= 2, 
            "Should have ASIL-D tests, found: {}", stats.asil_d_count);
        
        // Should have memory safety tests
        assert!(stats.memory_count >= 2, 
            "Should have memory tests, found: {}", stats.memory_count);
        
        // Should have resource safety tests
        assert!(stats.resource_count >= 2, 
            "Should have resource tests, found: {}", stats.resource_count);
        
        // Verify totals make sense
        let level_total = stats.qm_count + stats.asil_a_count + stats.asil_b_count + 
                         stats.asil_c_count + stats.asil_d_count;
        assert_eq!(level_total, stats.total_count, 
            "ASIL level counts should sum to total");
        
        let category_total = stats.unit_count + stats.integration_count + stats.safety_count + 
                           stats.performance_count + stats.memory_count + stats.resource_count;
        assert_eq!(category_total, stats.total_count, 
            "Category counts should sum to total");
    }
}