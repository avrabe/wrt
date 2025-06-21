#![deny(warnings)]

// Comprehensive tests for the new bounded collections

use wrt_foundation::{
    bounded::BoundedErrorKind,
    BoundedBitSet, BoundedBuilder, BoundedDeque, BoundedMap, BoundedQueue, BoundedSet,
    MemoryBuilder, NoStdProvider, StringBuilder, WrtProviderFactory, budget_aware_provider::CrateId,
    VerificationLevel, traits::BoundedCapacity,
, safe_managed_alloc};

#[cfg(not(feature = "std"))]
extern crate alloc;

#[cfg(all(not(feature = "std")))]
use std::{string::String, vec::Vec};
#[cfg(feature = "std")]
use std::string::String;

#[test]
fn test_bounded_queue_operations() {
    let provider = safe_managed_alloc!(1024, CrateId::Foundation).unwrap();
    let mut queue = BoundedQueue::<u32, 5, NoStdProvider<1024>>::new(provider).unwrap();

    // Check empty queue properties
    assert_eq!(queue.len(), 0);
    assert_eq!(queue.capacity(), 5);
    assert!(queue.is_empty());
    assert!(!queue.is_full());
    assert!(queue.dequeue().unwrap().is_none());
    assert!(queue.peek().unwrap().is_none());

    // Test enqueue operations
    for i in 0..5 {
        assert!(queue.enqueue(i).is_ok());
        assert_eq!(queue.len(), i as usize + 1);
    }

    // Test full queue
    assert!(queue.is_full());
    assert_eq!(queue.enqueue(5).unwrap_err().kind(), BoundedErrorKind::CapacityExceeded);

    // Test peek
    assert_eq!(queue.peek().unwrap(), Some(0));
    assert_eq!(queue.len(), 5); // Peek doesn't change length

    // Test dequeue
    for i in 0..5 {
        assert_eq!(queue.dequeue().unwrap(), Some(i));
        assert_eq!(queue.len(), 4 - i as usize);
    }

    // Test empty queue after dequeue
    assert!(queue.is_empty());
    assert!(queue.dequeue().unwrap().is_none());

    // Test wrap-around behavior with mixed enqueue/dequeue
    for i in 0..3 {
        assert!(queue.enqueue(i).is_ok());
    }

    assert_eq!(queue.dequeue().unwrap(), Some(0));
    assert_eq!(queue.dequeue().unwrap(), Some(1));

    for i in 3..6 {
        assert!(queue.enqueue(i).is_ok());
    }

    assert_eq!(queue.len(), 4);

    // Verify expected queue contents after wrap-around
    assert_eq!(queue.dequeue().unwrap(), Some(2));
    assert_eq!(queue.dequeue().unwrap(), Some(3));
    assert_eq!(queue.dequeue().unwrap(), Some(4));
    assert_eq!(queue.dequeue().unwrap(), Some(5));
    assert!(queue.dequeue().unwrap().is_none());

    // Test checksum verification
    for i in 0..5 {
        assert!(queue.enqueue(i).is_ok());
    }

    assert!(queue.verify_checksum());

    // Test verification level setting
    queue.set_verification_level(VerificationLevel::Off);
    assert_eq!(queue.verification_level(), VerificationLevel::Off);

    queue.set_verification_level(VerificationLevel::Full);
    assert_eq!(queue.verification_level(), VerificationLevel::Full);
}

#[test]
fn test_bounded_map_operations() {
    let provider = safe_managed_alloc!(1024, CrateId::Foundation).unwrap();
    let mut map = BoundedMap::<u32, String, 5, NoStdProvider<1024>>::new(provider).unwrap();

    // Check empty map properties
    assert_eq!(map.len(), 0);
    assert_eq!(map.capacity(), 5);
    assert!(map.is_empty());
    assert!(!map.is_full());
    assert!(map.get(&1).unwrap().is_none());
    assert!(!map.contains_key(&1).unwrap());

    // Test insert operations
    for i in 0..5 {
        let value = format!("value-{}", i);
        assert_eq!(map.insert(i, value.clone()).unwrap(), None);
        assert_eq!(map.len(), i as usize + 1);
        assert_eq!(map.get(&i).unwrap(), Some(value));
    }

    // Test full map
    assert!(map.is_full());
    assert_eq!(
        map.insert(5, "overflow".to_string()).unwrap_err().kind(),
        BoundedErrorKind::CapacityExceeded
    );

    // Test update existing key
    assert_eq!(map.insert(2, "updated".to_string()).unwrap(), Some("value-2".to_string()));
    assert_eq!(map.get(&2).unwrap(), Some("updated".to_string()));
    assert_eq!(map.len(), 5); // Length unchanged after update

    // Test remove
    assert_eq!(map.remove(&3).unwrap(), Some("value-3".to_string()));
    assert_eq!(map.len(), 4);
    assert!(!map.contains_key(&3).unwrap());

    // Test insert after remove (should succeed now that we have space)
    assert_eq!(map.insert(5, "new-entry".to_string()).unwrap(), None);
    assert_eq!(map.len(), 5);
    assert_eq!(map.get(&5).unwrap(), Some("new-entry".to_string()));

    // Test clear
    assert!(map.clear().is_ok());
    assert!(map.is_empty());
    assert_eq!(map.len(), 0);

    // Test verification level setting
    map.set_verification_level(VerificationLevel::Off);
    assert_eq!(map.verification_level(), VerificationLevel::Off);

    map.set_verification_level(VerificationLevel::Full);
    assert_eq!(map.verification_level(), VerificationLevel::Full);
}

#[test]
fn test_bounded_set_operations() {
    let provider = NoStdProvider::new(1024, VerificationLevel::Critical);
    let mut set = BoundedSet::<String, 5, NoStdProvider<1024>>::new(provider).unwrap();

    // Check empty set properties
    assert_eq!(set.len(), 0);
    assert_eq!(set.capacity(), 5);
    assert!(set.is_empty());
    assert!(!set.is_full());
    assert!(!set.contains(&"item".to_string()).unwrap());

    // Test insert operations
    for i in 0..5 {
        let value = format!("item-{}", i);
        assert!(set.insert(value.clone()).unwrap());
        assert_eq!(set.len(), i as usize + 1);
        assert!(set.contains(&value).unwrap());
    }

    // Test full set
    assert!(set.is_full());
    assert_eq!(
        set.insert("overflow".to_string()).unwrap_err().kind(),
        BoundedErrorKind::CapacityExceeded
    );

    // Test insert duplicate (should return false without error)
    assert!(!set.insert("item-2".to_string()).unwrap());
    assert_eq!(set.len(), 5); // Length unchanged

    // Test remove
    assert!(set.remove(&"item-3".to_string()).unwrap());
    assert_eq!(set.len(), 4);
    assert!(!set.contains(&"item-3".to_string()).unwrap());

    // Test remove non-existent item (should return false without error)
    assert!(!set.remove(&"non-existent".to_string()).unwrap());

    // Test insert after remove (should succeed now that we have space)
    assert!(set.insert("new-item".to_string()).unwrap());
    assert_eq!(set.len(), 5);
    assert!(set.contains(&"new-item".to_string()).unwrap());

    // Test clear
    assert!(set.clear().is_ok());
    assert!(set.is_empty());
    assert_eq!(set.len(), 0);

    // Test verification level setting
    set.set_verification_level(VerificationLevel::Off);
    assert_eq!(set.verification_level(), VerificationLevel::Off);

    set.set_verification_level(VerificationLevel::Full);
    assert_eq!(set.verification_level(), VerificationLevel::Full);
}

#[test]
fn test_bounded_deque_operations() {
    let provider = NoStdProvider::new(1024, VerificationLevel::Critical);
    let mut deque = BoundedDeque::<u32, 5, NoStdProvider<1024>>::new(provider).unwrap();

    // Check empty deque properties
    assert_eq!(deque.len(), 0);
    assert_eq!(deque.capacity(), 5);
    assert!(deque.is_empty());
    assert!(!deque.is_full());
    assert!(deque.front().unwrap().is_none());
    assert!(deque.back().unwrap().is_none());
    assert!(deque.pop_front().unwrap().is_none());
    assert!(deque.pop_back().unwrap().is_none());

    // Test push_back operations
    for i in 0..3 {
        assert!(deque.push_back(i).is_ok());
        assert_eq!(deque.len(), i as usize + 1);
        assert_eq!(deque.back().unwrap(), Some(i));
    }

    // Test push_front operations
    assert!(deque.push_front(100).is_ok());
    assert!(deque.push_front(200).is_ok());
    assert_eq!(deque.len(), 5);

    // Test full deque
    assert!(deque.is_full());
    assert_eq!(deque.push_back(300).unwrap_err().kind(), BoundedErrorKind::CapacityExceeded);
    assert_eq!(deque.push_front(300).unwrap_err().kind(), BoundedErrorKind::CapacityExceeded);

    // Check deque state with front/back
    assert_eq!(deque.front().unwrap(), Some(200));
    assert_eq!(deque.back().unwrap(), Some(2));

    // Test pop_front operations
    assert_eq!(deque.pop_front().unwrap(), Some(200));
    assert_eq!(deque.pop_front().unwrap(), Some(100));
    assert_eq!(deque.len(), 3);
    assert_eq!(deque.front().unwrap(), Some(0));

    // Test pop_back operations
    assert_eq!(deque.pop_back().unwrap(), Some(2));
    assert_eq!(deque.pop_back().unwrap(), Some(1));
    assert_eq!(deque.len(), 1);
    assert_eq!(deque.back().unwrap(), Some(0));

    // Test last element and empty deque
    assert_eq!(deque.pop_front().unwrap(), Some(0));
    assert!(deque.is_empty());
    assert!(deque.pop_front().unwrap().is_none());
    assert!(deque.pop_back().unwrap().is_none());

    // Test alternating push/pop operations for wrap-around behavior
    for i in 0..5 {
        if i % 2 == 0 {
            assert!(deque.push_front(i).is_ok());
        } else {
            assert!(deque.push_back(i).is_ok());
        }
    }

    assert_eq!(deque.len(), 5);
    assert!(deque.is_full());

    // Verify elements in expected order: 4, 2, 0, 1, 3
    assert_eq!(deque.pop_front().unwrap(), Some(4));
    assert_eq!(deque.pop_front().unwrap(), Some(2));
    assert_eq!(deque.pop_front().unwrap(), Some(0));
    assert_eq!(deque.pop_front().unwrap(), Some(1));
    assert_eq!(deque.pop_front().unwrap(), Some(3));
    assert!(deque.is_empty());

    // Test clear
    for i in 0..3 {
        assert!(deque.push_back(i).is_ok());
    }
    assert_eq!(deque.len(), 3);

    assert!(deque.clear().is_ok());
    assert!(deque.is_empty());
    assert_eq!(deque.len(), 0);

    // Test checksum verification
    for i in 0..5 {
        assert!(deque.push_back(i).is_ok());
    }

    assert!(deque.verify_checksum());

    // Test verification level setting
    deque.set_verification_level(VerificationLevel::Off);
    assert_eq!(deque.verification_level(), VerificationLevel::Off);

    deque.set_verification_level(VerificationLevel::Full);
    assert_eq!(deque.verification_level(), VerificationLevel::Full);
}

#[test]
fn test_bounded_bitset_operations() {
    let mut bitset = BoundedBitSet::<100>::new();

    // Check empty bitset properties
    assert_eq!(bitset.len(), 0);
    assert_eq!(bitset.capacity(), 100);
    assert!(bitset.is_empty());
    assert!(!bitset.is_full());

    // Test set operations
    for i in 0..50 {
        assert!(bitset.set(i).unwrap());
        assert_eq!(bitset.len(), i as usize + 1);
        assert!(bitset.contains(i).unwrap());
    }

    // Test set already set bit (should return false without error)
    assert!(!bitset.set(25).unwrap());
    assert_eq!(bitset.len(), 50); // Length unchanged

    // Test clear operations
    for i in 0..25 {
        assert!(bitset.clear(i).unwrap());
        assert_eq!(bitset.len(), 49 - i as usize);
        assert!(!bitset.contains(i).unwrap());
    }

    // Test clear already cleared bit (should return false without error)
    assert!(!bitset.clear(10).unwrap());
    assert_eq!(bitset.len(), 25); // Length unchanged

    // Test toggle operations
    for i in 0..10 {
        let was_set = bitset.contains(i).unwrap();
        let is_now_set = bitset.toggle(i).unwrap();
        assert_ne!(was_set, is_now_set);
        assert_eq!(bitset.contains(i).unwrap(), is_now_set);
    }

    // Test out of bounds access
    assert!(bitset.set(100).is_err());
    assert!(bitset.clear(100).is_err());
    assert!(bitset.contains(100).is_err());
    assert!(bitset.toggle(100).is_err());

    // Test set_all and clear_all
    bitset.set_all();
    assert_eq!(bitset.len(), 100);
    assert!(bitset.is_full());

    for i in 0..100 {
        assert!(bitset.contains(i).unwrap());
    }

    bitset.clear_all();
    assert_eq!(bitset.len(), 0);
    assert!(bitset.is_empty());

    for i in 0..100 {
        assert!(!bitset.contains(i).unwrap());
    }

    // Test checksum verification
    for i in 0..50 {
        assert!(bitset.set(i).unwrap());
    }

    assert!(bitset.verify_checksum());

    // Test verification level setting
    bitset.set_verification_level(VerificationLevel::Off);
    assert_eq!(bitset.verification_level(), VerificationLevel::Off);

    bitset.set_verification_level(VerificationLevel::Full);
    assert_eq!(bitset.verification_level(), VerificationLevel::Full);
}

#[test]
fn test_bounded_builder_pattern() {
    // Test BoundedBuilder for BoundedVec
    let vec_builder = BoundedBuilder::<u32, 10, NoStdProvider<1024>>::new()
        .with_verification_level(VerificationLevel::Critical);

    let mut vec = vec_builder.build_vec().unwrap();
    assert_eq!(vec.capacity(), 10);
    assert_eq!(vec.verification_level(), VerificationLevel::Critical);

    // Test BoundedBuilder for BoundedStack
    let stack_builder = BoundedBuilder::<u32, 20, NoStdProvider<1024>>::new()
        .with_verification_level(VerificationLevel::Full);

    let stack = stack_builder.build_stack().unwrap();
    assert_eq!(stack.capacity(), 20);
    assert_eq!(stack.verification_level(), VerificationLevel::Full);

    // Test StringBuilder for BoundedString
    let string_builder = StringBuilder::<128, NoStdProvider<1024>>::new()
        .with_content("Hello, world!")
        .with_truncation(true);

    let string = string_builder.build_string().unwrap();
    assert_eq!(string.as_str().unwrap(), "Hello, world!");

    // Test StringBuilder for WasmName
    let name_builder = StringBuilder::<64, NoStdProvider<1024>>::new()
        .with_content("function_name")
        .with_truncation(false);

    let name = name_builder.build_wasm_name().unwrap();
    assert_eq!(name.as_str().unwrap(), "function_name");

    // Test modern provider factory - using safe_managed_alloc for simplicity
    let provider = safe_managed_alloc!(4096, CrateId::Foundation).unwrap();
    assert!(provider.capacity() <= 4096); // Capacity may be capped
    // Note: verification level testing would need provider API enhancement

    // Test MemoryBuilder
    let memory_builder = MemoryBuilder::<NoStdProvider<1024>>::new()
        .with_size(2048)
        .with_verification_level(VerificationLevel::Full);

    let memory_handler = memory_builder.build_safe_memory_handler().unwrap();
    assert_eq!(memory_handler.verification_level(), VerificationLevel::Full);
}

#[test]
fn test_interoperability() {
    // Test interoperability between different bounded collections

    // Build a BoundedMap using safe modern allocation
    let provider = safe_managed_alloc!(1024, CrateId::Foundation).unwrap();

    let mut map = BoundedMap::<u32, String, 5, NoStdProvider<1024>>::new(provider).unwrap();

    // Populate map with values from StringBuilder
    for i in 0..5 {
        let string_builder = StringBuilder::<64, NoStdProvider<1024>>::new()
            .with_content(match i {
                0 => "zero",
                1 => "one",
                2 => "two",
                3 => "three",
                _ => "four",
            })
            .with_truncation(true);

        let string = string_builder.build_string().unwrap();
        map.insert(i, string.as_str().unwrap().to_string()).unwrap();
    }

    // Verify map contents
    assert_eq!(map.len(), 5);
    assert!(map.is_full());

    for i in 0..5 {
        let expected = match i {
            0 => "zero",
            1 => "one",
            2 => "two",
            3 => "three",
            _ => "four",
        };

        assert_eq!(map.get(&i).unwrap(), Some(expected.to_string()));
    }

    // Test using BoundedSet with BoundedQueue
    let mut set = BoundedSet::<u32, 10, NoStdProvider<1024>>::new(provider).unwrap();
    let mut queue = BoundedQueue::<u32, 10, NoStdProvider<1024>>::new(provider).unwrap();

    // Add values to queue
    for i in 0..8 {
        queue.enqueue(i).unwrap();
    }

    // Dequeue items and add to set (will filter duplicates)
    while let Some(value) = queue.dequeue().unwrap() {
        set.insert(value % 5).unwrap(); // Creates duplicates with modulo
    }

    // Verify set has only unique values
    assert_eq!(set.len(), 5); // Only 0,1,2,3,4 due to modulo

    for i in 0..5 {
        assert!(set.contains(&i).unwrap());
    }
}

// Add performance benchmark test if the test environment supports it
#[cfg(feature = "std")]
#[test]
fn test_bounded_collections_performance() {
    use std::time::{Duration, Instant};

    // Create large collections
    let mut deque = BoundedDeque::<u32, 10_000, NoStdProvider<4_194_304>>::new(
        NoStdProvider::new(4 * 1024 * 1024, VerificationLevel::Critical), // 4MB buffer
    )
    .unwrap();

    let mut bitset = BoundedBitSet::<100_000>::new();

    // Measure deque performance
    let start = Instant::now();

    for i in 0..5000 {
        deque.push_back(i).unwrap();
    }

    for _ in 0..2500 {
        deque.pop_front().unwrap();
    }

    for i in 0..2500 {
        deque.push_front(i).unwrap();
    }

    let deque_duration = start.elapsed();

    // Measure bitset performance
    let start = Instant::now();

    for i in 0..50_000 {
        bitset.set(i % 100_000).unwrap();
    }

    for i in 0..25_000 {
        bitset.clear(i % 100_000).unwrap();
    }

    for i in 0..10_000 {
        bitset.toggle(i % 100_000).unwrap();
    }

    let bitset_duration = start.elapsed();

    // Print performance results (not checking specific values)
    println!("BoundedDeque operations took: {:?}", deque_duration);
    println!("BoundedBitSet operations took: {:?}", bitset_duration);

    // Just assert that operations completed in reasonable time
    assert!(deque_duration < Duration::from_secs(1));
    assert!(bitset_duration < Duration::from_secs(1));
}
