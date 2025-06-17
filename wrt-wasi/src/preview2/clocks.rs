//! WASI clocks interface implementation
//!
//! Implements the `wasi:clocks` interface for time operations using WRT's
//! platform abstractions and proven patterns.

use crate::prelude::*;
use wrt_platform::time::PlatformTime;
use wrt_component::values::Value;
use core::any::Any;

/// WASI monotonic clock now operation
///
/// Implements `wasi:clocks/monotonic-clock.now` for monotonic time
pub fn wasi_monotonic_clock_now(
    _target: &mut dyn Any,
    _args: Vec<Value>,
) -> Result<Vec<Value>> {
    // Get monotonic time using platform abstraction
    let time = PlatformTime::new();
    let nanoseconds = time.monotonic_now()
        .map_err(|_| Error::new(
            ErrorCategory::Runtime,
            codes::WASI_CAPABILITY_UNAVAILABLE,
            kinds::WasiResourceError("Monotonic clock not available")
        ))?;
    
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
    let time = PlatformTime::new();
    let (seconds, nanoseconds) = time.wall_clock_now()
        .map_err(|_| Error::new(
            ErrorCategory::Runtime,
            codes::WASI_CAPABILITY_UNAVAILABLE,
            kinds::WasiResourceError("Wall clock not available")
        ))?;
    
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
    let time = PlatformTime::new();
    let resolution = time.monotonic_resolution()
        .map_err(|_| Error::new(
            ErrorCategory::Runtime,
            codes::WASI_CAPABILITY_UNAVAILABLE,
            kinds::WasiResourceError("Monotonic clock resolution not available")
        ))?;
    
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
    let time = PlatformTime::new();
    let resolution = time.wall_clock_resolution()
        .map_err(|_| Error::new(
            ErrorCategory::Runtime,
            codes::WASI_CAPABILITY_UNAVAILABLE,
            kinds::WasiResourceError("Wall clock resolution not available")
        ))?;
    
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
    let time = PlatformTime::new();
    let cpu_time = time.process_cpu_time()
        .map_err(|_| Error::new(
            ErrorCategory::Runtime,
            codes::WASI_CAPABILITY_UNAVAILABLE,
            kinds::WasiResourceError("Process CPU time not available")
        ))?;
    
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
    let time = PlatformTime::new();
    let cpu_time = time.thread_cpu_time()
        .map_err(|_| Error::new(
            ErrorCategory::Runtime,
            codes::WASI_CAPABILITY_UNAVAILABLE,
            kinds::WasiResourceError("Thread CPU time not available")
        ))?;
    
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
        _ => Err(Error::new(
            ErrorCategory::Parse,
            codes::WASI_INVALID_FD,
            kinds::WasiResourceError("Invalid datetime format")
        )),
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
                return Err(Error::new(
                    ErrorCategory::Resource,
                    codes::WASI_PERMISSION_DENIED,
                    kinds::WasiPermissionError("Realtime clock access denied")
                ));
            }
            
            let time = PlatformTime::new();
            let (seconds, nanoseconds) = time.wall_clock_now()
                .map_err(|_| Error::new(
                    ErrorCategory::Runtime,
                    codes::WASI_CAPABILITY_UNAVAILABLE,
                    kinds::WasiResourceError("Wall clock not available")
                ))?;
            
            Ok(seconds * 1_000_000_000 + nanoseconds as u64)
        }
        WasiClockType::Monotonic => {
            if !capabilities.monotonic_access {
                return Err(Error::new(
                    ErrorCategory::Resource,
                    codes::WASI_PERMISSION_DENIED,
                    kinds::WasiPermissionError("Monotonic clock access denied")
                ));
            }
            
            let time = PlatformTime::new();
            time.monotonic_now()
                .map_err(|_| Error::new(
                    ErrorCategory::Runtime,
                    codes::WASI_CAPABILITY_UNAVAILABLE,
                    kinds::WasiResourceError("Monotonic clock not available")
                ))
        }
        WasiClockType::ProcessCpuTime => {
            if !capabilities.process_cputime_access {
                return Err(Error::new(
                    ErrorCategory::Resource,
                    codes::WASI_PERMISSION_DENIED,
                    kinds::WasiPermissionError("Process CPU time access denied")
                ));
            }
            
            let time = PlatformTime::new();
            time.process_cpu_time()
                .map_err(|_| Error::new(
                    ErrorCategory::Runtime,
                    codes::WASI_CAPABILITY_UNAVAILABLE,
                    kinds::WasiResourceError("Process CPU time not available")
                ))
        }
        WasiClockType::ThreadCpuTime => {
            if !capabilities.thread_cputime_access {
                return Err(Error::new(
                    ErrorCategory::Resource,
                    codes::WASI_PERMISSION_DENIED,
                    kinds::WasiPermissionError("Thread CPU time access denied")
                ));
            }
            
            let time = PlatformTime::new();
            time.thread_cpu_time()
                .map_err(|_| Error::new(
                    ErrorCategory::Runtime,
                    codes::WASI_CAPABILITY_UNAVAILABLE,
                    kinds::WasiResourceError("Thread CPU time not available")
                ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_wasi_monotonic_clock_now() -> Result<()> {
        let result = wasi_monotonic_clock_now(&mut (), vec![])?;
        assert_eq!(result.len(), 1);
        
        // Should return a u64 timestamp
        if let Value::U64(timestamp) = &result[0] {
            // Timestamp should be non-zero (current time)
            assert!(*timestamp > 0);
        } else {
            panic!("Expected u64 timestamp");
        }
        
        Ok(())
    }
    
    #[test]
    fn test_wasi_wall_clock_now() -> Result<()> {
        let result = wasi_wall_clock_now(&mut (), vec![])?;
        assert_eq!(result.len(), 1);
        
        // Should return a tuple of (seconds, nanoseconds)
        if let Value::Tuple(time_parts) = &result[0] {
            assert_eq!(time_parts.len(), 2);
            
            // First should be seconds (u64)
            assert!(matches!(time_parts[0], Value::U64(_)));
            // Second should be nanoseconds (u32)
            assert!(matches!(time_parts[1], Value::U32(_)));
            
            // Verify nanoseconds are in valid range
            if let Value::U32(nanos) = &time_parts[1] {
                assert!(*nanos < 1_000_000_000);
            }
        } else {
            panic!("Expected tuple of (seconds, nanoseconds)");
        }
        
        Ok(())
    }
    
    #[test]
    fn test_nanoseconds_to_datetime_conversion() {
        let nanoseconds = 1_234_567_890_123_456_789u64;
        let datetime = nanoseconds_to_datetime(nanoseconds);
        
        if let Value::Record(fields) = datetime {
            assert_eq!(fields.len(), 2);
            
            let mut seconds_found = false;
            let mut nanos_found = false;
            
            for (key, value) in fields {
                match key.as_str() {
                    "seconds" => {
                        if let Value::U64(s) = value {
                            assert_eq!(s, 1_234_567_890);
                            seconds_found = true;
                        }
                    }
                    "nanoseconds" => {
                        if let Value::U32(ns) = value {
                            assert_eq!(ns, 123_456_789);
                            nanos_found = true;
                        }
                    }
                    _ => panic!("Unexpected field: {}", key),
                }
            }
            
            assert!(seconds_found && nanos_found);
        } else {
            panic!("Expected record");
        }
    }
    
    #[test]
    fn test_datetime_to_nanoseconds_conversion() -> Result<()> {
        let datetime = Value::Record(vec![
            ("seconds".to_string(), Value::U64(1_234_567_890)),
            ("nanoseconds".to_string(), Value::U32(123_456_789)),
        ]);
        
        let nanoseconds = datetime_to_nanoseconds(&datetime)?;
        assert_eq!(nanoseconds, 1_234_567_890_123_456_789);
        
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
        let result = get_time_with_capabilities(WasiClockType::ProcessCpuTime, &capabilities);
        assert!(result.is_err());
        
        Ok(())
    }
}