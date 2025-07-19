//! WASI clocks interface implementation
//!
//! Implements the `wasi:clocks` interface for time operations using WRT's
//! platform abstractions and proven patterns.

use crate::prelude::*;
use crate::capabilities::WasiClockCapabilities;
use crate::host_provider::resource_manager::WasiClockType;
use wrt_platform::time::PlatformTime;
use crate::Value;
use core::any::Any;

/// WASI monotonic clock now operation
///
/// Implements `wasi:clocks/monotonic-clock.now` for monotonic time
pub fn wasi_monotonic_clock_now(
    _target: &mut dyn Any,
    _args: Vec<Value>,
) -> Result<Vec<Value>> {
    // Get monotonic time using platform abstraction
    let nanoseconds: u64 = PlatformTime::monotonic_ns);
    
    Ok(vec![Value::U64(nanoseconds)])
}

/// WASI wall clock now operation
///
/// Implements `wasi:clocks/wall-clock.now` for wall clock time
pub fn wasi_wall_clock_now(
    _target: &mut dyn Any,
    _args: Vec<Value>,
) -> Result<Vec<Value>> {
    // Get wall clock time using platform abstraction
    let total_ns = PlatformTime::wall_clock_ns()
        .map_err(|_| Error::wasi_capability_unavailable("Wall clock not available"))?;
    
    // Convert to seconds and nanoseconds
    let seconds = total_ns / 1_000_000_000;
    let nanoseconds = (total_ns % 1_000_000_000) as u32;
    
    // Return as tuple (seconds, nanoseconds)
    Ok(vec![Value::Tuple(vec![
        Value::U64(seconds),
        Value::U32(nanoseconds),
    ])])
}

/// WASI monotonic clock resolution operation
///
/// Implements `wasi:clocks/monotonic-clock.resolution` for clock precision
pub fn wasi_monotonic_clock_resolution(
    _target: &mut dyn Any,
    _args: Vec<Value>,
) -> Result<Vec<Value>> {
    // Get monotonic clock resolution using platform abstraction
    // For now, return 1 nanosecond resolution
    let resolution = 1u64;
    
    Ok(vec![Value::U64(resolution)])
}

/// WASI wall clock resolution operation
///
/// Implements `wasi:clocks/wall-clock.resolution` for wall clock precision
pub fn wasi_wall_clock_resolution(
    _target: &mut dyn Any,
    _args: Vec<Value>,
) -> Result<Vec<Value>> {
    // Get wall clock resolution using platform abstraction
    // For now, return 1 nanosecond resolution
    let resolution = 1u64;
    
    Ok(vec![Value::U64(resolution)])
}

/// WASI process CPU time operation
///
/// Implements CPU time measurement for the current process
pub fn wasi_process_cpu_time_now(
    _target: &mut dyn Any,
    _args: Vec<Value>,
) -> Result<Vec<Value>> {
    // Get process CPU time using platform abstraction
    // TODO: Implement when platform support is available
    let cpu_time = 0u64;
    
    Ok(vec![Value::U64(cpu_time)])
}

/// WASI thread CPU time operation
///
/// Implements CPU time measurement for the current thread
pub fn wasi_thread_cpu_time_now(
    _target: &mut dyn Any,
    _args: Vec<Value>,
) -> Result<Vec<Value>> {
    // Get thread CPU time using platform abstraction
    // TODO: Implement when platform support is available
    let cpu_time = 0u64;
    
    Ok(vec![Value::U64(cpu_time)])
}

/// Convert nanoseconds to WASI datetime record
///
/// Helper function to convert nanoseconds since Unix epoch to WASI datetime
pub fn nanoseconds_to_datetime(nanoseconds: u64) -> Value {
    let seconds = nanoseconds / 1_000_000_000;
    let nanos = (nanoseconds % 1_000_000_000) as u32;
    
    Value::Record(vec![
        ("seconds".to_string(), Value::U64(seconds)),
        ("nanoseconds".to_string(), Value::U32(nanos)),
    ])
}

/// Convert WASI datetime record to nanoseconds
///
/// Helper function to convert WASI datetime to nanoseconds since Unix epoch
pub fn datetime_to_nanoseconds(datetime: &Value) -> Result<u64> {
    match datetime {
        Value::Record(fields) => {
            let mut seconds = 0u64;
            let mut nanoseconds = 0u32;
            
            for (key, value) in fields {
                match key.as_str() {
                    "seconds" => {
                        if let Value::U64(s) = value {
                            seconds = *s;
                        }
                    }
                    "nanoseconds" => {
                        if let Value::U32(ns) = value {
                            nanoseconds = *ns;
                        }
                    }
                    _ => {} // Ignore unknown fields
                }
            }
            
            Ok(seconds * 1_000_000_000 + nanoseconds as u64)
        }
        _ => Err(Error::wasi_invalid_fd("Invalid datetime format")),
    }
}

/// Get current time with specified clock capabilities
///
/// Helper function that respects WASI clock capabilities
pub fn get_time_with_capabilities(
    clock_type: WasiClockType,
    capabilities: &WasiClockCapabilities,
) -> Result<u64> {
    use crate::host_provider::resource_manager::WasiClockType;
    
    match clock_type {
        WasiClockType::Realtime => {
            if !capabilities.realtime_access {
                return Err(Error::wasi_permission_denied("Realtime clock access denied";
            }
            
            let total_ns = PlatformTime::wall_clock_ns()
                .map_err(|_| Error::wasi_capability_unavailable("Wall clock not available"))?;
            
            Ok(total_ns)
        }
        WasiClockType::Monotonic => {
            if !capabilities.monotonic_access {
                return Err(Error::wasi_permission_denied("Monotonic clock access denied";
            }
            
            Ok(PlatformTime::monotonic_ns())
        }
        WasiClockType::ProcessCpuTime => {
            if !capabilities.process_cputime_access {
                return Err(Error::wasi_permission_denied("Process CPU time access denied";
            }
            
            // TODO: Implement when platform support is available
            Ok(0u64)
        }
        WasiClockType::ThreadCpuTime => {
            if !capabilities.thread_cputime_access {
                return Err(Error::wasi_permission_denied("Thread CPU time access denied";
            }
            
            // TODO: Implement when platform support is available
            Ok(0u64)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_wasi_monotonic_clock_now() -> Result<()> {
        let result = wasi_monotonic_clock_now(&mut (), vec![])?;
        assert_eq!(result.len(), 1;
        
        // Should return a u64 timestamp
        if let Value::U64(timestamp) = &result[0] {
            // Timestamp should be non-zero (current time)
            assert!(*timestamp > 0);
        } else {
            panic!("Expected u64 timestamp";
        }
        
        Ok(())
    }
    
    #[test]
    fn test_wasi_wall_clock_now() -> Result<()> {
        let result = wasi_wall_clock_now(&mut (), vec![])?;
        assert_eq!(result.len(), 1;
        
        // Should return a tuple of (seconds, nanoseconds)
        if let Value::Tuple(time_parts) = &result[0] {
            assert_eq!(time_parts.len(), 2;
            
            // First should be seconds (u64)
            assert!(matches!(time_parts[0], Value::U64(_));
            // Second should be nanoseconds (u32)
            assert!(matches!(time_parts[1], Value::U32(_));
            
            // Verify nanoseconds are in valid range
            if let Value::U32(nanos) = &time_parts[1] {
                assert!(*nanos < 1_000_000_000);
            }
        } else {
            panic!("Expected tuple of (seconds, nanoseconds)";
        }
        
        Ok(())
    }
    
    #[test]
    fn test_nanoseconds_to_datetime_conversion() {
        let nanoseconds = 1_234_567_890_123_456_789u64;
        let datetime = nanoseconds_to_datetime(nanoseconds;
        
        if let Value::Record(fields) = datetime {
            assert_eq!(fields.len(), 2;
            
            let mut seconds_found = false;
            let mut nanos_found = false;
            
            for (key, value) in fields {
                match key.as_str() {
                    "seconds" => {
                        if let Value::U64(s) = value {
                            assert_eq!(s, 1_234_567_890;
                            seconds_found = true;
                        }
                    }
                    "nanoseconds" => {
                        if let Value::U32(ns) = value {
                            assert_eq!(ns, 123_456_789;
                            nanos_found = true;
                        }
                    }
                    _ => panic!("Unexpected field: {}", key),
                }
            }
            
            assert!(seconds_found && nanos_found);
        } else {
            panic!("Expected record";
        }
    }
    
    #[test]
    fn test_datetime_to_nanoseconds_conversion() -> Result<()> {
        let datetime = Value::Record(vec![
            ("seconds".to_string(), Value::U64(1_234_567_890)),
            ("nanoseconds".to_string(), Value::U32(123_456_789)),
        ];
        
        let nanoseconds = datetime_to_nanoseconds(&datetime)?;
        assert_eq!(nanoseconds, 1_234_567_890_123_456_789;
        
        Ok(())
    }
    
    #[test]
    fn test_time_with_capabilities() -> Result<()> {
        use crate::host_provider::resource_manager::WasiClockType;
        
        // Test with allowed access
        let capabilities = WasiClockCapabilities {
            realtime_access: true,
            monotonic_access: true,
            process_cputime_access: false,
            thread_cputime_access: false,
        };
        
        // Should succeed for allowed clocks
        let _realtime = get_time_with_capabilities(WasiClockType::Realtime, &capabilities)?;
        let _monotonic = get_time_with_capabilities(WasiClockType::Monotonic, &capabilities)?;
        
        // Should fail for denied clocks
        let result = get_time_with_capabilities(WasiClockType::ProcessCpuTime, &capabilities;
        assert!(result.is_err();
        
        Ok(())
    }
}