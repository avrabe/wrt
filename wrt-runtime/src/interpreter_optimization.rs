//! Interpreter Optimization using Branch Hints
//!
//! This module implements performance optimizations for WebAssembly interpretation
//! based on branch prediction hints. These optimizations improve execution speed
//! even without JIT compilation by making the interpreter more efficient.

extern crate alloc;

use crate::prelude::{BoundedCapacity, Debug, Eq, PartialEq};
use crate::branch_prediction::{
    BranchLikelihood, ModuleBranchPredictor, PredictiveExecutionContext,
};
use crate::bounded_runtime_infra::{
    BoundedInterpreterOptMap, RuntimeProvider, new_interpreter_opt_map
};
use wrt_error::{Error, ErrorCategory, Result, codes};
use wrt_foundation::types::Instruction;

#[cfg(feature = "std")]
use std::vec::Vec;
#[cfg(not(feature = "std"))]
use alloc::vec::Vec;

/// Optimization strategy for interpreter execution
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[derive(Default)]
pub enum OptimizationStrategy {
    /// No optimization - standard interpretation
    None,
    /// Basic branch prediction only
    #[default]
    BranchPrediction,
    /// Branch prediction + instruction prefetching
    PredictionWithPrefetch,
    /// All optimizations enabled
    Aggressive,
}


/// Execution path optimization information
#[derive(Debug, Clone)]
pub struct ExecutionPath {
    /// Sequence of instruction offsets in execution order
    pub instruction_sequence: wrt_foundation::bounded::BoundedVec<u32, 256, wrt_foundation::safe_memory::NoStdProvider<1024>>,
    /// Predicted probability of this path being taken
    pub probability: f64,
    /// Whether this path should be optimized for speed
    pub is_hot_path: bool,
}

impl ExecutionPath {
    /// Create new execution path
    pub fn new(instruction_sequence: Vec<u32>, probability: f64) -> Result<Self> {
        let mut bounded_sequence = wrt_foundation::bounded::BoundedVec::new(
            wrt_foundation::safe_memory::NoStdProvider::<1024>::default()
        )?;
        
        for instruction in instruction_sequence {
            bounded_sequence.push(instruction).map_err(|_| Error::new(
                ErrorCategory::Memory,
                codes::MEMORY_ERROR,
                "Too many instructions in execution path"
            ))?;
        }
        
        Ok(Self {
            instruction_sequence: bounded_sequence,
            probability,
            is_hot_path: probability > 0.7, // Hot if > 70% likely
        })
    }
    
    /// Check if this path is likely to be executed
    pub fn is_likely(&self) -> bool {
        self.probability > 0.5
    }
    
    /// Get optimization priority (higher = more important to optimize)
    pub fn optimization_priority(&self) -> u32 {
        if self.is_hot_path {
            (self.probability * 100.0) as u32
        } else {
            0
        }
    }
}

/// Instruction prefetch cache for predicted execution paths
#[derive(Debug)]
pub struct InstructionPrefetchCache {
    /// Cached instructions for quick access using bounded collections
    cache: BoundedInterpreterOptMap<crate::prelude::Instruction>,
    /// Cache hit statistics
    pub cache_hits: u64,
    /// Cache miss statistics
    pub cache_misses: u64,
}

impl InstructionPrefetchCache {
    /// Create new prefetch cache
    pub fn new() -> Result<Self> {
        Ok(Self {
            cache: new_interpreter_opt_map()?,
            cache_hits: 0,
            cache_misses: 0,
        })
    }
    
    /// Prefetch instruction at offset
    pub fn prefetch(&mut self, offset: u32, instruction: crate::prelude::Instruction) -> Result<()> {
        self.cache.insert(offset, instruction)?;
        Ok(())
    }
    
    /// Get cached instruction if available
    pub fn get_cached(&mut self, offset: u32) -> Option<crate::prelude::Instruction> {
        if let Ok(Some(instruction)) = self.cache.get(&offset) {
            self.cache_hits += 1;
            Some(instruction.clone())
        } else {
            self.cache_misses += 1;
            None
        }
    }
    
    /// Get cache hit ratio
    pub fn hit_ratio(&self) -> f64 {
        let total = self.cache_hits + self.cache_misses;
        if total == 0 {
            0.0
        } else {
            self.cache_hits as f64 / total as f64
        }
    }
    
    /// Clear the cache
    pub fn clear(&mut self) {
        #[cfg(feature = "std")]
        {
            let _ = self.cache.clear();
        }
        #[cfg(not(feature = "std"))]
        {
            let _ = self.cache.clear();
        }
    }
}

impl Default for InstructionPrefetchCache {
    fn default() -> Self {
        Self::new().unwrap_or_else(|_| {
            // Fallback with basic structure
            Self {
                cache: BoundedInterpreterOptMap::default(),
                cache_hits: 0,
                cache_misses: 0,
            }
        })
    }
}

/// Optimized interpreter execution engine
#[derive(Debug)]
pub struct OptimizedInterpreter {
    /// Branch prediction system
    pub predictor: ModuleBranchPredictor,
    /// Optimization strategy to use
    pub strategy: OptimizationStrategy,
    /// Instruction prefetch cache
    pub prefetch_cache: InstructionPrefetchCache,
    /// Execution statistics
    pub execution_stats: InterpreterStats,
}

impl OptimizedInterpreter {
    /// Create new optimized interpreter
    pub fn new(predictor: ModuleBranchPredictor, strategy: OptimizationStrategy) -> Result<Self> {
        Ok(Self {
            predictor,
            strategy,
            prefetch_cache: InstructionPrefetchCache::new()?,
            execution_stats: InterpreterStats::new(),
        })
    }
    
    /// Prepare for function execution with optimization
    pub fn prepare_function_execution(&mut self, function_index: u32) -> Result<()> {
        self.execution_stats.function_calls += 1;
        
        // Analyze function for optimization opportunities
        let has_predictor = self.predictor.get_function_predictor(function_index).is_some();
        if has_predictor {
            match self.strategy {
                OptimizationStrategy::None => {
                    // No optimization
                }
                OptimizationStrategy::BranchPrediction => {
                    // Just record that we have predictions available
                    self.execution_stats.predicted_functions += 1;
                }
                OptimizationStrategy::PredictionWithPrefetch => {
                    // Get the predictor (now returns owned value)
                    if let Some(func_predictor) = self.predictor.get_function_predictor(function_index) {
                        self.prefetch_likely_paths(&func_predictor)?;
                    }
                }
                OptimizationStrategy::Aggressive => {
                    // Get the predictor (now returns owned value)
                    if let Some(func_predictor) = self.predictor.get_function_predictor(function_index) {
                        self.prefetch_likely_paths(&func_predictor)?;
                        self.optimize_execution_paths(&func_predictor)?;
                    }
                }
            }
        }
        
        Ok(())
    }
    
    /// Optimize execution for branch instruction
    pub fn optimize_branch_execution(
        &mut self,
        function_index: u32,
        instruction_offset: u32,
        actual_branch_taken: bool,
    ) -> BranchOptimizationResult {
        let mut result = BranchOptimizationResult::new();
        
        // Get branch prediction
        if let Some(prediction_taken) = self.predictor.is_branch_predicted_taken(function_index, instruction_offset) {
            result.had_prediction = true;
            result.predicted_taken = prediction_taken;
            result.actual_taken = actual_branch_taken;
            result.prediction_correct = prediction_taken == actual_branch_taken;
            
            // Update statistics
            if result.prediction_correct {
                self.execution_stats.correct_predictions += 1;
            } else {
                self.execution_stats.incorrect_predictions += 1;
                // Clear prefetch cache on misprediction
                if matches!(self.strategy, OptimizationStrategy::PredictionWithPrefetch | OptimizationStrategy::Aggressive) {
                    self.prefetch_cache.clear();
                    result.cache_cleared = true;
                }
            }
            
            // Get likelihood for optimization decisions
            let likelihood = self.predictor.get_branch_likelihood(function_index, instruction_offset);
            result.confidence = likelihood.probability();
            
            // Prefetch next instructions if prediction is strong
            if likelihood.is_strong_prediction() {
                if let Some(next_offset) = self.predictor.predict_next_instruction(function_index, instruction_offset) {
                    result.should_prefetch = true;
                    result.prefetch_target = Some(next_offset);
                }
            }
        }
        
        self.execution_stats.total_branches += 1;
        result
    }
    
    /// Check if instruction is available in prefetch cache
    pub fn get_prefetched_instruction(&mut self, offset: u32) -> Option<crate::prelude::Instruction> {
        if matches!(self.strategy, OptimizationStrategy::PredictionWithPrefetch | OptimizationStrategy::Aggressive) {
            self.prefetch_cache.get_cached(offset)
        } else {
            None
        }
    }
    
    /// Prefetch instruction for future execution
    pub fn prefetch_instruction(&mut self, offset: u32, instruction: crate::prelude::Instruction) -> Result<()> {
        if matches!(self.strategy, OptimizationStrategy::PredictionWithPrefetch | OptimizationStrategy::Aggressive) {
            self.prefetch_cache.prefetch(offset, instruction)?;
            self.execution_stats.instructions_prefetched += 1;
        }
        Ok(())
    }
    
    /// Get prediction accuracy percentage
    pub fn prediction_accuracy(&self) -> f64 {
        let total = self.execution_stats.correct_predictions + self.execution_stats.incorrect_predictions;
        if total == 0 {
            0.0
        } else {
            self.execution_stats.correct_predictions as f64 / total as f64
        }
    }
    
    /// Get optimization effectiveness metrics
    pub fn get_optimization_metrics(&self) -> OptimizationMetrics {
        OptimizationMetrics {
            prediction_accuracy: self.prediction_accuracy(),
            cache_hit_ratio: self.prefetch_cache.hit_ratio(),
            total_branches: self.execution_stats.total_branches,
            predicted_branches: self.execution_stats.correct_predictions + self.execution_stats.incorrect_predictions,
            functions_optimized: self.execution_stats.predicted_functions,
            instructions_prefetched: self.execution_stats.instructions_prefetched,
        }
    }
    
    // Private helper methods
    
    fn prefetch_likely_paths(&mut self, func_predictor: &crate::branch_prediction::FunctionBranchPredictor) -> Result<()> {
        // TODO: Implement intelligent prefetching based on likely execution paths
        // For now, this is a placeholder that would analyze the function's
        // predictions and prefetch instructions for highly likely branches
        self.execution_stats.prefetch_operations += 1;
        Ok(())
    }
    
    fn optimize_execution_paths(&mut self, func_predictor: &crate::branch_prediction::FunctionBranchPredictor) -> Result<()> {
        // TODO: Implement execution path optimization
        // This could reorder instruction processing, pre-compute likely values, etc.
        self.execution_stats.path_optimizations += 1;
        Ok(())
    }
}

/// Result of branch optimization
#[derive(Debug, Clone)]
pub struct BranchOptimizationResult {
    /// Whether a prediction was available
    pub had_prediction: bool,
    /// What the prediction was (if available)
    pub predicted_taken: bool,
    /// What actually happened
    pub actual_taken: bool,
    /// Whether the prediction was correct
    pub prediction_correct: bool,
    /// Confidence level of the prediction (0.0 to 1.0)
    pub confidence: f64,
    /// Whether prefetching should be done
    pub should_prefetch: bool,
    /// Target offset for prefetching
    pub prefetch_target: Option<u32>,
    /// Whether prefetch cache was cleared due to misprediction
    pub cache_cleared: bool,
}

impl BranchOptimizationResult {
    fn new() -> Self {
        Self {
            had_prediction: false,
            predicted_taken: false,
            actual_taken: false,
            prediction_correct: false,
            confidence: 0.0,
            should_prefetch: false,
            prefetch_target: None,
            cache_cleared: false,
        }
    }
}

/// Statistics for interpreter execution
#[derive(Debug, Clone)]
pub struct InterpreterStats {
    /// Number of function calls
    pub function_calls: u64,
    /// Number of functions with predictions
    pub predicted_functions: u64,
    /// Total branch instructions executed
    pub total_branches: u64,
    /// Correct branch predictions
    pub correct_predictions: u64,
    /// Incorrect branch predictions
    pub incorrect_predictions: u64,
    /// Instructions successfully prefetched
    pub instructions_prefetched: u64,
    /// Number of prefetch operations performed
    pub prefetch_operations: u64,
    /// Number of execution path optimizations
    pub path_optimizations: u64,
}

impl InterpreterStats {
    fn new() -> Self {
        Self {
            function_calls: 0,
            predicted_functions: 0,
            total_branches: 0,
            correct_predictions: 0,
            incorrect_predictions: 0,
            instructions_prefetched: 0,
            prefetch_operations: 0,
            path_optimizations: 0,
        }
    }
}

/// Optimization effectiveness metrics
#[derive(Debug, Clone)]
pub struct OptimizationMetrics {
    /// Branch prediction accuracy (0.0 to 1.0)
    pub prediction_accuracy: f64,
    /// Instruction cache hit ratio (0.0 to 1.0)
    pub cache_hit_ratio: f64,
    /// Total number of branch instructions
    pub total_branches: u64,
    /// Number of branches with predictions
    pub predicted_branches: u64,
    /// Number of functions that were optimized
    pub functions_optimized: u64,
    /// Total instructions prefetched
    pub instructions_prefetched: u64,
}

impl OptimizationMetrics {
    /// Calculate overall optimization effectiveness score
    #[must_use] pub fn effectiveness_score(&self) -> f64 {
        if self.total_branches == 0 {
            return 0.0;
        }
        
        let prediction_coverage = if self.total_branches > 0 {
            self.predicted_branches as f64 / self.total_branches as f64
        } else {
            0.0
        };
        
        // Weighted score combining accuracy, coverage, and cache performance
        (self.prediction_accuracy * 0.5) + (prediction_coverage * 0.3) + (self.cache_hit_ratio * 0.2)
    }
    
    /// Check if optimizations are providing significant benefit
    #[must_use] pub fn is_effective(&self) -> bool {
        self.effectiveness_score() > 0.6 && self.predicted_branches > 10
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::branch_prediction::{BranchPrediction, FunctionBranchPredictor};
    
    #[test]
    fn test_execution_path() {
        let path = ExecutionPath::new(vec![1, 5, 10, 15], 0.8);
        assert!(path.is_likely());
        assert!(path.is_hot_path);
        assert_eq!(path.optimization_priority(), 80);
    }
    
    #[test]
    fn test_optimization_strategy() {
        assert_eq!(OptimizationStrategy::default(), OptimizationStrategy::BranchPrediction);
    }
    
    #[cfg(feature = "std")]
    #[test]
    fn test_instruction_prefetch_cache() {
        use wrt_foundation::types::Instruction;
        
        let mut cache = InstructionPrefetchCache::new();
        let instr = Instruction::<wrt_foundation::safe_memory::NoStdProvider<1024>>::Nop;
        
        cache.prefetch(10, instr).unwrap();
        assert!(cache.get_cached(10).is_some());
        assert!(cache.get_cached(20).is_none());
        
        assert_eq!(cache.cache_hits, 1);
        assert_eq!(cache.cache_misses, 1);
        assert_eq!(cache.hit_ratio(), 0.5);
    }
    
    #[cfg(feature = "std")]
    #[test]
    fn test_optimized_interpreter() {
        let predictor = ModuleBranchPredictor::new();
        let mut interpreter = OptimizedInterpreter::new(predictor, OptimizationStrategy::BranchPrediction);
        
        interpreter.prepare_function_execution(0).unwrap();
        assert_eq!(interpreter.execution_stats.function_calls, 1);
        
        let result = interpreter.optimize_branch_execution(0, 10, true);
        assert!(!result.had_prediction); // No predictions set up
        assert_eq!(interpreter.execution_stats.total_branches, 1);
    }
    
    #[test]
    fn test_optimization_metrics() {
        let metrics = OptimizationMetrics {
            prediction_accuracy: 0.8,
            cache_hit_ratio: 0.7,
            total_branches: 100,
            predicted_branches: 80,
            functions_optimized: 10,
            instructions_prefetched: 50,
        };
        
        assert!(metrics.effectiveness_score() > 0.7);
        assert!(metrics.is_effective());
    }
}