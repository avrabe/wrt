==============
Platform Layer
==============

This section describes the platform abstraction layer introduced to support multiple operating systems and execution environments.

Background & Motivation
------------------------

The original WRT interpreter assumed a POSIX-like OS. The new AOT translator requires a runtime helper library that can run on diverse environments (macOS, Linux, QNX, Zephyr, bare-metal) and expose a stable C ABI.

Modern Arm hardening features (PAC/BTI, MTE) also require OS-specific memory mapping flags (e.g., `PROT_MTE` on Linux).

This rework introduces a platform abstraction layer (`wrt_platform`) to isolate OS-specific code behind compile-time feature flags.

Design
------

The platform layer provides abstract traits for OS-dependent functionality:

*   **`PageAllocator`**: Handles allocation, growth, and protection of memory pages (typically 64 KiB Wasm pages). Implementations use OS-specific mechanisms like `mmap` (Linux/macOS/QNX) or custom allocators (bare-metal).
*   **`FutexLike`**: Provides synchronization primitives similar to Linux futexes, abstracting over mechanisms like `futex` (Linux), `__ulock_wait` (macOS), `SyncCondvar*` (QNX), `k_futex_wait` (Zephyr), or spin-loops (bare-metal).

Implementations are selected at compile time using feature flags (e.g., `platform-linux`, `platform-macos`).

The `wrt-helper` library exposes C-ABI functions (like `wrt_memory_grow`, `wrt_atomic_wait`) that internally use these platform traits.

Supported Platforms (Phased Implementation)
-------------------------------------------

1.  **macOS arm64**: `mmap` for memory, `__ulock_wait` for futex.
2.  **Linux aarch64**: `mmap` (with `PROT_MTE` if enabled), `futex` syscalls.
3.  **QNX 7.1 aarch64**: `mmap` (with `MAP_LAZY`), `SyncCondvar*` APIs.
4.  **Bare-metal (EL1-N) / Zephyr**: Bump allocator, WFE/SEVL spin futex / `k_futex_*` primitives.

References
----------

*   Zephyr futex API: `k_futex_wait/k_futex_wake` docs (docs.zephyrproject.org) 