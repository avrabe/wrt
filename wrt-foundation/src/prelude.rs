// WRT - wrt-foundation
// Copyright (c) 2025 Ralf Anton Beier
// Licensed under the MIT license.
// SPDX-License-Identifier: MIT

//! Prelude module for wrt-foundation
//!
//! This module provides a unified set of imports for both std and `no_std`
//! environments. It re-exports commonly used types and traits to ensure
//! consistency across all crates in the WRT project and simplify imports in
//! individual modules.

// Binary std/no_std choice - conditional imports only

// Core traits and types available in both std and no_std
// alloc-only imports (when std is not available)
#[cfg(all(feature = "alloc", not(feature = "std")))]
pub use alloc::{
    boxed::Box,
    collections::{
        BTreeMap,
        BTreeSet,
    },
    format,
    string::{
        String,
        ToString,
    },
    vec,
    vec::Vec,
};
pub use core::{
    any::Any,
    clone::Clone,
    cmp::{
        Eq,
        Ord,
        PartialEq,
        PartialOrd,
    },
    convert::{
        TryFrom,
        TryInto,
    },
    default::Default,
    fmt::{
        self,
        Debug,
        Display,
        Write,
    },
    hash::Hash,
    marker::{
        Copy,
        PhantomData,
        Sized,
    },
    mem,
    ops::{
        Deref,
        DerefMut,
    },
    slice,
    str,
};
// std-only imports
#[cfg(feature = "std")]
pub use std::{
    boxed::Box,
    collections::{
        BTreeMap,
        BTreeSet,
        HashMap,
        HashSet,
    },
    format,
    string::{
        String,
        ToString,
    },
    sync::{
        Arc,
        Mutex,
        RwLock,
    },
    vec,
    vec::Vec,
};

// no_std alternatives using bounded collections - handled in main re-exports below
#[cfg(feature = "use-hashbrown")]
pub use hashbrown::HashMap as BHashMap;
// Binary std/no_std choice
// HashSet, Arc are NOT exported by this prelude. Users should use bounded types or core types
// directly.

// Re-export from wrt_error - this is the standard Result type for WRT
pub use wrt_error::prelude::*;
pub use wrt_error::{
    codes,
    kinds,
    Error,
    ErrorCategory,
    Result,
};

// Feature-gated re-exports that can't be included in the main use block
#[cfg(feature = "std")]
pub use crate::component_builder::{
    ComponentTypeBuilder,
    ExportBuilder,
    ImportBuilder,
    NamespaceBuilder,
};
// Modern memory system convenience functions already imported above

// Binary std/no_std choice
#[cfg(feature = "std")]
pub use crate::conversion::{
    ref_type_to_val_type,
    val_type_to_ref_type,
};
// Re-export from wrt_sync, only if the feature is active
// #[cfg(feature = "wrt-sync")] // Or a more specific feature if wrt-sync is always a dep

// Re-export platform-specific memory builders if the feature is enabled
// Memory builders removed in clean architecture
// #[cfg(feature = "platform-memory")]
// pub use crate::memory_builder::{LinearMemoryBuilder, PalMemoryProviderBuilder};
// Binary std/no_std choice
#[cfg(not(feature = "std"))]
pub use crate::no_std_hashmap::SimpleHashMap;
// Re-export from this crate
pub use crate::{
    // ASIL testing framework
    asil_testing::{
        get_asil_tests,
        get_test_statistics,
        get_tests_by_asil,
        get_tests_by_category,
        register_asil_test,
        AsilTestMetadata,
        TestCategory,
        TestStatistics,
    },
    // Atomic memory operations
    atomic_memory::{
        AtomicMemoryExt,
        AtomicMemoryOps,
    },
    // Bounded collections
    bounded::{
        BoundedStack,
        BoundedString,
        BoundedVec,
        CapacityError,
        WasmName,
    },
    bounded_collections::{
        BoundedDeque,
        BoundedMap,
        BoundedQueue,
        BoundedSet,
    },
    // Builder patterns
    builder::{
        BoundedBuilder,
        MemoryBuilder,
        ResourceBuilder,
        ResourceItemBuilder,
        StringBuilder,
    },
    // Builtin types
    builtin::BuiltinType,
    // Component model types
    component::{
        ComponentType,
        ExternType,
        InstanceType, // Limits,
        Namespace,
        ResourceType,
    },
    memory_coordinator::{
        AllocationId,
        CrateIdentifier,
        GenericMemoryCoordinator,
    },
    // Resource types
    resource::ResourceOperation,
    // Safe memory types (SafeMemoryHandler, SafeSlice, SafeStack are already here from direct
    // re-exports) Sections (SectionId, SectionType, Section are usually handled by decoder)
    // Binary std/no_std choice
    // safe_memory::NoStdProvider, // Re-exported below to avoid duplicate
    // Safety system types
    safety_system::{
        AsilLevel,
        SafeMemoryAllocation,
        SafetyContext,
        SafetyGuard,
    },
    // Validation traits (moved to traits module to break circular dependency)
    traits::{
        BoundedCapacity,
        Checksummed,
        Validatable, /* ValidationContext,
                      * ValidationError and ValidationResult will be re-added when validation
                      * module is restored */
    },
    // Traits
    traits::{
        FromFormat,
        ToFormat,
    },
    // Common types (BlockType, FuncType, GlobalType, MemoryType, TableType, ValueType are already
    // here)
    types::{
        // BlockType,
        FuncType,
        GlobalType,
        Limits,
        MemoryType,
        RefType,
        TableType,
        ValueType,
    },
    // New unified types from Agent A deliverables (simplified)
    unified_types_simple::{
        DefaultTypes,
        DesktopTypes,
        EmbeddedTypes,
        PlatformCapacities,
        SafetyCriticalTypes,
        UnifiedTypes,
    },
    // Value representations
    values::{
        FloatBits32,
        FloatBits64,
        Value,
    },
    // Verification types
    verification::{
        Checksum,
        VerificationLevel,
    },
    // Direct re-exports for convenience (original list)
    // ResourceType, // Already covered by component::* above
    SafeMemoryHandler,
    SafeSlice,
};
// std-only memory provider
// UnifiedStdProvider is now part of the modern memory system
#[cfg(feature = "std")]
// Alloc-dependent re-exports
#[cfg(feature = "std")]
pub use crate::{
    // Component builders
    component_value::{
        ComponentValue,
        ValType,
    },
    component_value_store::{
        ComponentValueStore,
        ValueRef,
    },
    component_value_store_builder::ComponentValueStoreBuilder,
};
// Memory system core exports
pub use crate::{
    // Modern memory system types
    generic_memory_guard::GenericMemoryGuard,
    // Core memory allocation macro
    safe_managed_alloc,
    // Memory provider types
    safe_memory::Provider as MemoryProvider,
    wrt_memory_system::CapabilityWrtFactory,
};

// Binary std/no_std choice
/// Maximum number of arguments/results for WebAssembly functions
pub const MAX_WASM_FUNCTION_PARAMS: usize = 128;

/// Binary std/no_std choice
// Convenient type aliases for WebAssembly function parameters
/// Binary std/no_std choice - bounded vector for function arguments
#[cfg(not(feature = "std"))]
pub type ArgVec<T> =
    BoundedVec<T, MAX_WASM_FUNCTION_PARAMS, NoStdProvider<{ MAX_WASM_FUNCTION_PARAMS * 16 }>>;

/// Binary std/no_std choice - standard vector for function arguments
#[cfg(feature = "std")]
pub type ArgVec<T> = Vec<T>;

// Memory system convenience re-exports
// For no_std environments, use simpler type alias
#[cfg(not(any(feature = "std", feature = "alloc")))]
pub use crate::capabilities::NoStdCapabilityContext as CapabilityContext;
// Capability system exports (when available)
#[cfg(any(feature = "std", feature = "alloc"))]
pub use crate::capabilities::{
    CapabilityAwareProvider,
    CapabilityProviderFactory,
    ProviderCapabilityExt,
};
// Re-export NoStdProvider only once
pub use crate::safe_memory::NoStdProvider;
pub use crate::{
    // Budget management
    budget_aware_provider::CrateId,
    // Capability system
    capabilities::{
        CapabilityMask,
        MemoryCapability,
        MemoryGuard,
        MemoryOperation,
        MemoryOperationType,
        MemoryRegion,
    },
    // Memory initialization
    memory_init::MemoryInitializer,
};
