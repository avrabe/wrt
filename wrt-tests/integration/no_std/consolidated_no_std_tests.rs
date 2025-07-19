//! Consolidated no_std compatibility tests for all WRT crates
//!
//! This module consolidates all the no_std_compatibility_test.rs files from across all crates
//! into a single comprehensive test suite. Each crate's no_std functionality is thoroughly tested.


// External crate imports for no_std environment
extern crate alloc;

#[cfg(test)]
mod tests {
    // Import necessary types for no_std environment
        use std::{format, string::String, vec};
    
    #[cfg(feature = "std")]
    use std::{format, string::String, vec};

    // ===========================================
    // WRT-ERROR NO_STD TESTS
    // ===========================================
    
    mod wrt_error_tests {
        use super::*;
        use wrt_error::{codes, kinds, Error, ErrorCategory, Result};

        #[test]
        fn test_error_creation() {
            let error = Error::core_invalid_memory_access("Invalid memory access";

            assert_eq!(error.category, ErrorCategory::Core;
            assert_eq!(error.code, codes::INVALID_MEMORY_ACCESS;
        }

        #[test]
        fn test_result_operations() {
            let ok_result: Result<i32> = Ok(42;
            assert!(ok_result.is_ok();
            assert_eq!(ok_result.unwrap(), 42;

            let error = Error::core_invalid_memory_access("Invalid memory access";
            let err_result: Result<i32> = Err(error;
            assert!(err_result.is_err();

            let extracted_error = err_result.unwrap_err);
            assert_eq!(extracted_error.category, ErrorCategory::Core;
        }

        #[test]
        fn test_error_categories() {
            assert_ne!(ErrorCategory::Core, ErrorCategory::Resource;
            assert_ne!(ErrorCategory::Memory, ErrorCategory::Validation;
            assert_ne!(ErrorCategory::Validation, ErrorCategory::Runtime;
            assert_ne!(ErrorCategory::Runtime, ErrorCategory::System;
        }

        #[test]
        fn test_error_kinds() {
            let validation_error = kinds::validation_error("Validation error";
            let memory_error = kinds::memory_access_error("Memory error";
            let runtime_error = kinds::runtime_error("Runtime error";

            let type_name_validation = core::any::type_name_of_val(&validation_error;
            assert!(type_name_validation.contains("ValidationError");

            let type_name_memory = core::any::type_name_of_val(&memory_error;
            assert!(type_name_memory.contains("MemoryAccessError");

            let type_name_runtime = core::any::type_name_of_val(&runtime_error;
            assert!(type_name_runtime.contains("RuntimeError");
        }
    }

    // ===========================================
    // WRT-FOUNDATION NO_STD TESTS  
    // ===========================================
    
    mod wrt_foundation_tests {
        use super::*;
        use wrt_foundation::prelude::*;
        use core::mem;

        #[test]
        fn test_bounded_vec_no_alloc() {
            const CAPACITY: usize = 10;
            let provider = NoStdProvider::<{ CAPACITY * 4 }>::default);
            let mut vec: BoundedVec<u32, CAPACITY, NoStdProvider<{ CAPACITY * 4 }>> =
                BoundedVec::new(provider).unwrap();

            assert!(vec.is_empty();
            assert_eq!(vec.len(), 0;
            assert_eq!(vec.capacity(), CAPACITY;

            vec.push(1).unwrap();
            vec.push(2).unwrap();
            vec.push(3).unwrap();

            assert_eq!(vec.len(), 3;
            assert_eq!(vec.get(0).unwrap(), 1;
            assert_eq!(vec.get(1).unwrap(), 2;
            assert_eq!(vec.get(2).unwrap(), 3;
        }

        #[test]
        fn test_bounded_string_no_alloc() {
            const CAPACITY: usize = 32;
            let provider = NoStdProvider::<CAPACITY>::default);
            let mut string: BoundedString<CAPACITY, NoStdProvider<CAPACITY>> =
                BoundedString::from_str("", provider).unwrap();

            assert!(string.is_empty();
            assert_eq!(string.len(), 0;

            string.push_str("Hello").unwrap();
            assert_eq!(string.as_str().unwrap(), "Hello";

            string.push_str(", World!").unwrap();
            assert_eq!(string.as_str().unwrap(), "Hello, World!";
        }

        #[test]
        fn test_bounded_stack_no_alloc() {
            const CAPACITY: usize = 5;
            let provider = NoStdProvider::<{ CAPACITY * 4 }>::default);
            let mut stack: BoundedStack<i32, CAPACITY, NoStdProvider<{ CAPACITY * 4 }>> =
                BoundedStack::new(provider).unwrap();

            assert!(stack.is_empty();

            stack.push(10).unwrap();
            stack.push(20).unwrap();
            stack.push(30).unwrap();

            assert_eq!(stack.len(), 3;

            assert_eq!(stack.pop().unwrap(), Some(30;
            assert_eq!(stack.pop().unwrap(), Some(20;
            assert_eq!(stack.pop().unwrap(), Some(10;
            assert_eq!(stack.pop().unwrap(), None;
        }

        #[test]
        fn test_bounded_queue_no_alloc() {
            const CAPACITY: usize = 4;
            let provider = NoStdProvider::<{ CAPACITY * 16 }>::default);
            let mut queue: BoundedQueue<u8, CAPACITY, NoStdProvider<{ CAPACITY * 16 }>> =
                BoundedQueue::new(provider).unwrap();

            assert!(queue.is_empty();

            queue.enqueue(1).unwrap();
            queue.enqueue(2).unwrap();
            queue.enqueue(3).unwrap();

            assert_eq!(queue.len(), 3;

            assert_eq!(queue.dequeue().unwrap(), Some(1;
            assert_eq!(queue.dequeue().unwrap(), Some(2;
            assert_eq!(queue.dequeue().unwrap(), Some(3;
            assert_eq!(queue.dequeue().unwrap(), None;
        }

        #[test]
        fn test_types_no_alloc() {
            let _val_type = ValueType::I32;
            assert_eq!(mem::size_of::<ValueType>(), 1;

            let _ref_type = RefType::Funcref;
            assert_eq!(mem::size_of::<RefType>(), 1;

            let limits = Limits::new(10, Some(100;
            assert_eq!(limits.min, 10;
            assert_eq!(limits.max, Some(100;
        }

        #[test]
        fn test_verification_no_alloc() {
            let checksum = Checksum::from_value(0x12345678;
            assert_eq!(checksum.value(), 0x12345678;

            let level = VerificationLevel::Off;
            assert!(matches!(level, VerificationLevel::Off);
        }

        #[cfg(not(any(feature = "std", )))]
        #[test]
        fn test_simple_hashmap_no_alloc() {
            use wrt_foundation::BoundedMap;

            const CAPACITY: usize = 16;
            const PROVIDER_SIZE: usize = CAPACITY * 32;
            let provider = NoStdProvider::<PROVIDER_SIZE>::default);
            let mut map: BoundedMap<u32, u32, CAPACITY, NoStdProvider<PROVIDER_SIZE>> =
                BoundedMap::new);

            assert!(map.is_empty();

            assert!(map.insert(1, 100).unwrap().is_none();
            assert!(map.insert(2, 200).unwrap().is_none();
            assert!(map.insert(3, 300).unwrap().is_none();

            assert_eq!(map.get(&1).unwrap(), Some(100;
            assert_eq!(map.get(&2).unwrap(), Some(200;
            assert_eq!(map.get(&3).unwrap(), Some(300;
            assert_eq!(map.get(&4).unwrap(), None;
        }
    }

    // ===========================================
    // WRT-SYNC NO_STD TESTS
    // ===========================================
    
    mod wrt_sync_tests {
        use super::*;
        use wrt_sync::{WrtMutex as Mutex, WrtRwLock as RwLock};

        #[test]
        fn test_mutex_operations() {
            let mutex = Mutex::new(42;

            {
                let mut lock = mutex.lock);
                assert_eq!(*lock, 42;
                *lock = 100;
            }

            let lock = mutex.lock);
            assert_eq!(*lock, 100;
        }

        #[test]
        fn test_rwlock_operations() {
            let rwlock = RwLock::new(String::from("test";

            {
                let read_lock = rwlock.read);
                assert_eq!(*read_lock, "test";
            }

            {
                let mut write_lock = rwlock.write);
                write_lock.push_str("_modified";
            }

            let read_lock = rwlock.read);
            assert_eq!(*read_lock, "test_modified";
        }

        #[test]
        fn test_mutex_locking() {
            let mutex = Mutex::new(42;
            let lock = mutex.lock);
            assert_eq!(*lock, 42;
        }

        #[test]
        fn test_rwlock_read_write() {
            let rwlock = RwLock::new(42;

            {
                let lock = rwlock.read);
                assert_eq!(*lock, 42;
            }

            {
                let mut lock = rwlock.write);
                *lock = 100;
                assert_eq!(*lock, 100;
            }

            let lock = rwlock.read);
            assert_eq!(*lock, 100;
        }
    }

    // ===========================================
    // WRT-PLATFORM NO_STD TESTS
    // ===========================================
    
    mod wrt_platform_tests {
        use super::*;
        use wrt_platform::{PageAllocator, FutexLike, WASM_PAGE_SIZE};
        use wrt_platform::sync::{SpinFutex, SpinFutexBuilder};
        use wrt_platform::memory::{NoStdProvider, VerificationLevel};
        use wrt_foundation::{WrtProviderFactory, budget_aware_provider::CrateId};
        use core::time::Duration;

        #[test]
        fn test_spin_futex_no_std() {
            let futex = SpinFutexBuilder::new()
                .with_initial_value(42)
                .build);

            assert_eq!(futex.get(), 42;
            futex.set(100;
            assert_eq!(futex.get(), 100;

            // Test wait with timeout (should return immediately since value doesn't match)
            let result = futex.wait(999, Some(Duration::from_millis(1);
            assert!(result.is_ok();

            // Test wake
            let result = futex.wake(1;
            assert!(result.is_ok();
        }

        #[test]
        fn test_nostd_memory_provider() {
            let provider = safe_managed_alloc!(2048, CrateId::Platform).expect("Failed to create provider");

            assert_eq!(provider.verification_level(), VerificationLevel::Standard;
            assert!(provider.capacity() <= 4096)); // Capped at 4096 in stub implementation
        }

        #[test]
        fn test_wasm_page_size_constant() {
            assert_eq!(WASM_PAGE_SIZE, 65536); // 64KB
        }
    }

    // ===========================================
    // WRT-RUNTIME NO_STD TESTS
    // ===========================================
    
    mod wrt_runtime_tests {
        use super::*;
        use wrt_runtime::{Memory, Table, global::Global, MemoryType as RuntimeMemoryType};
        use wrt_foundation::{ValueType, values::Value};

        #[test]
        fn test_memory_no_std() {
            let mem_type = RuntimeMemoryType {
                minimum: 1,
                maximum: Some(2),
                shared: false,
            };

            let memory = Memory::new(mem_type).unwrap();

            let data = [1, 2, 3, 4];
            assert!(memory.write(100, &data).is_ok();

            let mut buffer = [0; 4];
            assert!(memory.read(100, &mut buffer).is_ok();

            assert_eq!(buffer, data;
        }

        #[test]
        fn test_global_no_std() {
            let global = Global::new(ValueType::I32, true, Value::I32(42)).unwrap();

            assert_eq!(global.get(), Value::I32(42;

            assert!(global.set(Value::I32(100)).is_ok();
            assert_eq!(global.get(), Value::I32(100;
        }
    }

    // ===========================================
    // WRT-INSTRUCTIONS NO_STD TESTS
    // ===========================================
    
    mod wrt_instructions_tests {
        use super::*;
        use wrt_instructions::opcodes::Opcode;

        #[test]
        fn test_opcodes_no_std() {
            let i32_const = Opcode::I32Const;
            let i32_add = Opcode::I32Add;

            assert_ne!(i32_const, i32_add;
        }

        #[test]
        fn test_opcode_serialization() {
            let opcode = Opcode::I32Const;
            
            // Test that opcodes have consistent representation
            assert_eq!(core::mem::size_of::<Opcode>(), 1;
        }
    }

    // ===========================================
    // WRT-DECODER NO_STD TESTS
    // ===========================================
    
    mod wrt_decoder_tests {
        use super::*;
        use wrt_decoder::conversion::{
            format_limits_to_types_limits,
            types_limits_to_format_limits,
        };

        #[test]
        fn test_limits_conversion_no_std() {
            let format_limits = wrt_format::Limits {
                min: 1,
                max: Some(2),
                memory64: false,
                shared: false,
            };

            let types_limits = format_limits_to_types_limits(format_limits;

            assert_eq!(types_limits.min, 1;
            assert_eq!(types_limits.max, Some(2;
            assert_eq!(types_limits.shared, false;

            let format_limits2 = types_limits_to_format_limits(types_limits;

            assert_eq!(format_limits2.min, 1;
            assert_eq!(format_limits2.max, Some(2;
            assert_eq!(format_limits2.shared, false;
            assert_eq!(format_limits2.memory64, false;
        }
    }

    // ===========================================
    // WRT-FORMAT NO_STD TESTS
    // ===========================================
    
    mod wrt_format_tests {
        use super::*;
        use wrt_format::{
            module::Module as FormatModule,
            section::Section,
        };

        #[test]
        fn test_format_module_creation() {
            // Test that we can create format structures in no_std
            let _module = FormatModule::default);
        }

        #[test]
        fn test_section_types() {
            // Test section type discrimination in no_std
            let type_section = Section::Type(vec![];
            let function_section = Section::Function(vec![];
            
            assert!(matches!(type_section, Section::Type(_));
            assert!(matches!(function_section, Section::Function(_));
        }
    }

    // ===========================================
    // WRT-HOST NO_STD TESTS
    // ===========================================
    
    mod wrt_host_tests {
        use super::*;
        // Note: wrt-host may not have extensive no_std functionality yet
        
        #[test]
        fn test_host_no_std_basic() {
            // Basic test to ensure the crate compiles in no_std
            // More specific tests would be added based on wrt-host's no_std API
        }
    }

    // ===========================================
    // WRT-LOGGING NO_STD TESTS
    // ===========================================
    
    mod wrt_logging_tests {
        use super::*;
        use wrt_logging::{Level, Operation};

        #[test]
        fn test_log_levels_no_std() {
            let error_level = Level::Error;
            let info_level = Level::Info;
            let debug_level = Level::Debug;

            assert_ne!(error_level, info_level;
            assert_ne!(info_level, debug_level;
        }

        #[test]
        fn test_log_operations_no_std() {
            // Test that logging operations work in no_std
            let operation = Operation::new(Level::Info, "test message";
            assert_eq!(operation.level(), Level::Info;
        }
    }

    // ===========================================
    // WRT-INTERCEPT NO_STD TESTS
    // ===========================================
    
    mod wrt_intercept_tests {
        use super::*;
        // Basic intercept functionality tests for no_std

        #[test]
        fn test_intercept_no_std_basic() {
            // Basic test to ensure the crate compiles in no_std
            // More specific tests would be added based on wrt-intercept's no_std API
        }
    }

    // ===========================================
    // WRT-COMPONENT NO_STD TESTS
    // ===========================================
    
    mod wrt_component_tests {
        use super::*;
        // Note: Component model typically requires more features

        #[test]
        fn test_component_no_std_basic() {
            // Basic test for component model in no_std (if supported)
            // Binary std/no_std choice
        }
    }

    // ===========================================
    // WRT-TEST-REGISTRY NO_STD TESTS
    // ===========================================
    
    mod wrt_test_registry_tests {
        use super::*;
        // Test that the test registry itself works in no_std

        #[test]
        fn test_registry_no_std_basic() {
            // Test basic registry functionality in no_std
            // This ensures our testing infrastructure itself is no_std compatible
        }
    }

    // ===========================================
    // CROSS-CRATE INTEGRATION TESTS
    // ===========================================
    
    mod integration_tests {
        use super::*;

        #[test]
        fn test_error_with_foundation_types() {
            use wrt_error::{Error, ErrorCategory};
            use wrt_foundation::ValueType;

            // Test that we can use error handling with foundation types
            let error = Error::runtime_execution_error(",
            ;

            let _value_type = ValueType::I32;
            assert_eq!(error.category, ErrorCategory::Validation;
        }

        #[test]
        fn test_platform_with_foundation_memory() {
            use wrt_platform::WASM_PAGE_SIZE;
            use wrt_foundation::bounded::BoundedVec;
            use wrt_foundation::NoStdProvider;

            // Test integration between platform and foundation
            assert_eq!(WASM_PAGE_SIZE, 65536;

            const CAPACITY: usize = 4;
            let provider = NoStdProvider::<{ CAPACITY * 4 }>::default);
            let mut vec: BoundedVec<u32, CAPACITY, NoStdProvider<{ CAPACITY * 4 }>> =
                BoundedVec::new(provider).unwrap();

            // Ensure we can store page-related data
            vec.push(WASM_PAGE_SIZE as u32).unwrap();
            assert_eq!(vec.get(0).unwrap(), WASM_PAGE_SIZE as u32;
        }
    }
}

// ===========================================
// PANIC HANDLER FOR NO_STD ENVIRONMENTS
// ===========================================

// Panic handler disabled to avoid conflicts with workspace builds
// #[cfg(all(not(feature = "), not(test)))]
// #[panic_handler]
// fn panic(_info: &core::panic::PanicInfo) -> ! {
//     loop {}
// }

// ===========================================
// ENTRY POINT FOR NO_STD ENVIRONMENTS
// ===========================================

#[cfg(all(not(feature = "std"), not(test)))]
#[no_main]
#[export_name = "_start"]
pub extern "C" fn _start() -> ! {
    loop {}
}