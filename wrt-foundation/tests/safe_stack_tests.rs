//! Tests for SafeStack

#![cfg(all(test,))] // Binary std/no_std choice

// This import is for no_std, but if no no_std tests use it from this file, it
// might still be warned.
#[cfg(not(feature = "std"))]
use wrt_foundation::safe_memory::NoStdMemoryProvider;
// Commented out unused imports from original warnings
// use wrt_foundation::prelude::Checksummed;
// use wrt_foundation::validation::BoundedCapacity;
// use wrt_foundation::verification::VerificationLevel; // Was part of verification::{Checksum,
// VerificationLevel} const U32_SIZE: usize = core::mem::size_of::<u32>();
#[cfg(feature = "std")]
use wrt_foundation::safe_memory::StdMemoryProvider;
#[cfg(feature = "std")]
use wrt_foundation::{
    bounded::{BoundedStack, CapacityError /* , CHECKSUM_SIZE */},
    safe_memory::MemoryProvider,
    WrtResult,
};
// Imports presumably used by TestValue impls (Checksummable, ToBytes, FromBytes) which are not
// std-gated
use wrt_foundation::{
    traits::{Checksummable, FromBytes, SerializationError, ToBytes},
    verification::Checksum,
};

#[derive(Debug, Clone, PartialEq)]
struct TestValue {
    id: u32,
    data: u32,
}

impl Checksummable for TestValue {
    fn update_checksum(&self, checksum: &mut Checksum) {
        checksum.update_slice(&self.id.to_le_bytes());
        checksum.update_slice(&self.data.to_le_bytes());
    }
}

impl ToBytes for TestValue {
    const SERIALIZED_SIZE: usize = 8;
    fn write_bytes(&self, buffer: &mut [u8]) -> core::result::Result<(), SerializationError> {
        if buffer.len() < <Self as ToBytes>::SERIALIZED_SIZE {
            return Err(SerializationError::IncorrectSize);
        }
        buffer[0..4].copy_from_slice(&self.id.to_le_bytes());
        buffer[4..8].copy_from_slice(&self.data.to_le_bytes());
        Ok(())
    }
}

impl FromBytes for TestValue {
    const SERIALIZED_SIZE: usize = 8;
    fn from_bytes(buffer: &[u8]) -> core::result::Result<Self, SerializationError> {
        if buffer.len() < <Self as FromBytes>::SERIALIZED_SIZE {
            return Err(SerializationError::IncorrectSize);
        }
        let id_bytes: [u8; 4] =
            buffer[0..4].try_into().map_err(|_| SerializationError::InvalidFormat)?;
        let data_bytes: [u8; 4] =
            buffer[4..8].try_into().map_err(|_| SerializationError::InvalidFormat)?;
        let id = u32::from_le_bytes(id_bytes);
        let data = u32::from_le_bytes(data_bytes);
        Ok(TestValue { id, data })
    }
}

#[test]
#[cfg(feature = "std")]
fn test_safe_stack_creation_and_basic_ops() {
    const CAPACITY_ELEMENTS: usize = 10;
    const ITEM_SIZE: usize = <TestValue as ToBytes>::SERIALIZED_SIZE; // 8 bytes
    const REQUIRED_BYTES: usize = CAPACITY_ELEMENTS * ITEM_SIZE + CHECKSUM_SIZE;

    let mut stack = BoundedStack::<TestValue, CAPACITY_ELEMENTS, StdMemoryProvider>::new(
        StdMemoryProvider::new(vec![0u8; REQUIRED_BYTES]), // Initialize with sized Vec
    )
    .unwrap();

    let value1 = TestValue { id: 1, data: 100 };
    stack.push(value1.clone()).unwrap();
    let value2 = TestValue { id: 2, data: 200 };
    stack.push(value2.clone()).unwrap();
    let value3 = TestValue { id: 3, data: 300 };
    stack.push(value3.clone()).unwrap();

    assert_eq!(stack.len(), 3);
    assert!(!stack.is_empty());

    let popped3 = stack.pop().unwrap();
    assert_eq!(popped3, Some(value3));

    let popped2 = stack.pop().unwrap();
    assert_eq!(popped2, Some(value2));

    assert_eq!(stack.peek().expect("Peek failed on stack"), value1.clone());
    assert_eq!(stack.len(), 1);

    while stack.pop().unwrap().is_some() {}
    assert!(stack.is_empty());
}

#[test]
#[cfg(feature = "std")]
fn test_safe_stack_verification_levels_push_pop() {
    const CAPACITY_ELEMENTS: usize = 10;
    const ITEM_SIZE: usize = U32_SIZE; // 4 bytes
    const REQUIRED_BYTES_PER_STACK: usize = CAPACITY_ELEMENTS * ITEM_SIZE + CHECKSUM_SIZE;

    let mut stack_none = BoundedStack::<u32, CAPACITY_ELEMENTS, _>::new(StdMemoryProvider::new(
        vec![0u8; REQUIRED_BYTES_PER_STACK],
    ))
    .unwrap();
    stack_none.set_verification_level(VerificationLevel::Off);

    let mut stack_sampling = BoundedStack::<u32, CAPACITY_ELEMENTS, _>::new(
        StdMemoryProvider::new(vec![0u8; REQUIRED_BYTES_PER_STACK]),
    )
    .unwrap();
    stack_sampling.set_verification_level(VerificationLevel::default()); // Sampling

    let mut stack_full = BoundedStack::<u32, CAPACITY_ELEMENTS, _>::new(StdMemoryProvider::new(
        vec![0u8; REQUIRED_BYTES_PER_STACK],
    ))
    .unwrap();
    stack_full.set_verification_level(VerificationLevel::Full);

    for i in 0..CAPACITY_ELEMENTS as u32 {
        stack_none.push(i).unwrap();
        stack_sampling.push(i).unwrap();
        stack_full.push(i).unwrap();
    }

    for i in (0..CAPACITY_ELEMENTS as u32).rev() {
        assert_eq!(stack_none.pop().unwrap(), Some(i));
        assert_eq!(stack_sampling.pop().unwrap(), Some(i));
        assert_eq!(stack_full.pop().unwrap(), Some(i));
    }

    assert!(stack_none.is_empty());
    assert!(stack_sampling.is_empty());
    assert!(stack_full.is_empty());
}

#[test]
#[cfg(feature = "std")]
fn test_safe_stack_capacity_and_errors() {
    const CAPACITY_ELEMENTS: usize = 5;
    const ITEM_SIZE: usize = U32_SIZE;
    const REQUIRED_BYTES: usize = CAPACITY_ELEMENTS * ITEM_SIZE + CHECKSUM_SIZE;

    let mut stack = BoundedStack::<u32, CAPACITY_ELEMENTS, StdMemoryProvider>::new(
        StdMemoryProvider::new(vec![0u8; REQUIRED_BYTES]),
    )
    .unwrap();

    assert!(stack.pop().unwrap().is_none());
    assert!(stack.peek().is_none());

    for i in 0..CAPACITY_ELEMENTS as u32 {
        stack.push(i).unwrap();
    }
    assert_eq!(stack.len(), CAPACITY_ELEMENTS);
    assert!(stack.is_full());
    assert!(stack.push(CAPACITY_ELEMENTS as u32).is_err());

    for _ in 0..CAPACITY_ELEMENTS {
        stack.pop().unwrap();
    }
    assert!(stack.is_empty());
}

#[test]
#[cfg(feature = "std")]
fn test_safe_stack_integrity_mixed_ops_levels() {
    const CAPACITY_ELEMENTS: usize = 20;
    const ITEM_SIZE: usize = U32_SIZE;
    const REQUIRED_BYTES_PER_STACK: usize = CAPACITY_ELEMENTS * ITEM_SIZE + CHECKSUM_SIZE;

    let mut stack_none = BoundedStack::<u32, CAPACITY_ELEMENTS, _>::new(StdMemoryProvider::new(
        vec![0u8; REQUIRED_BYTES_PER_STACK],
    ))
    .unwrap();
    stack_none.set_verification_level(VerificationLevel::Off);

    let mut stack_sampling = BoundedStack::<u32, CAPACITY_ELEMENTS, _>::new(
        StdMemoryProvider::new(vec![0u8; REQUIRED_BYTES_PER_STACK]),
    )
    .unwrap();
    stack_sampling.set_verification_level(VerificationLevel::default());

    let mut stack_full = BoundedStack::<u32, CAPACITY_ELEMENTS, _>::new(StdMemoryProvider::new(
        vec![0u8; REQUIRED_BYTES_PER_STACK],
    ))
    .unwrap();
    stack_full.set_verification_level(VerificationLevel::Full);

    for i in 0..10_u32 {
        stack_none.push(i).unwrap();
        stack_sampling.push(i).unwrap();
        stack_full.push(i).unwrap();
    }
    assert_eq!(stack_none.len(), 10);
    assert_eq!(stack_sampling.len(), 10);
    assert_eq!(stack_full.len(), 10);
    assert_eq!(stack_full.peek().expect("Peek failed"), 9_u32);
    assert_eq!(stack_full.pop().unwrap(), Some(9_u32));
    assert_eq!(stack_sampling.pop().unwrap(), Some(9_u32));
    assert_eq!(stack_none.pop().unwrap(), Some(9_u32));
    assert_eq!(stack_none.len(), 9);

    for i in 9..CAPACITY_ELEMENTS as u32 {
        stack_none.push(i).unwrap();
        stack_sampling.push(i).unwrap();
        stack_full.push(i).unwrap();
    }
    assert!(stack_full.is_full());
    assert!(stack_full.push(CAPACITY_ELEMENTS as u32).is_err());
    assert!(stack_sampling.push(CAPACITY_ELEMENTS as u32).is_err());
    assert!(stack_none.push(CAPACITY_ELEMENTS as u32).is_err());

    for _ in 0..CAPACITY_ELEMENTS {
        assert!(stack_none.pop().unwrap().is_some());
        assert!(stack_sampling.pop().unwrap().is_some());
        assert!(stack_full.pop().unwrap().is_some());
    }
    assert!(stack_none.is_empty());
    assert!(stack_sampling.is_empty());
    assert!(stack_full.is_empty());
}

#[test]
#[cfg(feature = "std")]
fn test_safe_stack_checksum_logic() {
    const CAPACITY_ELEMENTS: usize = 3;
    const ITEM_SIZE: usize = U32_SIZE;
    const REQUIRED_BYTES_PER_STACK: usize = CAPACITY_ELEMENTS * ITEM_SIZE + CHECKSUM_SIZE;

    let mut stack = BoundedStack::<u32, CAPACITY_ELEMENTS, _>::new(StdMemoryProvider::new(
        vec![0u8; REQUIRED_BYTES_PER_STACK],
    ))
    .unwrap();
    stack.set_verification_level(VerificationLevel::Off);

    for i in 0..CAPACITY_ELEMENTS as u32 {
        stack.push(i).unwrap();
    }
    let checksum_off = stack.checksum();

    stack.set_verification_level(VerificationLevel::Full);
    stack.recalculate_checksum();
    let checksum_on_initial = stack.checksum();
    if CAPACITY_ELEMENTS > 0 {
        assert_ne!(
            checksum_off, checksum_on_initial,
            "Checksum should change after recalculation with verification on if stack was not \
             empty"
        );
    }

    if CAPACITY_ELEMENTS > 0 {
        stack.pop().unwrap();
        let checksum_after_pop = stack.checksum();
        assert_ne!(
            checksum_on_initial, checksum_after_pop,
            "Checksum should change after pop if stack was not empty"
        );

        stack.push(100).unwrap();
        let checksum_after_push = stack.checksum();
        assert_ne!(
            checksum_after_pop, checksum_after_push,
            "Checksum should change after push if stack was not full"
        );
    }

    while stack.pop().unwrap().is_some() {}
    assert!(stack.is_empty());
    let checksum_empty_verified = stack.checksum();

    let mut empty_stack_verified = BoundedStack::<u32, CAPACITY_ELEMENTS, _>::new(
        StdMemoryProvider::new(vec![0u8; REQUIRED_BYTES_PER_STACK]),
    )
    .unwrap();
    empty_stack_verified.set_verification_level(VerificationLevel::Full);
    assert_eq!(
        checksum_empty_verified,
        empty_stack_verified.checksum(),
        "Checksum of cleared stack should match newly created verified empty stack"
    );
}
