// WRT - wrt-runtime
// Module: Core WebAssembly Runtime
// SW-REQ-ID: REQ_001
// SW-REQ-ID: REQ_002
// SW-REQ-ID: REQ_MEM_SAFETY_001
//
// Copyright (c) 2024 Ralf Anton Beier
// Licensed under the MIT license.
// SPDX-License-Identifier: MIT

//! WebAssembly Runtime (WRT) - Runtime Implementation
//!
//! This crate provides the core runtime types and implementations for
//! WebAssembly, shared between both the core WebAssembly and Component Model
//! implementations.
//!
//! # Safety
//!
//! Most modules forbid unsafe code. Only specific modules that require direct
//! memory access (atomic operations, wait queues) allow unsafe code with
//! documented safety invariants.

#![cfg_attr(not(feature = "std"), no_std)]
// Note: unsafe_code is allowed selectively in specific modules that need it
#![warn(missing_docs)]
#![warn(clippy::missing_panics_doc)]

// Import std when available
#[cfg(feature = "std")]
extern crate std;

// Binary std/no_std choice
#[cfg(any(feature = "std", feature = "alloc"))]
extern crate alloc;

// Re-export prelude module publicly
pub use prelude::*;

// Test module for clean architecture migration
pub mod clean_runtime_test;

// Core modules
#[cfg(any(feature = "std", feature = "alloc"))]
pub mod atomic_execution;
#[cfg(any(feature = "std", feature = "alloc"))]
// Disabled due to missing atomic ops methods
// pub mod atomic_execution_safe;
#[cfg(any(feature = "std", feature = "alloc"))]
pub mod atomic_memory_model;
#[cfg(any(feature = "std", feature = "alloc"))]
pub mod branch_prediction;
pub mod cfi_engine;
pub mod core_types;
pub mod execution;
#[cfg(test)]
mod execution_tests;
/// Format bridge interface
pub mod format_bridge;
pub mod func;
pub mod global;
#[cfg(any(feature = "std", feature = "alloc"))]
pub mod interpreter_optimization;
pub mod memory;

// Simplified type system - CRITICAL COMPILATION FIX
pub mod simple_types;
pub mod unified_types;

// Component model integration
pub mod capability_integration;
pub mod component_unified;
pub mod memory_adapter;
#[cfg(test)]
mod memory_adapter_test;
pub mod memory_config_adapter;
pub mod memory_helpers;
/// WebAssembly module representation and management
pub mod module;
pub mod module_builder;
pub mod module_instance;
pub mod prelude;
/// SIMD execution integration adapter
// Disabled due to compilation issues
// pub mod simd_execution_adapter;
pub mod stackless;
pub mod table;
#[cfg(any(feature = "std", feature = "alloc"))]
pub mod thread_manager;
pub mod types;
#[cfg(any(feature = "std", feature = "alloc"))]
pub mod wait_queue;
pub mod wit_debugger_integration;

// Platform-aware runtime and unified memory management
pub mod platform_runtime;

// Bounded infrastructure for static memory allocation
pub mod bounded_runtime_infra;

// Capability-based execution engine
#[cfg(any(feature = "std", feature = "alloc"))]
pub mod engine;

// Instruction parser for bytecode to instruction conversion
pub mod instruction_parser;
#[cfg(test)]
mod instruction_parser_tests;

// Temporary stub modules for parallel development
mod foundation_stubs;
mod component_stubs;

// Runtime state and resource management
pub mod state;
pub mod resources;
pub mod component;

// Import platform abstractions from wrt-foundation
pub use wrt_foundation::platform_abstraction;

// Re-export commonly used types
#[cfg(any(feature = "std", feature = "alloc"))]
pub use atomic_execution::{AtomicMemoryContext, AtomicExecutionStats};
pub use core_types::{CallFrame, ComponentExecutionState, ExecutionContext};
#[cfg(any(feature = "std", feature = "alloc"))]
pub use atomic_memory_model::{
    AtomicMemoryModel, MemoryOrderingPolicy, ConsistencyValidationResult,
    MemoryModelPerformanceMetrics, DataRaceReport, OrderingViolationReport,
};
#[cfg(any(feature = "std", feature = "alloc"))]
pub use branch_prediction::{
    BranchLikelihood, BranchPrediction, FunctionBranchPredictor, ModuleBranchPredictor,
    PredictiveExecutionContext, PredictionStats,
};
pub use cfi_engine::{
    CfiEngineStatistics, CfiExecutionEngine, CfiExecutionResult, CfiViolationPolicy,
    CfiViolationType, ExecutionResult,
};
pub use execution::ExecutionStats;
// Note: ExecutionContext is defined in core_types, not execution
#[cfg(any(feature = "std", feature = "alloc"))]
pub use interpreter_optimization::{
    OptimizedInterpreter, OptimizationStrategy, OptimizationMetrics, 
    BranchOptimizationResult, ExecutionPath,
};
// pub use thread_manager::{
//     ThreadManager, ThreadConfig, ThreadInfo, ThreadState, ThreadExecutionContext,
//     ThreadExecutionStats, ThreadManagerStats, ThreadId,
// };
// pub use wait_queue::{
//     WaitQueueManager, WaitQueue, WaitQueueId, WaitResult, WaitQueueStats,
//     WaitQueueGlobalStats, pause,
// };
// #[cfg(feature = "wit-debug-integration")]
// pub use wit_debugger_integration::{
//     WrtRuntimeState, WrtDebugMemory, DebuggableWrtRuntime,
//     create_wit_enabled_runtime, create_component_metadata, 
//     create_function_metadata, create_type_metadata,
//     ComponentMetadata, FunctionMetadata, TypeMetadata, WitTypeKind,
//     Breakpoint, BreakpointCondition,
// };
pub use func::Function as RuntimeFunction;
pub use prelude::FuncType;
pub use global::Global;
#[cfg(any(feature = "std", feature = "alloc"))]
pub use memory::Memory;
pub use memory_adapter::{MemoryAdapter, SafeMemoryAdapter, StdMemoryProvider};
pub use memory_helpers::ArcMemoryExt;
// pub use module::{
//     Data, Element, Export, ExportItem, ExportKind, Function, Import, Module, OtherExport,
// };
// pub use module_builder::{load_module_from_binary, ModuleBuilder}; // Temporarily disabled
// pub use module_instance::ModuleInstance;
// pub use stackless::{
//     StacklessCallbackRegistry, StacklessEngine, StacklessExecutionState, StacklessFrame,
// }; // Temporarily disabled due to compilation issues
pub use table::Table;

// Re-export platform-aware runtime types - temporarily disabled
// pub use platform_runtime::{PlatformAwareRuntime, PlatformMemoryAdapter, RuntimeMetrics};

/// The WebAssembly memory page size (64KiB)
pub const PAGE_SIZE: usize = 65536;

/// Component Model implementations of runtime interfaces - temporarily disabled
// pub mod component_impl;
/// Component Model trait definitions for runtime interfaces - temporarily disabled
// pub mod component_traits;

// Internal modules
#[cfg(test)]
mod tests;

// Re-export trait definitions - temporarily disabled
// Re-export implementations - temporarily disabled
// #[cfg(all(not(feature = "std"), not(feature = "std")))]
// pub use component_impl::no_alloc::MinimalComponent;
// #[cfg(feature = "std")]
// pub use component_impl::{ComponentRuntimeImpl, DefaultHostFunctionFactory};
// #[cfg(feature = "std")]
// pub use component_traits::{
//     ComponentInstance, ComponentRuntime, HostFunctionFactory,
//     HostFunction as ComponentHostFunction,
// };

// Panic handler is provided by the main binary crate to avoid conflicts

// Panic handler is provided by wrt-platform when needed
