#![allow(unsafe_code)]
// Allow unsafe syscalls for sync operations
// WRT - wrt-platform
// Module: Linux Synchronization Primitives
// SW-REQ-ID: REQ_PLATFORM_001
//
// Copyright (c) 2025 The WRT Project Developers
// Licensed under the MIT license.
// SPDX-License-Identifier: MIT

//! Linux-specific `FutexLike` implementation using direct futex syscalls
//! without libc.
//!
//! This implementation provides wait/notify synchronization using Linux futex
//! system calls directly, supporting no_std/no_alloc environments.

use core::{fmt, marker::PhantomData, ptr::NonNull, sync::atomic::AtomicU32};

use wrt_error::{codes, Error, ErrorCategory, Result};

use crate::sync::{FutexLike, TimeoutResult};

/// Linux syscall numbers for futex
#[cfg(target_arch = "x86_64")]
mod syscalls {
    pub const FUTEX: usize = 202;
}

#[cfg(target_arch = "aarch64")]
mod syscalls {
    pub const FUTEX: usize = 98;
}

/// Futex operation constants
const FUTEX_WAIT: u32 = 0;
const FUTEX_WAKE: u32 = 1;
const FUTEX_PRIVATE_FLAG: u32 = 128; // FUTEX_PRIVATE
const FUTEX_CLOCK_REALTIME: u32 = 256; // FUTEX_CLOCK_REALTIME

/// Combined futex operations (optimized for private futexes)
const FUTEX_WAIT_PRIVATE: u32 = FUTEX_WAIT | FUTEX_PRIVATE_FLAG;
const FUTEX_WAKE_PRIVATE: u32 = FUTEX_WAKE | FUTEX_PRIVATE_FLAG;

/// Timeout structure for futex operations
#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct TimeSpec {
    tv_sec: i64,
    tv_nsec: i64,
}

impl TimeSpec {
    /// Create a new TimeSpec from microseconds
    fn from_microseconds(us: u32) -> Self {
        let secs = us / 1_000_000;
        let nanos = (us % 1_000_000) * 1_000;
        Self { tv_sec: secs as i64, tv_nsec: nanos as i64 }
    }

    /// Create a zero timeout (immediate)
    fn zero() -> Self {
        Self { tv_sec: 0, tv_nsec: 0 }
    }
}

/// A `FutexLike` implementation for Linux using direct futex syscalls.
///
/// This implementation provides wait/notify synchronization using Linux
/// futex system calls with optimizations for private (process-local) futexes.
#[derive(Debug)]
pub struct LinuxFutex {
    /// The atomic value used for synchronization
    value: AtomicU32,
    /// Padding to ensure the value is on its own cache line
    _padding: [u8; 60], // 64 - sizeof(AtomicU32)
    /// Safety: We're carefully managing the lifetime of the value
    _marker: PhantomData<*mut ()>,
}

impl LinuxFutex {
    /// Creates a new `LinuxFutex` with the given initial value.
    pub fn new(initial_value: u32) -> Self {
        Self { value: AtomicU32::new(initial_value), _padding: [0; 60], _marker: PhantomData }
    }

    /// Direct syscall implementation of futex
    unsafe fn futex(
        uaddr: *const u32,
        futex_op: u32,
        val: u32,
        timeout: *const TimeSpec,
        uaddr2: *const u32,
        val3: u32,
    ) -> i32 {
        let result: isize;

        #[cfg(target_arch = "x86_64")]
        core::arch::asm!(
            "syscall",
            inout("rax") syscalls::FUTEX => result,
            in("rdi") uaddr,
            in("rsi") futex_op,
            in("rdx") val,
            in("r10") timeout,
            in("r8") uaddr2,
            in("r9") val3,
            out("rcx") _,
            out("r11") _,
        );

        #[cfg(target_arch = "aarch64")]
        core::arch::asm!(
            "svc #0",
            inout("x8") syscalls::FUTEX => _,
            inout("x0") uaddr => result,
            in("x1") futex_op,
            in("x2") val,
            in("x3") timeout,
            in("x4") uaddr2,
            in("x5") val3,
        );

        result as i32
    }

    /// Wait for the futex value to change from expected
    fn wait_impl(&self, expected: u32, timeout: Option<TimeSpec>) -> Result<TimeoutResult> {
        let addr = &self.value as *const AtomicU32 as *const u32;

        let timeout_ptr = match timeout {
            Some(ref ts) => ts as *const TimeSpec,
            None => core::ptr::null(),
        };

        // Call futex wait
        // SAFETY: We're calling futex with valid parameters. addr points to self.value,
        // which is valid for the lifetime of self.
        let result = unsafe {
            Self::futex(addr, FUTEX_WAIT_PRIVATE, expected, timeout_ptr, core::ptr::null(), 0)
        };

        match result {
            0 => Ok(TimeoutResult::Notified),    // Woken up by notify
            -110 => Ok(TimeoutResult::TimedOut), // ETIMEDOUT
            -11 => {
                // EAGAIN - value changed before we could wait
                Ok(TimeoutResult::Notified)
            }
            -4 => {
                // EINTR - interrupted by signal, treat as spurious wakeup
                Ok(TimeoutResult::Notified)
            }
            _ => Err(Error::new(
                ErrorCategory::System,
                codes::SYNCHRONIZATION_ERROR,
                "Futex wait operation failed",
            )),
        }
    }

    /// Wake up waiters on this futex
    fn wake_impl(&self, count: u32) -> Result<u32> {
        let addr = &self.value as *const AtomicU32 as *const u32;

        // Call futex wake
        // SAFETY: We're calling futex wake with valid parameters.
        let result = unsafe {
            Self::futex(addr, FUTEX_WAKE_PRIVATE, count, core::ptr::null(), core::ptr::null(), 0)
        };

        if result >= 0 {
            Ok(result as u32) // Number of waiters woken up
        } else {
            Err(Error::new(
                ErrorCategory::System,
                codes::SYNCHRONIZATION_ERROR,
                "Futex wake operation failed",
            ))
        }
    }
}

/// Builder for `LinuxFutex`.
#[derive(Debug)]
pub struct LinuxFutexBuilder {
    initial_value: u32,
}

impl Default for LinuxFutexBuilder {
    fn default() -> Self {
        Self { initial_value: 0 }
    }
}

impl LinuxFutexBuilder {
    /// Creates a new builder with default settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the initial value for the futex.
    pub fn with_initial_value(mut self, value: u32) -> Self {
        self.initial_value = value;
        self
    }

    /// Builds and returns a configured `LinuxFutex`.
    pub fn build(self) -> LinuxFutex {
        LinuxFutex::new(self.initial_value)
    }
}

impl FutexLike for LinuxFutex {
    fn wait(&self, expected: u32, timeout_us: Option<u32>) -> Result<TimeoutResult> {
        // Convert timeout to TimeSpec if provided
        let timeout = match timeout_us {
            Some(0) => Some(TimeSpec::zero()), // Immediate timeout
            Some(us) => Some(TimeSpec::from_microseconds(us)),
            None => None, // Infinite timeout
        };

        self.wait_impl(expected, timeout)
    }

    fn notify_one(&self) -> Result<bool> {
        match self.wake_impl(1)? {
            0 => Ok(false), // No waiters were woken
            _ => Ok(true),  // At least one waiter was woken
        }
    }

    fn notify_all(&self) -> Result<u32> {
        // Wake up to i32::MAX waiters (effectively all)
        self.wake_impl(i32::MAX as u32)
    }

    fn load(&self, ordering: core::sync::atomic::Ordering) -> u32 {
        self.value.load(ordering)
    }

    fn store(&self, val: u32, ordering: core::sync::atomic::Ordering) {
        self.value.store(val, ordering);
    }

    fn compare_exchange(
        &self,
        current: u32,
        new: u32,
        success: core::sync::atomic::Ordering,
        failure: core::sync::atomic::Ordering,
    ) -> core::result::Result<u32, u32> {
        self.value.compare_exchange(current, new, success, failure)
    }

    fn compare_exchange_weak(
        &self,
        current: u32,
        new: u32,
        success: core::sync::atomic::Ordering,
        failure: core::sync::atomic::Ordering,
    ) -> core::result::Result<u32, u32> {
        self.value.compare_exchange_weak(current, new, success, failure)
    }

    fn fetch_add(&self, val: u32, ordering: core::sync::atomic::Ordering) -> u32 {
        self.value.fetch_add(val, ordering)
    }

    fn fetch_sub(&self, val: u32, ordering: core::sync::atomic::Ordering) -> u32 {
        self.value.fetch_sub(val, ordering)
    }

    fn fetch_and(&self, val: u32, ordering: core::sync::atomic::Ordering) -> u32 {
        self.value.fetch_and(val, ordering)
    }

    fn fetch_or(&self, val: u32, ordering: core::sync::atomic::Ordering) -> u32 {
        self.value.fetch_or(val, ordering)
    }

    fn fetch_xor(&self, val: u32, ordering: core::sync::atomic::Ordering) -> u32 {
        self.value.fetch_xor(val, ordering)
    }
}

impl fmt::Display for LinuxFutex {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "LinuxFutex({})", self.value.load(core::sync::atomic::Ordering::Relaxed))
    }
}
