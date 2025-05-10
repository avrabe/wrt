//! Prelude module for wrt-test-registry
//!
//! This module provides a unified set of imports for both std and no_std environments.
//! It imports directly from each dependency crate rather than through wrt::prelude
//! to ensure proper separation of concerns and prevent circular dependencies.

// Core imports for both std and no_std environments
pub use core::{
    any::Any,
    cmp::{Eq, Ord, PartialEq, PartialOrd},
    convert::{TryFrom, TryInto},
    fmt,
    fmt::Debug,
    fmt::Display,
    marker::PhantomData,
    mem,
    ops::{Deref, DerefMut},
    slice, str,
    sync::atomic::{AtomicUsize, Ordering},
};

// Re-export from std when the std feature is enabled
#[cfg(feature = "std")]
pub use std::{
    boxed::Box,
    collections::{HashMap, HashSet},
    format, println,
    string::{String, ToString},
    sync::{Arc, Mutex, MutexGuard, RwLock, RwLockReadGuard, RwLockWriteGuard},
    time::Duration,
    vec,
    vec::Vec,
};

// Re-export from alloc when no_std but alloc is available
#[cfg(all(not(feature = "std"), feature = "alloc"))]
pub use alloc::{
    boxed::Box,
    collections::{BTreeMap as HashMap, BTreeSet as HashSet},
    format,
    string::{String, ToString},
    sync::Arc,
    vec,
    vec::Vec,
};

// OnceCell for both std and no_std
#[cfg(feature = "std")]
pub use once_cell::sync::OnceCell;

#[cfg(not(feature = "std"))]
pub use core::cell::OnceCell;

// === Implementation sequence imports ===

// 1. Re-export from wrt-error (foundation crate)
pub use wrt_error::{
    codes, context, helpers, kinds, Error, ErrorCategory, ErrorSource, FromError, Result,
    ToErrorCategory,
};

// 2. Re-export from wrt-types (core type definitions)
pub use wrt_types::{
    // Bounded collections (safety-first alternatives to standard collections)
    bounded::{BoundedError, BoundedHashMap, BoundedStack, BoundedVec, CapacityError},
    component::{
        ComponentType, ExternType, GlobalType as ComponentGlobalType, InstanceType,
        MemoryType as ComponentMemoryType, TableType as ComponentTableType,
    },
    component_value::{ComponentValue, ValType},
    // Safe memory types
    safe_memory::{
        MemoryProvider, MemorySafety, MemoryStats, SafeMemoryHandler, SafeSlice, SafeStack,
    },
    // Core types
    types::{BlockType, FuncType, GlobalType, Limits, MemoryType, RefType, TableType, ValueType},
    validation::{BoundedCapacity, Checksummed, Validatable as TypesValidatable},
    values::{v128, Value, V128},
    verification::{Checksum, VerificationLevel},
};

// 3. Re-export from wrt-format (format specifications)
pub use wrt_format::{
    binary::{binary_to_val_type, binary_to_value_type, val_type_to_binary, value_type_to_binary},
    component::Component as FormatComponent,
    limits::{format_limits_to_types_limits, types_limits_to_format_limits},
    module::Module as FormatModule,
    runtime::RuntimeLimits,
    validation::Validatable as FormatValidatable,
};

// 4. Re-export from wrt-decoder (binary parsing)
pub use wrt_decoder::{from_binary, module::Module as DecoderModule, parse, section_reader};

// 5. Re-export from wrt-sync (synchronization primitives)
pub use wrt_sync::{concurrency::ThreadSafe, sync_primitives::SyncAccess};

// Import synchronization primitives for no_std
#[cfg(not(feature = "std"))]
pub use wrt_sync::{
    WrtMutex as Mutex, WrtMutexGuard as MutexGuard, WrtRwLock as RwLock,
    WrtRwLockReadGuard as RwLockReadGuard, WrtRwLockWriteGuard as RwLockWriteGuard,
};

// 6. Re-export from wrt-instructions (instruction encoding/decoding)
pub use wrt_instructions::{
    behavior::{
        ControlFlow, ControlFlowBehavior, EngineBehavior, FrameBehavior, InstructionExecutor,
        Label, ModuleBehavior, StackBehavior,
    },
    calls::CallInstruction,
    control::ControlInstruction,
    memory_ops::{MemoryArg, MemoryLoad, MemoryStore},
    numeric::NumericInstruction,
    Instruction,
};

// 7. Re-export from wrt-intercept (function interception)
pub use wrt_intercept::{
    interceptor::{FunctionInterceptor, InterceptorRegistry},
    strategies::{DefaultInterceptStrategy, InterceptStrategy},
};

// 8. Re-export from wrt-host (host interface)
pub use wrt_host::{
    environment::{Environment, HostEnvironment},
    host_functions::{HostFunction, HostFunctionRegistry},
};

// 9. Re-export from wrt-component (component model)
pub use wrt_component::{
    instance::ComponentInstance,
    interface::{Interface, InterfaceMapping},
    module::ComponentModule,
};

// 10. Re-export from wrt-runtime (runtime execution)
pub use wrt_runtime::{
    component::{Component, Host, InstanceValue},
    execution::ExecutionStats,
    func::Function,
    global::Global,
    memory::Memory,
    module::{
        Data, Element, Export, ExportItem, ExportKind, Function as RuntimeFunction, Import, Module,
        OtherExport,
    },
    module_instance::ModuleInstance,
    stackless::{
        StacklessCallbackRegistry, StacklessEngine, StacklessExecutionState, StacklessFrame,
    },
    table::Table,
};

// 11. Re-export from wrt (main library)
// Only import essential functionality to avoid circular dependencies
pub use wrt::{
    load_module_from_binary, new_memory, new_module, new_stackless_engine, new_table,
    COMPONENT_VERSION, CORE_VERSION,
};

// Do not import from wrt directly to avoid circular dependencies
// Instead, re-export only what's needed from this crate
pub use crate::{TestCase, TestCaseImpl, TestConfig, TestRegistry, TestResult, TestStats};

// Define custom assert macros for test results
#[macro_export]
macro_rules! assert_test {
    ($cond:expr) => {
        if !$cond {
            return Err(format!("Assertion failed: {}", stringify!($cond)));
        }
    };
}

#[macro_export]
macro_rules! assert_eq_test {
    ($left:expr, $right:expr) => {
        if $left != $right {
            return Err(format!(
                "Assertion failed: {} != {}\n  left: {:?}\n right: {:?}",
                stringify!($left),
                stringify!($right),
                $left,
                $right
            ));
        }
    };
}
