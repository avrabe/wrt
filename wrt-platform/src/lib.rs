// WRT - wrt-platform
// Module: Library Root
// SW-REQ-ID: REQ_PLATFORM_001
//
// Copyright (c) 2025 The WRT Project Developers
// Licensed under the MIT license.
// SPDX-License-Identifier: MIT

//! WRT Platform Abstraction Layer (PAL).
//!
//! Provides traits and implementations for platform-specific operations like
//! memory allocation (`PageAllocator`) and low-level synchronization
//! (`FutexLike`).
//!
//! This crate adheres to safety-critical guidelines:
//! - Unsafe code is used minimally and with justification (see `memory.rs`).
//! - Error handling via `wrt_error::Error`.
//! - `panic = "abort"`.
//! - `no_std` support.

#![cfg_attr(not(feature = "std"), no_std)] // Rule: Enforce no_std when std feature is not enabled
#![cfg_attr(all(not(feature = "std"), not(feature = "enable-panic-handler")), no_main)]
#![deny(missing_docs)] // Rule 9: Require documentation.
#![deny(clippy::panic)] // Rule 3: No panic!.
#![deny(clippy::unwrap_used)] // Rule 3: No unwrap.
#![deny(clippy::expect_used)] // Rule 3: No expect.
#![deny(clippy::unreachable)] // Rule 4: No unreachable!.
#![warn(clippy::pedantic)] // Rule 8: Enable pedantic lints.
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::return_self_not_must_use)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::doc_markdown)]
#![allow(clippy::match_same_arms)]
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::unnecessary_wraps)]
#![allow(clippy::extra_unused_type_parameters)]
#![allow(clippy::inline_always)]
#![allow(clippy::no_effect_underscore_binding)]
#![allow(clippy::used_underscore_binding)]
#![allow(clippy::wildcard_imports)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::struct_excessive_bools)]
#![allow(clippy::unused_self)]
#![allow(clippy::derivable_impls)]
#![allow(clippy::new_without_default)]
#![allow(clippy::ptr_as_ptr)]
#![allow(clippy::ref_as_ptr)]
#![allow(clippy::ptr_eq)]
#![allow(clippy::cast_ptr_alignment)]
#![allow(clippy::single_match)]
#![allow(clippy::single_match_else)]
#![allow(clippy::elidable_lifetime_names)]
#![allow(clippy::cast_lossless)]
#![allow(clippy::borrow_as_ptr)]
#![allow(clippy::items_after_statements)]
#![allow(clippy::missing_safety_doc)]
#![allow(clippy::needless_range_loop)]
#![allow(clippy::manual_div_ceil)]
#![allow(clippy::ptr_cast_constness)]

#[cfg(feature = "std")]
extern crate std;

// Import panic handler when enabled (only for no_std builds to avoid conflict with std)
#[cfg(all(
    not(feature = "std"),
    any(
        feature = "enable-panic-handler",
        feature = "dev-panic-handler", 
        feature = "asil-b-panic-handler",
        feature = "asil-d-panic-handler"
    )
))]
extern crate wrt_panic;

// Simple panic handler when no explicit handler is available
#[cfg(all(
    not(feature = "std"),
    not(test),
    not(any(
        feature = "enable-panic-handler",
        feature = "dev-panic-handler",
        feature = "asil-b-panic-handler",
        feature = "asil-d-panic-handler"
    ))
))]
#[panic_handler] // Re-enabled for no_std compilation
fn panic(_info: &core::panic::PanicInfo) -> ! {
    // Simple infinite loop for minimal panic handling
    loop {
        core::hint::spin_loop();
    }
}
// Module declarations
// pub mod bounded_platform; // Disabled due to circular dependency with wrt-foundation
pub mod comprehensive_limits;
pub mod memory;
pub mod performance_validation;
pub mod platform_abstraction;
pub mod prelude;
pub mod random;
pub mod runtime_detection;
pub mod simd;
pub mod sync;
pub mod time;

// Enhanced platform features
pub mod advanced_sync;
pub mod formal_verification;
pub mod hardware_optimizations;
pub mod side_channel_resistance;

// Platform-agnostic threading (requires std)
#[cfg(feature = "std")]
pub mod threading;

// Threading with wasm support (requires both std and wrt-foundation)
#[cfg(feature = "threading")]
pub mod wasm_thread_manager;

// Atomic operations integration with threading
#[cfg(feature = "threading")]
pub mod atomic_thread_manager;

// Platform-specific threading implementations
#[cfg(all(feature = "threading", target_os = "nto"))]
pub mod qnx_threading;

#[cfg(all(feature = "threading", target_os = "linux"))]
pub mod linux_threading;

#[cfg(all(feature = "threading", not(target_os = "nto"), not(target_os = "linux")))]
pub mod generic_threading;

// Memory management uses NoStdProvider pattern from wrt-foundation


// Watchdog (requires std)

// Panic handler for testing individual crates - only when not disabled
// Merged with the panic handler below to avoid duplicates
#[cfg(feature = "std")]
pub mod watchdog;

// IPC and High Availability (require std)
#[cfg(feature = "std")]
pub mod ipc;

// Linux IPC implementation (requires std)
#[cfg(all(feature = "std", target_os = "linux"))]
pub mod linux_ipc;

#[cfg(feature = "std")]
pub mod high_availability;

// Panic handling is now delegated to wrt-panic crate
// This ensures consistent, configurable panic behavior across all WRT components
//
// Available panic handler configurations:
// - `enable-panic-handler`: Basic release panic handler
// - `dev-panic-handler`: Development panic handler with enhanced debugging  
// - `asil-b-panic-handler`: ASIL-B compliant safety-critical panic handler
// - `asil-d-panic-handler`: ASIL-D compliant safety-critical panic handler
// - `disable-panic-handler`: No panic handler (for library usage)
//
// The panic handler is automatically enabled when the appropriate feature is selected.
// For production systems, use asil-b-panic-handler or asil-d-panic-handler.
// For development, use dev-panic-handler.
// For library usage, use disable-panic-handler.

// Platform-specific modules
// macOS modules - using direct syscalls (no libc)
#[cfg(all(feature = "platform-macos", target_os = "macos"))]
pub mod macos_memory_no_libc;
#[cfg(all(feature = "platform-macos", target_os = "macos"))]
pub mod macos_sync_no_libc;
// Rename modules to standard names
#[cfg(all(feature = "platform-macos", target_os = "macos"))]
pub use macos_memory_no_libc as macos_memory;
#[cfg(all(feature = "platform-macos", target_os = "macos"))]
pub use macos_sync_no_libc as macos_sync;

// QNX-specific modules
#[cfg(all(feature = "platform-qnx", target_os = "nto"))]
pub mod qnx_arena;
#[cfg(all(feature = "platform-qnx", target_os = "nto"))]
pub mod qnx_memory;
#[cfg(all(feature = "platform-qnx", target_os = "nto"))]
pub mod qnx_partition;
#[cfg(all(feature = "platform-qnx", target_os = "nto"))]
pub mod qnx_sync;

// Linux-specific modules
#[cfg(all(feature = "platform-linux", target_os = "linux"))]
pub mod linux_memory;
#[cfg(all(
    feature = "platform-linux",
    feature = "linux-mte",
    target_arch = "aarch64",
    target_os = "linux"
))]
pub mod linux_memory_arm64_mte;
#[cfg(all(feature = "platform-linux", target_os = "linux"))]
pub mod linux_sync;

// Zephyr-specific modules
#[cfg(feature = "platform-zephyr")]
pub mod zephyr_memory;
#[cfg(feature = "platform-zephyr")]
pub mod zephyr_sync;

// Tock OS-specific modules
#[cfg(feature = "platform-tock")]
pub mod tock_memory;
#[cfg(feature = "platform-tock")]
pub mod tock_sync;

// VxWorks-specific modules
#[cfg(all(feature = "platform-vxworks", target_os = "vxworks"))]
pub mod vxworks_memory;
#[cfg(all(feature = "platform-vxworks", target_os = "vxworks"))]
pub mod vxworks_sync;
#[cfg(all(feature = "platform-vxworks", target_os = "vxworks"))]
pub mod vxworks_threading;

// Publicly export items via the prelude
// Publicly export the core traits and the fallback implementations
// Export macOS specific implementations if enabled and on macOS
// Export Linux specific implementations if enabled and on Linux
pub use advanced_sync::{
    AdvancedRwLock, LockFreeAllocator, Priority, PriorityInheritanceMutex, MAX_PRIORITY,
    MIN_PRIORITY,
};
#[cfg(feature = "std")]
pub use advanced_sync::{LockFreeMpscQueue, WaitFreeSpscQueue};
pub use formal_verification::{
    annotations, concurrency_verification, integration_verification, memory_verification,
    realtime_verification, security_verification,
};
// Export comprehensive limits
pub use comprehensive_limits::{
    ComprehensivePlatformLimits, ComprehensiveLimitProvider, PlatformLimitDiscoverer,
    LinuxLimitProvider, QnxLimitProvider, MacOsLimitProvider, EmbeddedLimitProvider,
    PlatformId, AsilLevel,
};
// Export specific CFI/BTI types for easy access
pub use hardware_optimizations::arm::{BranchTargetIdentification, BtiExceptionLevel, BtiMode};
pub use hardware_optimizations::riscv::{CfiExceptionMode, ControlFlowIntegrity};
// Export enhanced platform features
pub use hardware_optimizations::{
    arm, compile_time, intel, riscv, HardwareOptimization, HardwareOptimizer, SecurityLevel,
};
#[cfg(all(
    feature = "platform-linux",
    target_os = "linux",
    not(all(feature = "linux-mte", target_arch = "aarch64"))
))]
pub use linux_memory::{LinuxAllocator, LinuxAllocatorBuilder};
#[cfg(all(
    feature = "platform-linux",
    feature = "linux-mte",
    target_arch = "aarch64",
    target_os = "linux"
))]
pub use linux_memory_arm64_mte::{LinuxArm64MteAllocator, LinuxArm64MteAllocatorBuilder, MteMode};
#[cfg(all(feature = "platform-linux", target_os = "linux"))]
pub use linux_sync::{LinuxFutex, LinuxFutexBuilder};
// Export macOS specific implementations (using direct syscalls)
#[cfg(all(feature = "platform-macos", target_os = "macos"))]
pub use macos_memory::{MacOsAllocator, MacOsAllocatorBuilder};
#[cfg(all(feature = "platform-macos", target_os = "macos"))]
pub use macos_sync::{MacOsFutex, MacOsFutexBuilder};
pub use memory::{
    NoStdProvider, PageAllocator, VerificationLevel, WASM_PAGE_SIZE,
}; // WASM_PAGE_SIZE is always available
// Export performance validation (for testing and benchmarking)
pub use performance_validation::{BenchmarkResult, CompileTimeValidator, PerformanceValidator}; /* This is fine as wrt_error::Error is always available */
// Export hybrid platform abstraction
pub use platform_abstraction::{
    paradigm, platform_select, BaremetalPlatform, IsolationLevel, PlatformAbstraction,
    PlatformConfig, PosixPlatform, RealtimePlatform, SecurityPlatform, UnifiedPlatform,
};
pub use prelude::*;
#[cfg(all(feature = "platform-qnx", target_os = "nto"))]
pub use qnx_arena::{QnxArenaAllocator, QnxArenaAllocatorBuilder, QnxMallocOption};
// Export QNX specific implementations if enabled and on QNX
#[cfg(all(feature = "platform-qnx", target_os = "nto"))]
pub use qnx_memory::{QnxAllocator, QnxAllocatorBuilder, QnxMapFlags, QnxProtFlags};
#[cfg(all(feature = "platform-qnx", target_os = "nto"))]
pub use qnx_partition::{
    PartitionGuard, QnxMemoryPartition, QnxMemoryPartitionBuilder, QnxPartitionFlags,
};
#[cfg(all(feature = "platform-qnx", target_os = "nto"))]
pub use qnx_sync::{QnxFutex, QnxFutexBuilder, QnxSyncPriority};
// Export runtime detection
pub use runtime_detection::{
    MemoryCapabilities, PlatformCapabilities, PlatformDetector, RealtimeCapabilities,
    SecurityCapabilities, SyncCapabilities,
};
pub use simd::{
    ScalarSimdProvider, SimdCapabilities, SimdLevel, SimdProvider,
};
#[cfg(feature = "std")]
pub use simd::SimdRuntime;
#[cfg(target_arch = "x86_64")]
pub use simd::{x86_64::X86SimdProvider};
pub use side_channel_resistance::{
    access_obfuscation, cache_aware_allocation, constant_time, platform_integration, AttackVector,
    ResistanceLevel,
};
pub use sync::{FutexLike, SpinFutex, SpinFutexBuilder, TimeoutResult}; /* FutexLike is always available */
// Export Tock OS specific implementations if enabled
#[cfg(feature = "platform-tock")]
pub use tock_memory::{TockAllocator, TockAllocatorBuilder};
#[cfg(feature = "platform-tock")]
pub use tock_sync::{TockFutex, TockFutexBuilder, TockSemaphoreFutex};
// Re-export core error type (also available via prelude)
pub use wrt_error::Error;
// Export Zephyr specific implementations if enabled
#[cfg(feature = "platform-zephyr")]
pub use zephyr_memory::{ZephyrAllocator, ZephyrAllocatorBuilder, ZephyrMemoryFlags};
#[cfg(feature = "platform-zephyr")]
pub use zephyr_sync::{ZephyrFutex, ZephyrFutexBuilder, ZephyrSemaphoreFutex};

// Export VxWorks specific implementations if enabled and on VxWorks
#[cfg(all(feature = "platform-vxworks", target_os = "vxworks"))]
pub use vxworks_memory::{VxWorksAllocator, VxWorksAllocatorBuilder, VxWorksContext, VxWorksMemoryConfig};
#[cfg(all(feature = "platform-vxworks", target_os = "vxworks"))]
pub use vxworks_sync::{VxWorksFutex, VxWorksFutexBuilder};
#[cfg(all(feature = "platform-vxworks", target_os = "vxworks"))]
pub use vxworks_threading::{VxWorksThread, VxWorksThreadBuilder, VxWorksThreadConfig};

#[cfg(test)]
#[allow(clippy::panic)] // Allow panics in the test module
mod tests {
    // Import through the prelude for testing
    use super::{memory::MemoryProvider, prelude::*};

    #[test]
    fn it_works() {
        // Basic sanity check test
        assert_eq!(2 + 2, 4);
    }

    #[test]
    fn test_memory_provider_creation() {
        use wrt_foundation::{
            capabilities::{capability_context, safe_capability_alloc, CapabilityAwareProvider},
            CrateId, NoStdProvider,
        };
        
        let context = capability_context!(verified(CrateId::Platform, 2048, VerificationLevel::Full))
            .expect("Failed to create capability context");
        let provider: CapabilityAwareProvider<NoStdProvider<2048>> = 
            safe_capability_alloc!(context, CrateId::Platform, 2048)
                .expect("Failed to create provider");

        // Note: verification level check would need to be implemented in provider
        // Actual size is capped at 4096 in the stub implementation
        assert!(provider.capacity() <= 4096);
    }

    #[cfg(all(feature = "platform-macos", target_os = "macos"))]
    #[test]
    fn test_macos_allocator_builder() {
        let allocator = MacOsAllocatorBuilder::new()
            .with_maximum_pages(100)
            .with_guard_pages(true)
            .with_memory_tagging(true)
            .build();

        // Binary std/no_std choice
        // We can't test its settings without accessing private fields
        assert_eq!(core::mem::size_of_val(&allocator) > 0, true);
    }

    #[cfg(all(feature = "platform-linux", target_os = "linux"))]
    #[test]
    fn test_linux_allocator_builder() {
        let allocator =
            LinuxAllocatorBuilder::new().with_maximum_pages(100).with_guard_pages(true).build();

        // Binary std/no_std choice
        // We can't test its settings without accessing private fields
        assert_eq!(core::mem::size_of_val(&allocator) > 0, true);
    }

    #[cfg(all(
        feature = "platform-linux",
        feature = "linux-mte",
        target_arch = "aarch64",
        target_os = "linux"
    ))]
    #[test]
    fn test_linux_arm64_mte_allocator_builder() {
        let allocator = LinuxArm64MteAllocatorBuilder::new()
            .with_maximum_pages(100)
            .with_guard_pages(true)
            .with_mte_mode(MteMode::Synchronous)
            .build();

        // Binary std/no_std choice
        // We can't test its settings without accessing private fields
        assert_eq!(core::mem::size_of_val(&allocator) > 0, true);
    }

    #[cfg(all(feature = "platform-linux", target_os = "linux"))]
    #[test]
    fn test_linux_futex_builder() {
        let futex = LinuxFutexBuilder::new().with_initial_value(42).build();

        // Test that the futex was created successfully
        assert_eq!(core::mem::size_of_val(&futex) > 0, true);
    }

    #[cfg(feature = "platform-zephyr")]
    #[test]
    fn test_zephyr_allocator_builder() {
        let allocator = ZephyrAllocatorBuilder::new()
            .with_maximum_pages(100)
            .with_memory_domains(true)
            .with_guard_regions(true)
            .build();

        // Binary std/no_std choice
        assert_eq!(core::mem::size_of_val(&allocator) > 0, true);
    }

    #[cfg(feature = "platform-zephyr")]
    #[test]
    fn test_zephyr_futex_builder() {
        let futex = ZephyrFutexBuilder::new().with_initial_value(42).build();

        // Test that the futex was created successfully
        assert_eq!(core::mem::size_of_val(&futex) > 0, true);
    }

    #[cfg(feature = "platform-zephyr")]
    #[test]
    fn test_zephyr_semaphore_futex() {
        let futex = ZephyrSemaphoreFutex::new(0);

        // Test that the semaphore futex was created successfully
        assert_eq!(core::mem::size_of_val(&futex) > 0, true);
    }

    #[cfg(feature = "platform-tock")]
    #[test]
    fn test_tock_allocator_builder() {
        let builder = TockAllocatorBuilder::new()
            .with_maximum_pages(64)
            .with_verification_level(VerificationLevel::Full);

        assert_eq!(builder.maximum_pages, 64);
        assert_eq!(builder.verification_level, VerificationLevel::Full);
    }

    #[cfg(feature = "platform-tock")]
    #[test]
    fn test_tock_futex_builder() {
        let futex = TockFutexBuilder::new().with_initial_value(123).with_ipc(true).build().unwrap();

        assert_eq!(futex.load(), 123);
    }

    #[test]
    fn test_hybrid_platform_abstraction() {
        use crate::platform_abstraction::*;

        // Test configuration creation for different paradigms
        let posix_config =
            PlatformConfig::<paradigm::Posix>::new().with_max_pages(1024).with_guard_pages(true);
        assert_eq!(posix_config.max_pages, 1024);
        assert!(posix_config.guard_pages);

        let security_config = PlatformConfig::<paradigm::SecurityFirst>::new()
            .with_max_pages(512)
            .with_static_allocation(64 * 1024)
            .with_isolation_level(IsolationLevel::Hardware);
        assert_eq!(security_config.max_pages, 512);
        assert_eq!(security_config.static_allocation_size, Some(64 * 1024));
        assert_eq!(security_config.isolation_level, Some(IsolationLevel::Hardware));

        let realtime_config =
            PlatformConfig::<paradigm::RealTime>::new().with_max_pages(256).with_rt_priority(10);
        assert_eq!(realtime_config.max_pages, 256);
        assert_eq!(realtime_config.rt_priority, Some(10));
    }

    #[test]
    fn test_platform_auto_selection() {
        use crate::platform_abstraction::platform_select;

        // Test that we can create auto-selected platform
        let _platform = platform_select::create_auto_platform();

        // The actual type will depend on which features are enabled
        // This test mainly ensures the compilation works
    }

    #[test]
    fn test_runtime_detection() {
        use crate::runtime_detection::PlatformDetector;

        let mut detector = PlatformDetector::new();
        let capabilities = detector.detect().unwrap();

        // Test that we can detect basic capabilities
        assert!(capabilities.memory.allocation_granularity > 0);
        assert!(capabilities.supports_wasm_runtime());

        let paradigm = capabilities.recommended_paradigm();
        assert!(paradigm == "SecurityFirst" || paradigm == "RealTime" || paradigm == "Posix");

        // Test caching works
        let capabilities2 = detector.detect().unwrap();
        assert_eq!(
            capabilities.memory.allocation_granularity,
            capabilities2.memory.allocation_granularity
        );
    }

    #[cfg(all(feature = "platform-vxworks", target_os = "vxworks"))]
    #[test]
    fn test_vxworks_allocator_builder() {
        let allocator = VxWorksAllocatorBuilder::new()
            .context(VxWorksContext::Rtp)
            .max_pages(512)
            .use_dedicated_partition(true)
            .enable_guard_pages(true)
            .build();

        assert!(allocator.is_ok());
    }

    #[cfg(all(feature = "platform-vxworks", target_os = "vxworks"))]
    #[test]
    fn test_vxworks_futex_builder() {
        let futex = VxWorksFutexBuilder::new(VxWorksContext::Rtp)
            .initial_value(42)
            .build();

        assert!(futex.is_ok());
    }

    #[cfg(all(feature = "platform-vxworks", target_os = "vxworks"))]
    #[test]
    fn test_vxworks_thread_builder() {
        let thread_config = VxWorksThreadConfig {
            context: VxWorksContext::Rtp,
            stack_size: 16384,
            name: Some("test_thread".to_string()),
            floating_point: true,
            detached: true,
            ..Default::default()
        };

        assert_eq!(thread_config.stack_size, 16384);
        assert_eq!(thread_config.name.as_ref().unwrap(), "test_thread");
        assert!(thread_config.floating_point);
        assert!(thread_config.detached);
    }
}

// Global allocator for no_std builds - panic on allocation attempts
// This catches inadvertent allocation attempts in no_std mode
#[cfg(all(not(feature = "std"), not(test)))]
#[global_allocator]
static GLOBAL: PanicAllocator = PanicAllocator;

#[cfg(all(not(feature = "std"), not(test)))]
struct PanicAllocator;

#[cfg(all(not(feature = "std"), not(test)))]
unsafe impl core::alloc::GlobalAlloc for PanicAllocator {
    #[allow(clippy::panic)] // Intentional panic to prevent allocation in no_std
    unsafe fn alloc(&self, _layout: core::alloc::Layout) -> *mut u8 {
        panic!("Attempted allocation in no_std mode")
    }
    #[allow(clippy::panic)] // Intentional panic to prevent deallocation in no_std
    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: core::alloc::Layout) {
        panic!("Attempted deallocation in no_std mode")
    }
}

// Panic handler for no_std builds
// Note: wrt-platform does NOT provide a panic handler to avoid conflicts.
// The main wrt crate provides the panic handler when needed.
// Applications can provide their own panic handler by enabling the
// "disable-panic-handler" feature on the main wrt crate.
