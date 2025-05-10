//! Tests for SafeStack functionality in `wrt_types`.
use wrt_types::bounded::BoundedStack;
use wrt_types::traits::Checksummable;
use wrt_types::validation::BoundedCapacity;
use wrt_types::verification::Checksum;
use wrt_types::verification::VerificationLevel;

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

#[test]
fn test_safe_stack_as_vec_replacement() {
    const CAPACITY: usize = 10;
    let mut stack = BoundedStack::<TestValue, CAPACITY>::new();

    let value1 = TestValue { id: 1, data: 100 };
    stack.push(value1.clone()).unwrap();
    let value2 = TestValue { id: 2, data: 200 };
    stack.push(value2.clone()).unwrap();
    let value3 = TestValue { id: 3, data: 300 };
    stack.push(value3.clone()).unwrap();

    assert_eq!(stack.len(), 3);
    assert!(!stack.is_empty());

    let popped3 = stack.pop().unwrap();
    assert_eq!(popped3, value3);

    let popped2 = stack.pop().unwrap();
    assert_eq!(popped2, value2);

    let top = stack.peek().unwrap();
    assert_eq!(*top, value1);
    assert_eq!(stack.len(), 1);

    while stack.pop().is_some() {}
    assert!(stack.is_empty());
}

#[test]
fn test_safe_stack_with_different_verification_levels() {
    const CAPACITY: usize = 10;
    let mut stack_none = BoundedStack::<u32, CAPACITY>::new();
    stack_none.set_verification_level(VerificationLevel::None);

    let mut stack_standard = BoundedStack::<u32, CAPACITY>::new();
    stack_standard.set_verification_level(VerificationLevel::Standard);

    let mut stack_full = BoundedStack::<u32, CAPACITY>::new();
    stack_full.set_verification_level(VerificationLevel::Full);

    for i in 0..CAPACITY as u32 {
        stack_none.push(i).unwrap();
        stack_standard.push(i).unwrap();
        stack_full.push(i).unwrap();
    }

    for i in (0..CAPACITY as u32).rev() {
        assert_eq!(stack_none.pop().unwrap(), i);
        assert_eq!(stack_standard.pop().unwrap(), i);
        assert_eq!(stack_full.pop().unwrap(), i);
    }

    assert!(stack_none.is_empty());
    assert!(stack_standard.is_empty());
    assert!(stack_full.is_empty());
}

#[test]
#[cfg(feature = "std")]
fn test_safe_stack_to_vec_conversion() {
    const CAPACITY: usize = 5;
    let mut stack = BoundedStack::<u32, CAPACITY>::new();

    for i in 0..CAPACITY as u32 {
        stack.push(i).unwrap();
    }

    assert_eq!(stack.len(), CAPACITY);
}

#[test]
#[cfg(feature = "std")]
fn test_safe_stack_error_handling() {
    const CAPACITY: usize = 5;
    let mut stack = BoundedStack::<u32, CAPACITY>::new();

    assert!(stack.pop().is_none());

    for i in 0..CAPACITY as u32 {
        stack.push(i).unwrap();
    }
    assert!(stack.push(CAPACITY as u32).is_err());

    for _ in 0..CAPACITY {
        stack.pop().unwrap();
    }

    assert!(stack.is_empty());
}

#[test]
fn test_safe_stack_fixed() {
    const CAPACITY: usize = 20;
    let mut stack_none = BoundedStack::<u32, CAPACITY>::new();
    let mut stack_standard = BoundedStack::<u32, CAPACITY>::new();
    let mut stack_full = BoundedStack::<u32, CAPACITY>::new();

    stack_none.set_verification_level(VerificationLevel::None);
    stack_standard.set_verification_level(VerificationLevel::Standard);
    stack_full.set_verification_level(VerificationLevel::Full);

    for i in 0..10 {
        stack_none.push(i).unwrap();
        stack_standard.push(i).unwrap();
        stack_full.push(i).unwrap();
    }

    assert_eq!(stack_none.len(), 10);
    assert_eq!(stack_standard.len(), 10);
    assert_eq!(stack_full.len(), 10);

    assert_eq!(*stack_none.peek().unwrap(), 9);
    assert_eq!(*stack_standard.peek().unwrap(), 9);
    assert_eq!(*stack_full.peek().unwrap(), 9);

    assert_eq!(stack_none.pop().unwrap(), 9);
    assert_eq!(stack_standard.pop().unwrap(), 9);
    assert_eq!(stack_full.pop().unwrap(), 9);

    assert_eq!(stack_none.len(), 9);
    assert_eq!(stack_standard.len(), 9);
    assert_eq!(stack_full.len(), 9);

    for i in 9..CAPACITY as u32 {
        stack_none.push(i).unwrap();
        stack_standard.push(i).unwrap();
        stack_full.push(i).unwrap();
    }
    assert!(stack_none.push(CAPACITY as u32).is_err());
    assert!(stack_standard.push(CAPACITY as u32).is_err());
    assert!(stack_full.push(CAPACITY as u32).is_err());
}

#[test]
fn test_safe_stack_dynamic() {
    const CAPACITY: usize = 20;
    let mut stack_none = BoundedStack::<u32, CAPACITY>::new();
    let mut stack_standard = BoundedStack::<u32, CAPACITY>::new();
    let mut stack_full = BoundedStack::<u32, CAPACITY>::new();

    stack_none.set_verification_level(VerificationLevel::None);
    stack_standard.set_verification_level(VerificationLevel::Standard);
    stack_full.set_verification_level(VerificationLevel::Full);

    for i in 0..CAPACITY {
        stack_none.push(i).unwrap();
        stack_standard.push(i).unwrap();
        stack_full.push(i).unwrap();
    }

    assert_eq!(*stack_full.peek().unwrap(), (CAPACITY - 1) as u32);
    for i in (0..CAPACITY).rev() {
        assert_eq!(stack_none.pop().unwrap(), i as u32);
        assert_eq!(stack_standard.pop().unwrap(), i as u32);
        assert_eq!(stack_full.pop().unwrap(), i as u32);
    }

    assert!(stack_none.is_empty());
    assert!(stack_standard.is_empty());
    assert!(stack_full.is_empty());
}

#[test]
fn test_safe_stack_error_cases() {
    const CAPACITY: usize = 5;
    let mut stack = BoundedStack::<u32, CAPACITY>::new();
    for i in 0..CAPACITY as u32 {
        stack.push(i).unwrap();
    }

    for _ in 0..CAPACITY {
        assert!(stack.pop().is_some());
    }
    assert!(stack.pop().is_none());
    assert!(stack.peek().is_none());
}

#[test]
fn test_safe_stack_conversions() {
    const CAPACITY: usize = 10;
    let mut stack = BoundedStack::<u32, CAPACITY>::new();
    for i in 0..CAPACITY as u32 {
        stack.push(i).unwrap();
    }
}
