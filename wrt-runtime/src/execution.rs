//! Execution related structures and functions
//!
//! This module provides types and utilities for tracking execution statistics
//! and managing WebAssembly execution.

use crate::prelude::*;

/// Structure to track execution statistics
#[derive(Debug, Default, Clone)]
pub struct ExecutionStats {
    /// Number of instructions executed
    pub instructions_executed: u64,
    /// Memory usage in bytes
    pub memory_usage: usize,
    /// Maximum stack depth reached
    pub max_stack_depth: usize,
    /// Number of function calls
    pub function_calls: u64,
    /// Number of memory reads
    pub memory_reads: u64,
    /// Number of memory writes
    pub memory_writes: u64,
    /// Execution time in microseconds
    pub execution_time_us: u64,
    /// Gas used (if metering is enabled)
    pub gas_used: u64,
    /// Gas limit (if metering is enabled)
    pub gas_limit: u64,
}

impl ExecutionStats {
    /// Create a new instance with default values
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Reset all statistics to zero
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// Increment the instruction count
    pub fn increment_instructions(&mut self, count: u64) {
        self.instructions_executed = self.instructions_executed.saturating_add(count);
    }

    /// Update memory usage
    pub fn update_memory_usage(&mut self, bytes: usize) {
        self.memory_usage = self.memory_usage.saturating_add(bytes);
    }

    /// Update maximum stack depth
    pub fn update_stack_depth(&mut self, depth: usize) {
        self.max_stack_depth = self.max_stack_depth.max(depth);
    }

    /// Increment function call count
    pub fn increment_function_calls(&mut self, count: u64) {
        self.function_calls = self.function_calls.saturating_add(count);
    }

    /// Increment memory read count
    pub fn increment_memory_reads(&mut self, count: u64) {
        self.memory_reads = self.memory_reads.saturating_add(count);
    }

    /// Increment memory write count
    pub fn increment_memory_writes(&mut self, count: u64) {
        self.memory_writes = self.memory_writes.saturating_add(count);
    }

    /// Update execution time
    pub fn update_execution_time(&mut self, time_us: u64) {
        self.execution_time_us = self.execution_time_us.saturating_add(time_us);
    }

    /// Check if gas limit is exceeded
    pub fn is_gas_exceeded(&self) -> bool {
        self.gas_limit > 0 && self.gas_used >= self.gas_limit
    }

    /// Use gas and check if limit is exceeded
    pub fn use_gas(&mut self, amount: u64) -> Result<()> {
        self.gas_used = self.gas_used.saturating_add(amount);
        
        if self.is_gas_exceeded() {
            return Err(Error::new(
                ErrorCategory::Runtime,
                codes::GAS_LIMIT_EXCEEDED,
                format!("Gas limit of {} exceeded (used {})", self.gas_limit, self.gas_used),
            ));
        }
        
        Ok(())
    }
    
    /// Set gas limit
    pub fn set_gas_limit(&mut self, limit: u64) {
        self.gas_limit = limit;
    }
}

/// Execution context containing state for a running WebAssembly instance
#[derive(Debug, Clone)]
pub struct ExecutionContext {
    /// Execution statistics
    pub stats: ExecutionStats,
    /// Whether execution is currently trapped
    pub trapped: bool,
    /// Current function depth
    pub function_depth: usize,
    /// Maximum allowed function depth
    pub max_function_depth: usize,
}

impl ExecutionContext {
    /// Create a new execution context
    pub fn new(max_function_depth: usize) -> Self {
        Self {
            stats: ExecutionStats::default(),
            trapped: false,
            function_depth: 0,
            max_function_depth,
        }
    }

    /// Enter a function
    pub fn enter_function(&mut self) -> Result<()> {
        self.function_depth += 1;
        
        if self.function_depth > self.max_function_depth {
            self.trapped = true;
            return Err(Error::new(
                ErrorCategory::Runtime,
                codes::CALL_STACK_EXHAUSTED,
                format!("Call stack exhausted: depth {} exceeds maximum {}", 
                        self.function_depth, self.max_function_depth),
            ));
        }
        
        self.stats.increment_function_calls(1);
        self.stats.update_stack_depth(self.function_depth);
        
        Ok(())
    }

    /// Exit a function
    pub fn exit_function(&mut self) {
        if self.function_depth > 0 {
            self.function_depth -= 1;
        }
    }

    /// Check if execution is trapped
    pub fn is_trapped(&self) -> bool {
        self.trapped
    }

    /// Set trapped state
    pub fn set_trapped(&mut self, trapped: bool) {
        self.trapped = trapped;
    }
} 