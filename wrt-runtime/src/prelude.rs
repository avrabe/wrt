//! Prelude module for wrt-runtime
//!
//! This module provides a unified set of imports for both std and no_std
//! environments. It re-exports commonly used types and traits to ensure
//! consistency across all crates in the WRT project and simplify imports in
//! individual modules.

// Core imports for both std and no_std environments
// Re-export from alloc when the alloc feature is enabled and std is not
#[cfg(all(feature = "alloc", not(feature = "std")))]
pub use alloc::{
    boxed::Box,
    collections::{BTreeMap as HashMap, BTreeSet as HashSet},
    format,
    string::{String, ToString},
    sync::Arc,
    vec,
    vec::Vec,
};
pub use core::{
    any::Any,
    cmp::{Eq, Ord, PartialEq, PartialOrd},
    convert::{TryFrom, TryInto},
    fmt,
    fmt::{Debug, Display},
    marker::PhantomData,
    mem,
    ops::{Deref, DerefMut},
    slice, str,
    sync::atomic::{AtomicUsize, Ordering as AtomicOrdering},
};
// Re-export from std when the std feature is enabled
#[cfg(feature = "std")]
pub use std::{
    boxed::Box,
    collections::{HashMap, HashSet},
    format,
    string::{String, ToString},
    sync::{Arc, Mutex, MutexGuard, RwLock, RwLockReadGuard, RwLockWriteGuard},
    vec,
    vec::Vec,
};

// Re-export from wrt-decoder (aliased to avoid name clashes)
pub use wrt_decoder::component::Component as DecoderComponentDefinition;
// Re-export from wrt-instructions for instruction types
pub use wrt_decoder::instructions::Instruction;
pub use wrt_decoder::prelude::Module as DecoderModule;
// Re-export from wrt-error for error handling
pub use wrt_error::prelude::{
    codes, create_simple_component_error, create_simple_memory_error, create_simple_resource_error,
    create_simple_runtime_error, create_simple_type_error, create_simple_validation_error,
    kinds::{
        self, ComponentError, InvalidType, OutOfBoundsError, ParseError, ResourceError,
        RuntimeError, ValidationError,
    },
    Error, ErrorCategory, Result,
};
// Re-export from wrt-format for format specifications (aliased to avoid name clashes)
pub use wrt_format::component::Component as FormatComponent;
pub use wrt_format::{
    module::{
        Data as FormatData, Element as FormatElement, Export as FormatExport,
        ExportKind as FormatExportKind, Function as FormatFunction, Global as FormatGlobal,
        Import as FormatImport, ImportDesc as FormatImportDesc, Memory as FormatMemory,
        Table as FormatTable,
    },
    section::CustomSection as FormatCustomSection,
};
// Re-export from wrt-host (for runtime host interaction items)
pub use wrt_host::prelude::CallbackRegistry as HostFunctionRegistry;
pub use wrt_host::prelude::HostFunctionHandler as HostFunction;
pub use wrt_instructions::{
    control_ops::BranchTarget as Label, instruction_traits::PureInstruction as InstructionExecutor,
};
// Re-export from wrt-intercept (for runtime interception items)
pub use wrt_intercept::prelude::LinkInterceptor as InterceptorRegistry;
pub use wrt_intercept::prelude::LinkInterceptorStrategy as InterceptStrategy;
// Synchronization primitives for no_std (if alloc is enabled but not std)
#[cfg(all(feature = "alloc", not(feature = "std")))]
pub use wrt_sync::prelude::{Mutex, MutexGuard, RwLock, RwLockReadGuard, RwLockWriteGuard};
// Re-export from wrt-types for core types
pub use wrt_types::component::{ComponentType, ExternType};
// Re-export core types from wrt_types instead of wrt_format
pub use wrt_types::types::{
    CustomSection, /* Assuming this is the intended replacement for FormatCustomSection
                   * Add other direct re-exports from wrt_types::types if they were previously
                   * from wrt_format::module e.g., DataSegment,
                   * ElementSegment, Export, GlobalType, Import, MemoryType, TableType,
                   * FuncType For now, only replacing what was directly
                   * aliased or used in a way that implies a direct replacement need. */
};
pub use wrt_types::{
    prelude::{
        BlockType, BoundedStack, BoundedVec, ComponentValue, FuncType,
        GlobalType as CoreGlobalType, MemoryType as CoreMemoryType, ResourceType,
        SafeMemoryHandler, SafeSlice, SafeStack, TableType as CoreTableType,
        ValType as ComponentValType, Value, ValueType, VerificationLevel,
    },
    safe_memory::{MemorySafety, MemoryStats},
    types::Limits,
    values::V128,
};

// Execution related types defined in wrt-runtime
pub use crate::execution::{ExecutionContext, ExecutionStats}; /* Removed ExecutionResult as
                                                                * it's not defined in
                                                                * execution.rs */
// --- Local definitions from wrt-runtime ---
// These are types defined within wrt-runtime itself, re-exported for convenience if used
// widely.

// Core runtime structures
// pub use crate::func::Function; // Removed: func.rs only re-exports FuncType from wrt_types,
// which is already in prelude. RuntimeFunction (from module.rs) is the primary Function struct
// for the runtime.
pub use crate::global::Global;
// Adapters and helpers if they are part of the public API exported by this prelude
pub use crate::memory_adapter::MemoryAdapter;
// Module items specific to wrt-runtime module structure
pub use crate::module::{Data, Element, Export, ExportItem, ExportKind, Import, OtherExport};
// Stackless execution engine components
pub use crate::stackless::{
    StacklessCallbackRegistry, StacklessEngine, StacklessExecutionState, StacklessFrame,
    StacklessStack,
};
pub use crate::{
    memory::Memory, module::Module as RuntimeModule,
    module_instance::ModuleInstance as RuntimeModuleInstance, table::Table,
};

// The following re-exports from wrt_format are removed as wrt-runtime should
// not depend on wrt-format. pub use wrt_format::module::{ // REMOVED
// DataMode as FormatDataMode, Element as FormatElement, Export as FormatExport,
// // ... and others }; // REMOVED
// pub use wrt_format::section::CustomSection as FormatCustomSection; // REMOVED
// pub use wrt_format::component::Component as FormatComponent; // REMOVED
// (Component model types should come from wrt_component or wrt_types if
// foundational)
