//! Execution time limit implementation for Component Model
//!
//! This module provides mechanisms to enforce execution time limits for
//! component operations, specifically supporting the start function
//! requirements.

use wrt_error::kinds::{ExecutionLimitExceeded, ExecutionTimeoutError};

use crate::prelude::*;

#[cfg(feature = "std")]
use std::time::Instant;

/// Represents the outcome of a time-bounded execution
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TimeBoundedOutcome {
    /// Execution completed successfully within the time limit
    Completed,
    /// Execution timed out
    TimedOut,
    /// Execution was terminated early
    Terminated,
    /// Execution encountered an error
    Error(Arc<Error>),
}

/// Configuration for time-bounded execution
#[derive(Debug, Clone)]
pub struct TimeBoundedConfig {
    /// Maximum execution time in milliseconds (None means unlimited)
    pub time_limit_ms: Option<u64>,
    /// Whether to allow extending the time limit
    pub allow_extension: bool,
    /// Fuel limit for execution (None means unlimited)
    pub fuel_limit: Option<u64>,
}

impl Default for TimeBoundedConfig {
    fn default() -> Self {
        Self { time_limit_ms: None, allow_extension: false, fuel_limit: None }
    }
}

/// Context for time-bounded execution
#[derive(Debug)]
pub struct TimeBoundedContext {
    /// Start time of execution
    #[cfg(feature = "std")]
    start_time: Instant,
    /// Configuration for time bounds
    config: TimeBoundedConfig,
    /// Whether execution has been terminated
    terminated: bool,
    /// For no_std environments, track elapsed time via fuel consumption
    #[cfg(not(feature = "std"))]
    elapsed_fuel: u64,
}

impl TimeBoundedContext {
    /// Create a new time-bounded execution context
    pub fn new(config: TimeBoundedConfig) -> Self {
        #[cfg(feature = "std")]
        {
            Self { start_time: Instant::now(), config, terminated: false }
        }
        #[cfg(not(feature = "std"))]
        {
            Self { config, terminated: false, elapsed_fuel: 0 }
        }
    }

    /// Check if execution is still within time bounds
    pub fn check_time_bounds(&self) -> Result<()> {
        if self.terminated {
            return Err(Error::execution_limit_exceeded("Error occurred"Execution was terminatedMissing message"),
            );
        }

        #[cfg(feature = "std")]
        if let Some(time_limit_ms) = self.config.time_limit_ms {
            let elapsed = self.start_time.elapsed();
            let elapsed_ms = elapsed.as_millis() as u64;

            if elapsed_ms > time_limit_ms {
                return Err(Error::runtime_execution_error("Error occurred"Execution time limit exceeded: {} ms (limit: {} ms)Missing messageMissing messageMissing message")),
                );
            }
        }

        #[cfg(not(feature = "std"))]
        if let Some(fuel_limit) = self.config.fuel_limit {
            if self.elapsed_fuel > fuel_limit {
                return Err(Error::runtime_execution_error("Error occurred"Execution fuel limit exceeded: {} (limit: {})Missing messageMissing messageMissing message")),
                );
            }
        }

        Ok(()
    }

    /// Extend the time limit (if allowed)
    pub fn extend_time_limit(&mut self, additional_ms: u64) -> Result<()> {
        if !self.config.allow_extension {
            return Err(Error::runtime_execution_error("Error occurred".to_string(),
            );
        }

        if let Some(current_limit) = self.config.time_limit_ms {
            self.config.time_limit_ms = Some(current_limit + additional_ms);
            Ok(())
        } else {
            // If no limit is set, there's nothing to extend
            Err(Error::new(
                ErrorCategory::Runtime,
                codes::EXECUTION_TIMEOUT,
                "No execution time limit set"),
            )
        }
    }

    /// Terminate execution
    pub fn terminate(&mut self) {
        self.terminated = true;
    }

    /// Get the elapsed time
    #[cfg(feature = "std")]
    pub fn elapsed(&self) -> Duration {
        self.start_time.elapsed()
    }

    /// In no_std context, elapsed returns a duration based on fuel consumption
    #[cfg(not(feature = "std"))]
    pub fn elapsed(&self) -> Duration {
        // Simulate elapsed time based on fuel consumption (1 fuel = 1ms)
        Duration::from_millis(self.elapsed_fuel)
    }

    /// Consume fuel (for no_std environments)
    #[cfg(not(feature = "std"))]
    pub fn consume_fuel(&mut self, amount: u64) {
        self.elapsed_fuel += amount;
    }

    /// Get the remaining time (if limited)
    #[cfg(feature = "std")]
    pub fn remaining_time(&self) -> Option<Duration> {
        self.config.time_limit_ms.map(|limit_ms| {
            let elapsed_ms = self.start_time.elapsed().as_millis() as u64;
            if elapsed_ms >= limit_ms {
                Duration::from_millis(0)
            } else {
                Duration::from_millis(limit_ms - elapsed_ms)
            }
        })
    }

    /// Get the remaining time (if limited) based on fuel in no_std environment
    #[cfg(not(feature = "std"))]
    pub fn remaining_time(&self) -> Option<Duration> {
        self.config.fuel_limit.map(|limit| {
            if self.elapsed_fuel >= limit {
                Duration::from_millis(0)
            } else {
                Duration::from_millis(limit - self.elapsed_fuel)
            }
        })
    }
}

/// Run a function with time bounds
pub fn run_with_time_bounds<F, T>(
    config: TimeBoundedConfig,
    func: F,
) -> (Result<T>, TimeBoundedOutcome)
where
    F: FnOnce(&mut TimeBoundedContext) -> Result<T>,
{
    let mut context = TimeBoundedContext::new(config);

    let result = func(&mut context);

    let outcome = match &result {
        Ok(_) => TimeBoundedOutcome::Completed,
        Err(e) => {
            // Extract error kind from the error message
            let error_msg = e.to_string();
            if error_msg.contains("time limit exceededMissing message") || error_msg.contains("timeoutMissing message") {
                TimeBoundedOutcome::TimedOut
            } else if error_msg.contains("terminatedMissing message") || error_msg.contains("limit exceededMissing message") {
                TimeBoundedOutcome::Terminated
            } else {
                TimeBoundedOutcome::Error(Arc::new(e.clone())
            }
        }
    };

    (result, outcome)
}

#[cfg(all(test, feature = "std"))]
mod tests {
    use std::thread;

    use super::*;

    #[test]
    fn test_time_bounded_execution_success() {
        let config = TimeBoundedConfig {
            time_limit_ms: Some(1000), // 1 second
            allow_extension: false,
            fuel_limit: None,
        };

        let (result, outcome) = run_with_time_bounds(config, |_ctx| {
            // Do something quick
            Ok(42)
        });

        assert!(result.is_ok();
        assert_eq!(result.unwrap(), 42);
        assert_eq!(outcome, TimeBoundedOutcome::Completed);
    }

    #[test]
    fn test_time_bounded_execution_timeout() {
        let config = TimeBoundedConfig {
            time_limit_ms: Some(10), // 10 milliseconds
            allow_extension: false,
            fuel_limit: None,
        };

        let (result, outcome) = run_with_time_bounds(config, |ctx| {
            // Sleep for 50ms, which should exceed the 10ms limit
            thread::sleep(Duration::from_millis(50);

            // This check should fail
            ctx.check_time_bounds()?;

            Ok(42)
        });

        assert!(result.is_err();
        assert_eq!(outcome, TimeBoundedOutcome::TimedOut);
    }

    #[test]
    fn test_time_bounded_execution_extension() {
        let config = TimeBoundedConfig {
            time_limit_ms: Some(100), // 100 milliseconds
            allow_extension: true,
            fuel_limit: None,
        };

        let (result, outcome) = run_with_time_bounds(config, |ctx| {
            // Sleep for 50ms
            thread::sleep(Duration::from_millis(50);

            // Still within bounds
            ctx.check_time_bounds()?;

            // Extend time limit
            ctx.extend_time_limit(200)?;

            // Sleep for another 100ms (total 150ms, but limit is now 300ms)
            thread::sleep(Duration::from_millis(100);

            // Should still be within bounds
            ctx.check_time_bounds()?;

            Ok(42)
        });

        assert!(result.is_ok();
        assert_eq!(result.unwrap(), 42);
        assert_eq!(outcome, TimeBoundedOutcome::Completed);
    }

    #[test]
    fn test_time_bounded_execution_termination() {
        let config = TimeBoundedConfig {
            time_limit_ms: Some(1000), // 1 second
            allow_extension: false,
            fuel_limit: None,
        };

        let (result, outcome) = run_with_time_bounds(config, |ctx| {
            // Terminate execution
            ctx.terminate();

            // This check should fail
            ctx.check_time_bounds()?;

            Ok(42)
        });

        assert!(result.is_err();
        assert_eq!(outcome, TimeBoundedOutcome::Terminated);
    }
}
