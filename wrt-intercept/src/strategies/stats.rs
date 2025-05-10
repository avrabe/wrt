//! Statistics strategy for intercepting component function calls
//!
//! This strategy collects metrics on function calls between components and hosts.
//! It can track call counts, error rates, performance metrics, etc.
//!
//! Note: This strategy requires the `std` feature.

use crate::prelude::*;

#[cfg(feature = "std")]
use std::time::{Duration, Instant};

/// Statistics collected for a function
#[cfg(feature = "std")]
#[derive(Debug, Clone, Default)]
pub struct FunctionStats {
    /// Number of times the function was called
    pub call_count: u64,
    /// Number of successful calls
    pub success_count: u64,
    /// Number of failed calls
    pub error_count: u64,
    /// Total execution time in milliseconds
    pub total_time_ms: f64,
    /// Minimum execution time in milliseconds
    pub min_time_ms: Option<f64>,
    /// Maximum execution time in milliseconds
    pub max_time_ms: Option<f64>,
    /// Average execution time in milliseconds
    pub avg_time_ms: f64,
}

/// Configuration for the statistics strategy
#[cfg(feature = "std")]
#[derive(Debug, Clone)]
pub struct StatisticsConfig {
    /// Whether to track timings
    pub track_timing: bool,
    /// Whether to track errors
    pub track_errors: bool,
    /// Maximum number of functions to track (0 for unlimited)
    pub max_functions: usize,
}

#[cfg(feature = "std")]
impl Default for StatisticsConfig {
    fn default() -> Self {
        Self { track_timing: true, track_errors: true, max_functions: 1000 }
    }
}

/// A strategy that collects statistics on function calls
#[cfg(feature = "std")]
pub struct StatisticsStrategy {
    /// Configuration for this strategy
    config: StatisticsConfig,
    /// Statistics for each function
    stats: std::sync::RwLock<HashMap<String, FunctionStats>>,
    /// Cache of currently executing functions with their start times
    executing: std::sync::Mutex<HashMap<String, std::time::Instant>>,
}

#[cfg(feature = "std")]
impl Default for StatisticsStrategy {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "std")]
impl StatisticsStrategy {
    /// Create a new statistics strategy with default configuration
    pub fn new() -> Self {
        Self {
            config: StatisticsConfig::default(),
            stats: std::sync::RwLock::new(HashMap::new()),
            executing: std::sync::Mutex::new(HashMap::new()),
        }
    }

    /// Create a new statistics strategy with custom configuration
    pub fn with_config(config: StatisticsConfig) -> Self {
        Self {
            config,
            stats: std::sync::RwLock::new(HashMap::new()),
            executing: std::sync::Mutex::new(HashMap::new()),
        }
    }

    /// Helper function to generate a unique key for a function call
    fn function_key(source: &str, target: &str, function: &str) -> String {
        format!("{}->{}::{}", source, target, function)
    }

    /// Get statistics for all functions
    pub fn get_all_stats(&self) -> HashMap<String, FunctionStats> {
        match self.stats.read() {
            Ok(stats) => stats.clone(),
            _ => HashMap::new(),
        }
    }

    /// Get statistics for a specific function
    pub fn get_function_stats(
        &self,
        source: &str,
        target: &str,
        function: &str,
    ) -> Option<FunctionStats> {
        let key = Self::function_key(source, target, function);
        match self.stats.read() {
            Ok(stats) => stats.get(&key).cloned(),
            _ => None,
        }
    }

    /// Reset all statistics
    pub fn reset(&self) {
        if let Ok(mut stats) = self.stats.write() {
            stats.clear();
        }
        if let Ok(mut executing) = self.executing.lock() {
            executing.clear();
        }
    }
}

#[cfg(feature = "std")]
impl LinkInterceptorStrategy for StatisticsStrategy {
    fn before_call(
        &self,
        source: &str,
        target: &str,
        function: &str,
        args: &[Value],
    ) -> Result<Vec<Value>> {
        if self.config.track_timing {
            let key = Self::function_key(source, target, function);
            if let Ok(mut executing) = self.executing.lock() {
                executing.insert(key, Instant::now());
            }
        }

        // Return unmodified arguments
        Ok(args.to_vec())
    }

    fn after_call(
        &self,
        source: &str,
        target: &str,
        function: &str,
        _args: &[Value],
        result: Result<Vec<Value>>,
    ) -> Result<Vec<Value>> {
        let key = Self::function_key(source, target, function);
        let is_success = result.is_ok();
        let elapsed_ms = if self.config.track_timing {
            match self.executing.lock() {
                Ok(mut executing) => {
                    executing.remove(&key).map(|start| start.elapsed().as_secs_f64() * 1000.0)
                }
                _ => None,
            }
        } else {
            None
        };

        // Update statistics
        if let Ok(mut stats_map) = self.stats.write() {
            // Check if we're at the limit and this is a new function
            if self.config.max_functions > 0
                && stats_map.len() >= self.config.max_functions
                && !stats_map.contains_key(&key)
            {
                // At limit, don't track a new function
                return result;
            }

            let stats = stats_map.entry(key).or_insert_with(FunctionStats::default);

            // Update call counts
            stats.call_count += 1;
            if is_success {
                stats.success_count += 1;
            } else if self.config.track_errors {
                stats.error_count += 1;
            }

            // Update timing information
            if let Some(elapsed) = elapsed_ms {
                stats.total_time_ms += elapsed;

                if let Some(min) = stats.min_time_ms {
                    if elapsed < min {
                        stats.min_time_ms = Some(elapsed);
                    }
                } else {
                    stats.min_time_ms = Some(elapsed);
                }

                if let Some(max) = stats.max_time_ms {
                    if elapsed > max {
                        stats.max_time_ms = Some(elapsed);
                    }
                } else {
                    stats.max_time_ms = Some(elapsed);
                }

                stats.avg_time_ms = stats.total_time_ms / stats.call_count as f64;
            }
        }

        // Return unmodified result
        result
    }

    fn clone_strategy(&self) -> Arc<dyn LinkInterceptorStrategy> {
        Arc::new(Self {
            config: self.config.clone(),
            stats: RwLock::new(HashMap::new()),
            executing: Mutex::new(HashMap::new()),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_statistics_strategy() {
        let strategy = StatisticsStrategy::new();

        // Test before_call and after_call
        let source = "source";
        let target = "target";
        let function = "test_function";
        let args = vec![Value::I32(42)];

        // First call (success)
        strategy.before_call(source, target, function, &args).unwrap();
        thread::sleep(Duration::from_millis(10)); // Simulate some work
        let result = Ok(vec![Value::I64(123)]);
        strategy.after_call(source, target, function, &args, result).unwrap();

        // Second call (error)
        strategy.before_call(source, target, function, &args).unwrap();
        thread::sleep(Duration::from_millis(5)); // Simulate some work
        let result = Err(wrt_error::Error::new(
            wrt_error::ErrorCategory::Runtime,
            wrt_error::codes::RUNTIME_ERROR,
            "Test error",
        ));
        let _ = strategy.after_call(source, target, function, &args, result);

        // Check statistics
        let key = StatisticsStrategy::function_key(source, target, function);
        let stats = strategy.get_all_stats();
        assert!(stats.contains_key(&key));

        let func_stats = stats.get(&key).unwrap();
        assert_eq!(func_stats.call_count, 2);
        assert_eq!(func_stats.success_count, 1);
        assert_eq!(func_stats.error_count, 1);
        assert!(func_stats.total_time_ms > 0.0);
        assert!(func_stats.min_time_ms.unwrap() > 0.0);
        assert!(func_stats.max_time_ms.unwrap() > 0.0);
        assert!(func_stats.avg_time_ms > 0.0);
    }

    #[test]
    fn test_statistics_config() {
        let config =
            StatisticsConfig { track_timing: false, track_errors: true, max_functions: 10 };
        let strategy = StatisticsStrategy::with_config(config);

        let source = "source";
        let target = "target";
        let function = "test_function";
        let args = vec![Value::I32(42)];

        // Make a call
        strategy.before_call(source, target, function, &args).unwrap();
        let result = Ok(vec![Value::I64(123)]);
        strategy.after_call(source, target, function, &args, result).unwrap();

        // Check statistics - timing should not be tracked
        let key = StatisticsStrategy::function_key(source, target, function);
        let stats = strategy.get_all_stats();
        let func_stats = stats.get(&key).unwrap();
        assert_eq!(func_stats.call_count, 1);
        assert_eq!(func_stats.success_count, 1);
        assert_eq!(func_stats.total_time_ms, 0.0);
        assert!(func_stats.min_time_ms.is_none());
        assert!(func_stats.max_time_ms.is_none());
        assert_eq!(func_stats.avg_time_ms, 0.0);
    }

    #[test]
    fn test_statistics_reset() {
        let strategy = StatisticsStrategy::new();

        // Make a call
        let source = "source";
        let target = "target";
        let function = "test_function";
        let args = vec![Value::I32(42)];

        strategy.before_call(source, target, function, &args).unwrap();
        let result = Ok(vec![Value::I64(123)]);
        strategy.after_call(source, target, function, &args, result).unwrap();

        // Verify we have stats
        assert_eq!(strategy.get_all_stats().len(), 1);

        // Reset and verify
        strategy.reset();
        assert_eq!(strategy.get_all_stats().len(), 0);
    }
}
