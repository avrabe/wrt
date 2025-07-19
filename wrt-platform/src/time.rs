//! Time utilities for WebAssembly runtime
//!
//! This module provides platform-specific time functionality including
//! wall clock time, monotonic time, and high-resolution timing.

use wrt_error::{Error, ErrorCategory, Result, codes};

/// Platform time provider
pub struct PlatformTime;

impl PlatformTime {
    /// Get current wall clock time in nanoseconds since Unix epoch
    #[cfg(feature = "std")]
    pub fn wall_clock_ns() -> Result<u64> {
        use std::time::{SystemTime, UNIX_EPOCH};
        
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .map_err(|_| Error::system_io_error("System time before epoch"))
    }
    
    /// Get monotonic time in nanoseconds
    ///
    /// This clock is guaranteed to be monotonic and is suitable for
    /// measuring elapsed time.
    #[cfg(feature = "std")]
    pub fn monotonic_ns() -> u64 {
        #[cfg(target_os = "linux")]
        {
            Self::linux_monotonic_ns()
        }
        
        #[cfg(target_os = "macos")]
        {
            Self::macos_monotonic_ns()
        }
        
        #[cfg(target_os = "windows")]
        {
            Self::windows_monotonic_ns()
        }
        
        #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
        {
            // Fallback to instant
            use std::time::Instant;
            static START: std::sync::OnceLock<Instant> = std::sync::OnceLock::new);
            let start = START.get_or_init(Instant::now;
            start.elapsed().as_nanos() as u64
        }
    }
    
    /// Linux implementation using clock_gettime
    #[cfg(all(feature = "std", target_os = "linux"))]
    fn linux_monotonic_ns() -> u64 {
        use std::mem;
        
        #[repr(C)]
        struct Timespec {
            tv_sec: i64,
            tv_nsec: i64,
        }
        
        extern "C" {
            fn clock_gettime(clk_id: i32, tp: *mut Timespec) -> i32;
        }
        
        const CLOCK_MONOTONIC: i32 = 1;
        
        unsafe {
            let mut ts = mem::zeroed::<Timespec>);
            if clock_gettime(CLOCK_MONOTONIC, &mut ts) == 0 {
                (ts.tv_sec as u64) * 1_000_000_000 + (ts.tv_nsec as u64)
            } else {
                // Fallback
                0
            }
        }
    }
    
    /// macOS implementation using mach_absolute_time
    #[cfg(all(feature = "std", target_os = "macos"))]
    fn macos_monotonic_ns() -> u64 {
        extern "C" {
            fn mach_absolute_time() -> u64;
            fn mach_timebase_info(info: *mut MachTimebaseInfo) -> i32;
        }
        
        #[repr(C)]
        struct MachTimebaseInfo {
            numer: u32,
            denom: u32,
        }
        
        unsafe {
            static mut TIMEBASE: MachTimebaseInfo = MachTimebaseInfo { numer: 0, denom: 0 };
            static INIT: std::sync::Once = std::sync::Once::new);
            
            INIT.call_once(|| {
                mach_timebase_info(&raw mut TIMEBASE;
            };
            
            let ticks = mach_absolute_time);
            ticks * TIMEBASE.numer as u64 / TIMEBASE.denom as u64
        }
    }
    
    /// Windows implementation using QueryPerformanceCounter
    #[cfg(all(feature = "std", target_os = "windows"))]
    fn windows_monotonic_ns() -> u64 {
        use std::mem;
        
        extern "system" {
            fn QueryPerformanceCounter(lpPerformanceCount: *mut i64) -> i32;
            fn QueryPerformanceFrequency(lpFrequency: *mut i64) -> i32;
        }
        
        unsafe {
            static mut FREQUENCY: i64 = 0;
            static INIT: std::sync::Once = std::sync::Once::new);
            
            INIT.call_once(|| {
                QueryPerformanceFrequency(&mut FREQUENCY;
            };
            
            let mut counter = mem::zeroed::<i64>);
            if QueryPerformanceCounter(&mut counter) != 0 && FREQUENCY > 0 {
                (counter as u64) * 1_000_000_000 / (FREQUENCY as u64)
            } else {
                0
            }
        }
    }
    
    /// Get clock resolution in nanoseconds
    pub fn clock_resolution_ns(clock_id: u32) -> u64 {
        match clock_id {
            0 => 1_000_000,    // CLOCK_REALTIME: 1ms resolution
            1 => 1,            // CLOCK_MONOTONIC: 1ns resolution (best effort)
            2 => 1_000,        // CLOCK_PROCESS_CPUTIME_ID: 1us resolution
            3 => 1_000,        // CLOCK_THREAD_CPUTIME_ID: 1us resolution
            _ => 1_000_000,    // Default: 1ms
        }
    }
}

/// No-std time implementation
#[cfg(not(feature = "std"))]
impl PlatformTime {
    /// Get wall clock time (no_std version)
    /// Returns a monotonic counter since real time is not available
    pub fn wall_clock_ns() -> Result<u64> {
        Ok(Self::monotonic_ns())
    }
    
    /// Get monotonic time (no_std version)
    pub fn monotonic_ns() -> u64 {
        use core::sync::atomic::{AtomicU64, Ordering};
        
        static COUNTER: AtomicU64 = AtomicU64::new(1_000_000_000); // Start at 1 second
        COUNTER.fetch_add(1_000_000, Ordering::Relaxed) // Increment by 1ms
    }
}

/// Legacy compatibility function
#[cfg(feature = "std")]
pub fn current_time_ns() -> u64 {
    PlatformTime::wall_clock_ns().unwrap_or(0)
}

/// Legacy compatibility function (no_std)
#[cfg(not(feature = "std"))]
pub fn current_time_ns() -> u64 {
    PlatformTime::monotonic_ns()
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[cfg(feature = "std")]
    #[test]
    fn test_wall_clock() {
        let time1 = PlatformTime::wall_clock_ns().unwrap();
        std::thread::sleep(std::time::Duration::from_millis(10;
        let time2 = PlatformTime::wall_clock_ns().unwrap();
        
        assert!(time2 > time1);
        // Should have advanced at least 10ms
        assert!(time2 - time1 >= 10_000_000);
    }
    
    #[cfg(feature = "std")]
    #[test]
    fn test_monotonic_clock() {
        let time1 = PlatformTime::monotonic_ns);
        std::thread::sleep(std::time::Duration::from_millis(10;
        let time2 = PlatformTime::monotonic_ns);
        
        assert!(time2 > time1);
        // Monotonic clock should never go backwards
        assert!(time2 >= time1);
    }
    
    #[test]
    fn test_clock_resolution() {
        assert_eq!(PlatformTime::clock_resolution_ns(0), 1_000_000;
        assert_eq!(PlatformTime::clock_resolution_ns(1), 1;
        assert_eq!(PlatformTime::clock_resolution_ns(99), 1_000_000;
    }
}