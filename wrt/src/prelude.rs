//! Prelude module for wrt
//!
//! This module provides a unified set of imports for both std and no_std environments.
//! It re-exports commonly used types and traits from specialized crates to ensure
//! consistency across the WRT project and simplify imports in individual modules.

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

// Re-export from wrt-error (foundation crate)
pub use wrt_error::{
    codes, context, helpers, kinds, Error, ErrorCategory, ErrorSource, FromError, Result,
    ToErrorCategory,
};

// Re-export from wrt-types (core type definitions)
pub use wrt_types::{
    // Bounded collections (safety-first alternatives to standard collections)
    bounded::{BoundedError, BoundedHashMap, BoundedStack, BoundedVec, CapacityError},
    component::{
        ComponentType, ExternType, GlobalType as ComponentGlobalType, InstanceType,
        MemoryType as ComponentMemoryType, TableType as ComponentTableType,
    },
    component_value::{ComponentValue, ValType},
    // Safe memory types - prioritizing these over standard collections
    safe_memory::{
        MemoryProvider, MemorySafety, MemoryStats, MemoryVerification, SafeMemoryHandler,
        SafeSlice, SafeStack,
    },
    // Core types
    types::{BlockType, FuncType, GlobalType, Limits, MemoryType, RefType, TableType, ValueType},
    validation::{BoundedCapacity, Checksummed, Validatable as TypesValidatable},
    values::{v128, Value, V128},
    verification::{Checksum, VerificationLevel},
};

// Re-export from wrt-format (format specifications)
pub use wrt_format::{
    binary, component::Component as FormatComponent, is_state_section_name,
    module::Module as FormatModule, validation::Validatable as FormatValidatable, StateSection,
};

// Re-export from wrt-decoder (binary parsing)
pub use wrt_decoder::{
    create_engine_state_section, from_binary, get_data_from_state_section,
    module::Module as DecoderModule, parse, section_reader,
};

// Re-export from wrt-instructions (instruction encoding/decoding)
pub use wrt_instructions::{
    calls::CallInstruction,
    control::ControlInstruction,
    memory_ops::{MemoryArg, MemoryLoad, MemoryStore},
    numeric::NumericInstruction,
    Instruction,
};

// Re-export behavior traits from wrt-instructions
pub use wrt_instructions::behavior::{
    ControlFlow, ControlFlowBehavior, EngineBehavior, FrameBehavior, InstructionExecutor, Label,
    ModuleBehavior, StackBehavior,
};

// Re-export from wrt-sync (synchronization primitives)
pub use wrt_sync::{concurrency::ThreadSafe, sync_primitives::SyncAccess};

// Import synchronization primitives for no_std
#[cfg(not(feature = "std"))]
pub use wrt_sync::{
    WrtMutex as Mutex, WrtMutexGuard as MutexGuard, WrtRwLock as RwLock,
    WrtRwLockReadGuard as RwLockReadGuard, WrtRwLockWriteGuard as RwLockWriteGuard,
};

// Re-export from wrt-runtime (runtime execution)
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

// Re-export from wrt-host (host interface)
pub use wrt_host::{
    environment::{Environment, HostEnvironment},
    host_functions::{HostFunction, HostFunctionRegistry},
};

// Re-export from wrt-component (component model)
pub use wrt_component::{
    instance::ComponentInstance,
    interface::{Interface, InterfaceMapping},
    module::ComponentModule,
};

// Re-export from wrt-intercept (function interception)
pub use wrt_intercept::{
    interceptor::{FunctionInterceptor, InterceptorRegistry},
    strategies::{DefaultInterceptStrategy, InterceptStrategy},
};
