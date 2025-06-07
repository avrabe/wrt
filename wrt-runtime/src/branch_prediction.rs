//! Branch Prediction System for WebAssembly Interpreter
//!
//! This module implements profile-guided optimization using branch hints from
//! WebAssembly custom sections to improve interpreter performance through
//! better branch prediction and execution path optimization.

use crate::prelude::*;
use wrt_error::{Error, ErrorCategory, Result, codes};
use wrt_foundation::traits::*;

#[cfg(feature = "alloc")]
use alloc::vec::Vec;
#[cfg(feature = "std")]
use std::vec::Vec;

/// Branch prediction hint indicating likelihood of branch being taken
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BranchLikelihood {
    /// Branch is very unlikely to be taken (< 10% probability)
    VeryUnlikely,
    /// Branch is unlikely to be taken (10-40% probability)
    Unlikely,
    /// Branch probability is unknown or balanced (40-60% probability)
    Unknown,
    /// Branch is likely to be taken (60-90% probability)
    Likely,
    /// Branch is very likely to be taken (> 90% probability)
    VeryLikely,
}

impl BranchLikelihood {
    /// Create likelihood from branch hint value (0=false, 1=true)
    pub fn from_hint_value(hint: u8) -> Self {
        match hint {
            0 => BranchLikelihood::Unlikely,  // likely_false
            1 => BranchLikelihood::Likely,    // likely_true
            _ => BranchLikelihood::Unknown,
        }
    }
    
    /// Get probability estimate as a value between 0.0 and 1.0
    pub fn probability(&self) -> f64 {
        match self {
            BranchLikelihood::VeryUnlikely => 0.05,
            BranchLikelihood::Unlikely => 0.25,
            BranchLikelihood::Unknown => 0.50,
            BranchLikelihood::Likely => 0.75,
            BranchLikelihood::VeryLikely => 0.95,
        }
    }
    
    /// Check if branch is predicted to be taken
    pub fn is_predicted_taken(&self) -> bool {
        self.probability() > 0.5
    }
    
    /// Check if this is a strong prediction (high confidence)
    pub fn is_strong_prediction(&self) -> bool {
        matches!(self, BranchLikelihood::VeryUnlikely | BranchLikelihood::VeryLikely)
    }
}

impl Default for BranchLikelihood {
    fn default() -> Self {
        BranchLikelihood::Unknown
    }
}

/// Branch prediction information for a specific instruction
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct BranchPrediction {
    /// Instruction offset within function
    pub instruction_offset: u32,
    /// Predicted likelihood of branch being taken
    pub likelihood: BranchLikelihood,
    /// Target instruction offset if branch is taken
    pub taken_target: Option<u32>,
    /// Fall-through instruction offset if branch is not taken
    pub fallthrough_target: Option<u32>,
}

impl BranchPrediction {
    /// Create new branch prediction
    pub fn new(
        instruction_offset: u32,
        likelihood: BranchLikelihood,
        taken_target: Option<u32>,
        fallthrough_target: Option<u32>,
    ) -> Self {
        Self {
            instruction_offset,
            likelihood,
            taken_target,
            fallthrough_target,
        }
    }
    
    /// Get the predicted next instruction offset
    pub fn predicted_target(&self) -> Option<u32> {
        if self.likelihood.is_predicted_taken() {
            self.taken_target
        } else {
            self.fallthrough_target
        }
    }
    
    /// Get the unlikely target (for prefetching)
    pub fn unlikely_target(&self) -> Option<u32> {
        if self.likelihood.is_predicted_taken() {
            self.fallthrough_target
        } else {
            self.taken_target
        }
    }
}

impl wrt_foundation::traits::Checksummable for BranchPrediction {
    fn update_checksum(&self, checksum: &mut wrt_foundation::verification::Checksum) {
        checksum.update_slice(&self.instruction_offset.to_le_bytes());
        checksum.update_slice(&[self.likelihood as u8]);
    }
}

impl wrt_foundation::traits::ToBytes for BranchPrediction {
    fn serialized_size(&self) -> usize {
        12 // instruction_offset(4) + likelihood(1) + taken_target(4) + fallthrough_target(4) - simplified
    }

    fn to_bytes_with_provider<'a, P: wrt_foundation::MemoryProvider>(
        &self,
        writer: &mut wrt_foundation::traits::WriteStream<'a>,
        _provider: &P,
    ) -> wrt_foundation::Result<()> {
        writer.write_bytes(&self.instruction_offset.to_le_bytes())?;
        writer.write_bytes(&[self.likelihood as u8])?;
        writer.write_bytes(&self.taken_target.unwrap_or(0).to_le_bytes())?;
        writer.write_bytes(&self.fallthrough_target.unwrap_or(0).to_le_bytes())
    }
}

impl wrt_foundation::traits::FromBytes for BranchPrediction {
    fn from_bytes_with_provider<'a, P: wrt_foundation::MemoryProvider>(
        reader: &mut wrt_foundation::traits::ReadStream<'a>,
        _provider: &P,
    ) -> wrt_foundation::Result<Self> {
        let mut bytes = [0u8; 4];
        reader.read_bytes(&mut bytes)?;
        let instruction_offset = u32::from_le_bytes(bytes);
        
        let mut likelihood_byte = [0u8; 1];
        reader.read_bytes(&mut likelihood_byte)?;
        let likelihood = match likelihood_byte[0] {
            0 => BranchLikelihood::VeryUnlikely,
            1 => BranchLikelihood::Unlikely,
            2 => BranchLikelihood::Unknown,
            3 => BranchLikelihood::Likely,
            _ => BranchLikelihood::VeryLikely,
        };
        
        reader.read_bytes(&mut bytes)?;
        let taken_target = Some(u32::from_le_bytes(bytes));
        
        reader.read_bytes(&mut bytes)?;
        let fallthrough_target = Some(u32::from_le_bytes(bytes));
        
        Ok(Self {
            instruction_offset,
            likelihood,
            taken_target,
            fallthrough_target,
        })
    }
}

/// Function-level branch prediction table
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FunctionBranchPredictor {
    /// Function index
    pub function_index: u32,
    /// Branch predictions indexed by instruction offset
    #[cfg(feature = "std")]
    predictions: std::collections::BTreeMap<u32, BranchPrediction>,
    #[cfg(all(feature = "alloc", not(feature = "std")))]
    predictions: alloc::collections::BTreeMap<u32, BranchPrediction>,
    #[cfg(not(any(feature = "std", feature = "alloc")))]
    predictions: wrt_foundation::BoundedVec<BranchPrediction, 256, wrt_foundation::NoStdProvider<1024>>,
}

impl FunctionBranchPredictor {
    /// Create new function branch predictor
    pub fn new(function_index: u32) -> Self {
        Self {
            function_index,
            #[cfg(feature = "std")]
            predictions: std::collections::BTreeMap::new(),
            #[cfg(all(feature = "alloc", not(feature = "std")))]
            predictions: alloc::collections::BTreeMap::new(),
            #[cfg(not(any(feature = "std", feature = "alloc")))]
            predictions: wrt_foundation::BoundedVec::new(wrt_foundation::NoStdProvider::<1024>::default()).unwrap(),
        }
    }
    
    /// Add branch prediction for an instruction
    pub fn add_prediction(&mut self, prediction: BranchPrediction) -> Result<()> {
        #[cfg(any(feature = "std", feature = "alloc"))]
        {
            self.predictions.insert(prediction.instruction_offset, prediction);
            Ok(())
        }
        #[cfg(not(any(feature = "std", feature = "alloc")))]
        {
            self.predictions.push(prediction).map_err(|_| {
                Error::new(ErrorCategory::Memory, codes::MEMORY_ERROR, "Too many branch predictions")
            })
        }
    }
    
    /// Get branch prediction for instruction offset
    pub fn get_prediction(&self, instruction_offset: u32) -> Option<&BranchPrediction> {
        #[cfg(any(feature = "std", feature = "alloc"))]
        {
            self.predictions.get(&instruction_offset)
        }
        #[cfg(not(any(feature = "std", feature = "alloc")))]
        {
            for prediction in self.predictions.iter() {
                if prediction.instruction_offset == instruction_offset {
                    return Some(prediction);
                }
            }
            None
        }
    }
    
    /// Get predicted next instruction for current offset
    pub fn predict_next(&self, current_offset: u32) -> Option<u32> {
        self.get_prediction(current_offset)
            .and_then(|pred| pred.predicted_target())
    }
    
    /// Check if a branch at the given offset is predicted to be taken
    pub fn is_branch_predicted_taken(&self, instruction_offset: u32) -> Option<bool> {
        self.get_prediction(instruction_offset)
            .map(|pred| pred.likelihood.is_predicted_taken())
    }
    
    /// Get branch likelihood for instruction
    pub fn get_branch_likelihood(&self, instruction_offset: u32) -> BranchLikelihood {
        self.get_prediction(instruction_offset)
            .map(|pred| pred.likelihood)
            .unwrap_or_default()
    }
    
    /// Get all strong predictions (high confidence) for optimization
    #[cfg(any(feature = "std", feature = "alloc"))]
    pub fn get_strong_predictions(&self) -> Vec<&BranchPrediction> {
        self.predictions.values()
            .filter(|pred| pred.likelihood.is_strong_prediction())
            .collect()
    }
    
    /// Count total number of predictions
    pub fn prediction_count(&self) -> usize {
        #[cfg(any(feature = "std", feature = "alloc"))]
        {
            self.predictions.len()
        }
        #[cfg(not(any(feature = "std", feature = "alloc")))]
        {
            self.predictions.len()
        }
    }
}

impl wrt_foundation::traits::Checksummable for FunctionBranchPredictor {
    fn update_checksum(&self, checksum: &mut wrt_foundation::verification::Checksum) {
        checksum.update_slice(&self.function_index.to_le_bytes());
    }
}

impl wrt_foundation::traits::ToBytes for FunctionBranchPredictor {
    fn serialized_size(&self) -> usize {
        8 // Just function_index for simplicity
    }

    fn to_bytes_with_provider<'a, P: wrt_foundation::MemoryProvider>(
        &self,
        writer: &mut wrt_foundation::traits::WriteStream<'a>,
        _provider: &P,
    ) -> wrt_foundation::Result<()> {
        writer.write_bytes(&self.function_index.to_le_bytes())
    }
}

impl wrt_foundation::traits::FromBytes for FunctionBranchPredictor {
    fn from_bytes_with_provider<'a, P: wrt_foundation::MemoryProvider>(
        reader: &mut wrt_foundation::traits::ReadStream<'a>,
        _provider: &P,
    ) -> wrt_foundation::Result<Self> {
        let mut bytes = [0u8; 4];
        reader.read_bytes(&mut bytes)?;
        let function_index = u32::from_le_bytes(bytes);
        Ok(Self {
            function_index,
            ..Default::default()
        })
    }
}

/// Module-level branch prediction system
#[derive(Debug, Clone)]
pub struct ModuleBranchPredictor {
    /// Function predictors indexed by function index
    #[cfg(feature = "std")]
    function_predictors: std::collections::BTreeMap<u32, FunctionBranchPredictor>,
    #[cfg(all(feature = "alloc", not(feature = "std")))]
    function_predictors: alloc::collections::BTreeMap<u32, FunctionBranchPredictor>,
    #[cfg(not(any(feature = "std", feature = "alloc")))]
    function_predictors: wrt_foundation::BoundedVec<FunctionBranchPredictor, 1024, wrt_foundation::NoStdProvider<1024>>,
}

impl ModuleBranchPredictor {
    /// Create new module branch predictor
    pub fn new() -> Self {
        Self {
            #[cfg(feature = "std")]
            function_predictors: std::collections::BTreeMap::new(),
            #[cfg(all(feature = "alloc", not(feature = "std")))]
            function_predictors: alloc::collections::BTreeMap::new(),
            #[cfg(not(any(feature = "std", feature = "alloc")))]
            function_predictors: wrt_foundation::BoundedVec::new(wrt_foundation::NoStdProvider::<1024>::default()).unwrap(),
        }
    }
    
    /// Add function branch predictor
    pub fn add_function_predictor(&mut self, predictor: FunctionBranchPredictor) -> Result<()> {
        #[cfg(any(feature = "std", feature = "alloc"))]
        {
            self.function_predictors.insert(predictor.function_index, predictor);
            Ok(())
        }
        #[cfg(not(any(feature = "std", feature = "alloc")))]
        {
            self.function_predictors.push(predictor).map_err(|_| {
                Error::new(ErrorCategory::Memory, codes::MEMORY_ERROR, "Too many function predictors")
            })
        }
    }
    
    /// Get function branch predictor
    pub fn get_function_predictor(&self, function_index: u32) -> Option<&FunctionBranchPredictor> {
        #[cfg(any(feature = "std", feature = "alloc"))]
        {
            self.function_predictors.get(&function_index)
        }
        #[cfg(not(any(feature = "std", feature = "alloc")))]
        {
            for predictor in self.function_predictors.iter() {
                if predictor.function_index == function_index {
                    return Some(predictor);
                }
            }
            None
        }
    }
    
    /// Get mutable function branch predictor
    pub fn get_function_predictor_mut(&mut self, function_index: u32) -> Option<&mut FunctionBranchPredictor> {
        #[cfg(any(feature = "std", feature = "alloc"))]
        {
            self.function_predictors.get_mut(&function_index)
        }
        #[cfg(not(any(feature = "std", feature = "alloc")))]
        {
            for predictor in self.function_predictors.iter_mut() {
                if predictor.function_index == function_index {
                    return Some(predictor);
                }
            }
            None
        }
    }
    
    /// Predict next instruction for current execution context
    pub fn predict_next_instruction(
        &self,
        function_index: u32,
        instruction_offset: u32,
    ) -> Option<u32> {
        self.get_function_predictor(function_index)
            .and_then(|predictor| predictor.predict_next(instruction_offset))
    }
    
    /// Check if branch is predicted to be taken
    pub fn is_branch_predicted_taken(
        &self,
        function_index: u32,
        instruction_offset: u32,
    ) -> Option<bool> {
        self.get_function_predictor(function_index)
            .and_then(|predictor| predictor.is_branch_predicted_taken(instruction_offset))
    }
    
    /// Get branch likelihood for specific location
    pub fn get_branch_likelihood(
        &self,
        function_index: u32,
        instruction_offset: u32,
    ) -> BranchLikelihood {
        self.get_function_predictor(function_index)
            .map(|predictor| predictor.get_branch_likelihood(instruction_offset))
            .unwrap_or_default()
    }
    
    /// Create predictor from WebAssembly branch hint custom section
    #[cfg(all(feature = "alloc", feature = "decoder"))]
    pub fn from_branch_hints(
        branch_hints: &wrt_decoder::branch_hint_section::BranchHintSection,
        code_section: &[u8], // For analyzing branch targets
    ) -> Result<Self> {
        let mut predictor = Self::new();
        
        // Process each function's hints
        for func_idx in 0..branch_hints.function_count() {
            if let Some(hints) = branch_hints.get_function_hints(func_idx as u32) {
                let mut func_predictor = FunctionBranchPredictor::new(func_idx as u32);
                
                // Convert hints to predictions
                for (offset, hint) in hints.iter() {
                    let likelihood = BranchLikelihood::from_hint_value(hint.to_byte());
                    
                    // TODO: Analyze code section to determine branch targets
                    // For now, create prediction without specific targets
                    let prediction = BranchPrediction::new(
                        *offset,
                        likelihood,
                        None, // taken_target - would need code analysis
                        None, // fallthrough_target - would need code analysis
                    );
                    
                    func_predictor.add_prediction(prediction)?;
                }
                
                predictor.add_function_predictor(func_predictor)?;
            }
        }
        
        Ok(predictor)
    }
    
    /// Get total number of functions with predictions
    pub fn function_count(&self) -> usize {
        #[cfg(any(feature = "std", feature = "alloc"))]
        {
            self.function_predictors.len()
        }
        #[cfg(not(any(feature = "std", feature = "alloc")))]
        {
            self.function_predictors.len()
        }
    }
    
    /// Get total number of predictions across all functions
    pub fn total_prediction_count(&self) -> usize {
        #[cfg(any(feature = "std", feature = "alloc"))]
        {
            self.function_predictors.values()
                .map(|pred| pred.prediction_count())
                .sum()
        }
        #[cfg(not(any(feature = "std", feature = "alloc")))]
        {
            self.function_predictors.iter()
                .map(|pred| pred.prediction_count())
                .sum()
        }
    }
}

impl Default for ModuleBranchPredictor {
    fn default() -> Self {
        Self::new()
    }
}

/// Execution context with branch prediction support
#[derive(Debug)]
pub struct PredictiveExecutionContext {
    /// Current function index
    pub current_function: u32,
    /// Current instruction offset within function
    pub current_offset: u32,
    /// Branch predictor for the module
    pub predictor: ModuleBranchPredictor,
    /// Prediction accuracy statistics
    pub prediction_stats: PredictionStats,
}

impl PredictiveExecutionContext {
    /// Create new predictive execution context
    pub fn new(predictor: ModuleBranchPredictor) -> Self {
        Self {
            current_function: 0,
            current_offset: 0,
            predictor,
            prediction_stats: PredictionStats::new(),
        }
    }
    
    /// Update current execution position
    pub fn update_position(&mut self, function_index: u32, instruction_offset: u32) {
        self.current_function = function_index;
        self.current_offset = instruction_offset;
    }
    
    /// Get prediction for current position
    pub fn get_current_prediction(&self) -> Option<&BranchPrediction> {
        self.predictor
            .get_function_predictor(self.current_function)
            .and_then(|pred| pred.get_prediction(self.current_offset))
    }
    
    /// Predict next instruction offset
    pub fn predict_next(&self) -> Option<u32> {
        self.predictor.predict_next_instruction(self.current_function, self.current_offset)
    }
    
    /// Check if current position has a predicted branch
    pub fn has_branch_prediction(&self) -> bool {
        self.get_current_prediction().is_some()
    }
    
    /// Report branch prediction result for statistics
    pub fn report_branch_result(&mut self, was_taken: bool) {
        if let Some(prediction) = self.get_current_prediction() {
            let predicted_taken = prediction.likelihood.is_predicted_taken();
            if predicted_taken == was_taken {
                self.prediction_stats.record_correct_prediction();
            } else {
                self.prediction_stats.record_incorrect_prediction();
            }
        }
    }
}

/// Statistics for branch prediction accuracy
#[derive(Debug, Clone)]
pub struct PredictionStats {
    /// Number of correct predictions
    pub correct_predictions: u64,
    /// Number of incorrect predictions
    pub incorrect_predictions: u64,
    /// Number of total branch instructions encountered
    pub total_branches: u64,
}

impl PredictionStats {
    /// Create new prediction statistics
    pub fn new() -> Self {
        Self {
            correct_predictions: 0,
            incorrect_predictions: 0,
            total_branches: 0,
        }
    }
    
    /// Record a correct prediction
    pub fn record_correct_prediction(&mut self) {
        self.correct_predictions += 1;
        self.total_branches += 1;
    }
    
    /// Record an incorrect prediction
    pub fn record_incorrect_prediction(&mut self) {
        self.incorrect_predictions += 1;
        self.total_branches += 1;
    }
    
    /// Get prediction accuracy as percentage (0.0 to 1.0)
    pub fn accuracy(&self) -> f64 {
        if self.total_branches == 0 {
            0.0
        } else {
            self.correct_predictions as f64 / self.total_branches as f64
        }
    }
    
    /// Get total number of predictions made
    pub fn total_predictions(&self) -> u64 {
        self.correct_predictions + self.incorrect_predictions
    }
    
    /// Check if we have enough data for reliable statistics
    pub fn has_sufficient_data(&self) -> bool {
        self.total_predictions() >= 100
    }
}

impl Default for PredictionStats {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_branch_likelihood() {
        assert_eq!(BranchLikelihood::from_hint_value(0), BranchLikelihood::Unlikely);
        assert_eq!(BranchLikelihood::from_hint_value(1), BranchLikelihood::Likely);
        assert_eq!(BranchLikelihood::from_hint_value(2), BranchLikelihood::Unknown);
        
        assert!(!BranchLikelihood::Unlikely.is_predicted_taken());
        assert!(BranchLikelihood::Likely.is_predicted_taken());
        assert!(!BranchLikelihood::Unknown.is_predicted_taken());
        
        assert!(BranchLikelihood::VeryLikely.is_strong_prediction());
        assert!(!BranchLikelihood::Likely.is_strong_prediction());
    }
    
    #[test]
    fn test_branch_prediction() {
        let prediction = BranchPrediction::new(
            10,
            BranchLikelihood::Likely,
            Some(25),
            Some(11),
        );
        
        assert_eq!(prediction.predicted_target(), Some(25));
        assert_eq!(prediction.unlikely_target(), Some(11));
    }
    
    #[cfg(feature = "alloc")]
    #[test]
    fn test_function_branch_predictor() {
        let mut predictor = FunctionBranchPredictor::new(0);
        
        let prediction = BranchPrediction::new(
            10,
            BranchLikelihood::Likely,
            Some(25),
            Some(11),
        );
        
        predictor.add_prediction(prediction).unwrap();
        
        assert_eq!(predictor.predict_next(10), Some(25));
        assert_eq!(predictor.is_branch_predicted_taken(10), Some(true));
        assert_eq!(predictor.prediction_count(), 1);
    }
    
    #[cfg(feature = "alloc")]
    #[test]
    fn test_module_branch_predictor() {
        let mut module_predictor = ModuleBranchPredictor::new();
        let mut func_predictor = FunctionBranchPredictor::new(0);
        
        let prediction = BranchPrediction::new(
            10,
            BranchLikelihood::Likely,
            Some(25),
            Some(11),
        );
        
        func_predictor.add_prediction(prediction).unwrap();
        module_predictor.add_function_predictor(func_predictor).unwrap();
        
        assert_eq!(module_predictor.predict_next_instruction(0, 10), Some(25));
        assert_eq!(module_predictor.is_branch_predicted_taken(0, 10), Some(true));
        assert_eq!(module_predictor.function_count(), 1);
        assert_eq!(module_predictor.total_prediction_count(), 1);
    }
    
    #[test]
    fn test_prediction_stats() {
        let mut stats = PredictionStats::new();
        
        stats.record_correct_prediction();
        stats.record_correct_prediction();
        stats.record_incorrect_prediction();
        
        assert_eq!(stats.accuracy(), 2.0 / 3.0);
        assert_eq!(stats.total_predictions(), 3);
        assert!(!stats.has_sufficient_data());
    }
}