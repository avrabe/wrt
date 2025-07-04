//! Prelude module for wrt
//!
//! This module provides a unified set of imports for both std and no_std
//! environments. It re-exports commonly used types and traits from specialized
//! crates to ensure consistency across the WRT project and simplify imports in
//! individual modules.

// Core imports for both std and no_std environments
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
    sync::atomic::{AtomicUsize, Ordering},
};

// Re-export from std when the std feature is enabled (non-safety-critical)
#[cfg(all(feature = "std", not(feature = "safety-critical")))]
pub use std::{
    boxed::Box,
    collections::{HashMap, HashSet},
    format, println,
    string::{String, ToString},
    sync::{Arc, Mutex, MutexGuard, RwLock, RwLockReadGuard, RwLockWriteGuard},
    vec,
    vec::Vec,
};

// Re-export WRT allocator collections for safety-critical mode
#[cfg(all(feature = "std", feature = "safety-critical"))]
pub use std::{
    boxed::Box,
    format, println,
    string::{String, ToString},
    sync::{Arc, Mutex, MutexGuard, RwLock, RwLockReadGuard, RwLockWriteGuard},
};

#[cfg(all(feature = "std", feature = "safety-critical"))]
pub use wrt_foundation::allocator::{WrtVec as Vec, WrtHashMap as HashMap};

// HashSet for safety-critical mode (simplified as it's less commonly used)
#[cfg(all(feature = "std", feature = "safety-critical"))]
pub use std::collections::HashSet; // Using std HashSet - bounded alternative is BoundedSet in no_std mode

// Binary std/no_std choice - use our own memory management
#[cfg(not(feature = "std"))]
pub use wrt_foundation::{
    bounded::{BoundedString as String, BoundedVec as Vec},
    bounded_collections::{BoundedSet as HashSet, BoundedMap as HashMap},
};

// Binary std/no_std choice - format macro not available without alloc
#[cfg(not(feature = "std"))]
#[macro_export]
macro_rules! format {
    ($($arg:tt)*) => {{
        "static string - format not available in no_std without alloc"
    }};
}

// Binary std/no_std choice - vec macro using bounded collections
#[cfg(not(feature = "std"))]
#[macro_export]
macro_rules! vec {
    () => {{
        use wrt_foundation::safe_managed_alloc, budget_aware_provider::CrateId};
        let guard = safe_managed_alloc!(1024, CrateId::Runtime).unwrap();
        wrt_foundation::bounded::BoundedVec::new(guard.provider().clone()).unwrap()
    }};
    ($($x:expr),*) => {{
        use wrt_foundation::safe_managed_alloc, budget_aware_provider::CrateId};
        let guard = safe_managed_alloc!(1024, CrateId::Runtime).unwrap();
        let mut v = wrt_foundation::bounded::BoundedVec::new(guard.provider().clone()).unwrap();
        $(v.push($x).unwrap();)*
        v
    }};
}

// Safety-critical vec! macro that uses WRT allocator
#[cfg(all(feature = "std", feature = "safety-critical"))]
#[macro_export]
macro_rules! vec {
    () => {
        wrt_foundation::allocator::WrtVec::<_, {wrt_foundation::allocator::CrateId::Wrt as u8}, 256>::new()
    };
    ($($x:expr),*) => {{
        let mut v = wrt_foundation::allocator::WrtVec::<_, {wrt_foundation::allocator::CrateId::Wrt as u8}, 256>::new();
        $(let _ = v.push($x);)*
        v
    }};
}

// Standard vec! macro for non-safety-critical std mode
#[cfg(all(feature = "std", not(feature = "safety-critical")))]
pub use std::vec;

// Note: wrt-component exports would go here if available
// Note: wrt-decoder exports would go here if available
// Re-export from wrt-error (foundation crate)
pub use wrt_error::{
    codes, context, helpers, kinds, Error, ErrorCategory, ErrorSource, FromError, Result,
    ToErrorCategory,
};
// Note: wrt-format exports would go here if available
// Remove duplicate imports - already handled above
// Re-export from wrt-foundation (core foundation library)
pub use wrt_foundation::{
    // Bounded collections (safety-first alternatives to standard collections)
    bounded::{BoundedError, BoundedStack, BoundedVec, CapacityError},
    component::{
        ComponentType, ExternType, GlobalType as ComponentGlobalType, InstanceType,
        MemoryType as ComponentMemoryType, TableType as ComponentTableType,
    },
    component_value::{ComponentValue, ValType},
    // Safe memory types - prioritizing these over standard collections
    safe_memory::{
        MemoryProvider, SafeMemoryHandler,
        SafeSlice, SafeStack,
    },
    // Core types
    types::{BlockType, FuncType, ValueType},
    // validation::{Checksummed}, // Not available yet
    values::{v128, Value, V128},
    verification::{Checksum, VerificationLevel},
};

// Re-export clean types from wrt-foundation (when available)
#[cfg(any(feature = "std", feature = "alloc"))]
pub use wrt_foundation::clean_types::{
    ValType as CleanValType,
    FuncType as CleanFuncType,
    MemoryType as CleanMemoryType,
    TableType as CleanTableType,
    GlobalType as CleanGlobalType,
    ComponentType as CleanComponentType,
    ExternType as CleanExternType,
    InstanceType as CleanInstanceType,
    Value as CleanValue,
    // Nested types
    Field, Record, Tuple, Variant, Case, Enum, Result_ as CleanResult, Flags,
    Limits as CleanLimits, RefType as CleanRefType,
};
// Note: wrt-host exports would go here if available
// Note: wrt-instructions behavior exports would go here if available
// Note: wrt-instructions exports would go here if available
// Note: wrt-intercept exports would go here if available
// Re-export from wrt-platform (platform-specific implementations)
pub use wrt_platform::{
    BranchTargetIdentification, BtiExceptionLevel, BtiMode, CfiExceptionMode, ControlFlowIntegrity,
};
// Re-export from wrt-runtime (runtime execution)
// Selectively re-export working components to avoid compilation issues
pub use wrt_runtime::{
    // Module types
    module::Module,
    module_instance::ModuleInstance,
    // Stackless engine
    stackless::{
        StacklessEngine, StacklessExecutionState, StacklessFrame,
    },
    // Capability-based engine (when available)
    #[cfg(any(feature = "std", feature = "alloc"))]
    engine::{
        CapabilityEngine, CapabilityAwareEngine, EnginePreset,
        ModuleHandle, InstanceHandle, EngineBuilder,
    },
};
// Note: wrt-sync exports would go here if available
// Import synchronization primitives for no_std
#[cfg(not(feature = "std"))]
pub use wrt_sync::{
    WrtMutex as Mutex, WrtMutexGuard as MutexGuard, WrtRwLock as RwLock,
    WrtRwLockReadGuard as RwLockReadGuard, WrtRwLockWriteGuard as RwLockWriteGuard,
};

// Re-export CFI integration types from main wrt crate (std only currently) - temporarily disabled
// #[cfg(feature = "std")]
// pub use crate::cfi_integration::{
//     CfiConfiguration, CfiEngineStatistics as CfiIntegrationStatistics,
//     CfiExecutionResult as CfiIntegrationResult, CfiHardwareFeatures, CfiProtectedEngine,
//     CfiProtectedModule,
// };
