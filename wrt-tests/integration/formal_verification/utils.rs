//! Formal Verification Utilities
//!
//! This module provides common utilities, generators, and helpers for KANI
//! formal verification. These utilities ensure consistent verification patterns
//! across all proof modules and provide safety-checked value generation.
//!
//! # KANI Integration
//!
//! The utilities in this module are designed to work both with and without KANI:
//! - When KANI is available, they use `kani::any()` for symbolic execution
//! - When KANI is not available, they provide deterministic test values
//!
//! # Safety Properties
//!
//! All generators in this module ensure:
//! - Generated values respect domain constraints
//! - No undefined behavior in value generation
//! - Consistent behavior across verification runs

#![cfg_attr(not(feature = "std"), no_std)]
#![deny(clippy::all)]
#![warn(missing_docs)]
#![forbid(unsafe_code)]

extern crate alloc;

use wrt_foundation::safety_system::{AsilLevel, SafetyStandard, SafetyStandardType};
use wrt_foundation::safety_system::{DalLevel, SilLevel, MedicalClass, RailwaySil, AgricultureLevel};
use wrt_foundation::budget_aware_provider::CrateId;

/// Maximum memory size for bounded verification
pub const MAX_VERIFICATION_MEMORY: usize = 65536; // 64KB

/// Maximum allocation count for bounded verification
pub const MAX_VERIFICATION_ALLOCATIONS: usize = 16;

/// Maximum loop iterations for bounded verification
pub const MAX_VERIFICATION_ITERATIONS: usize = 10;

/// Generate valid ASIL level for verification
///
/// # Returns
///
/// A valid ASIL level that can be used in formal verification.
/// When KANI is available, generates symbolic values; otherwise
/// returns deterministic test values.
#[cfg(any(doc, kani, feature = "kani"))]
pub fn any_asil_level() -> AsilLevel {
    #[cfg(kani)]
    {
        let level: u8 = kani::any();
        kani::assume(level <= 4); // Valid ASIL levels: 0-4
        unsafe { core::mem::transmute(level) }
    }
    
    #[cfg(not(kani))]
    {
        // Provide deterministic test value when not in KANI mode
        AsilLevel::AsilC // Default to ASIL-C for traditional testing
    }
}

/// Generate bounded memory size for verification
///
/// # Arguments
///
/// * `max` - Maximum allowed size
///
/// # Returns
///
/// A memory size between 1 and `max`, suitable for bounded verification.
#[cfg(any(doc, kani, feature = "kani"))]
pub fn any_memory_size(max: usize) -> usize {
    #[cfg(kani)]
    {
        let size: usize = kani::any();
        kani::assume(size > 0 && size <= max);
        size
    }
    
    #[cfg(not(kani))]
    {
        // Use smaller deterministic values for traditional testing
        core::cmp::min(max, 1024)
    }
}

/// Generate valid memory alignment
///
/// # Returns
///
/// A power-of-2 alignment value (1, 2, 4, or 8) suitable for memory operations.
#[cfg(any(doc, kani, feature = "kani"))]
pub fn any_alignment() -> usize {
    #[cfg(kani)]
    {
        let align_power: u8 = kani::any();
        kani::assume(align_power <= 3); // 2^0, 2^1, 2^2, 2^3 = 1, 2, 4, 8
        1 << align_power
    }
    
    #[cfg(not(kani))]
    {
        4 // Default to 4-byte alignment for traditional testing
    }
}

/// Generate valid CrateId for verification
///
/// # Returns
///
/// A valid CrateId that can be used for budget allocation testing.
#[cfg(any(doc, kani, feature = "kani"))]
pub fn any_crate_id() -> CrateId {
    #[cfg(kani)]
    {
        let crate_id: u8 = kani::any();
        match crate_id % 8 {
            0 => CrateId::Foundation,
            1 => CrateId::Component,
            2 => CrateId::Runtime,
            3 => CrateId::Host,
            4 => CrateId::Sync,
            5 => CrateId::Platform,
            6 => CrateId::Instructions,
            _ => CrateId::Debug,
        }
    }
    
    #[cfg(not(kani))]
    {
        CrateId::Foundation // Default for traditional testing
    }
}

/// Generate valid SafetyStandard for cross-standard verification
///
/// # Returns
///
/// A SafetyStandard with valid parameters for verification.
#[cfg(any(doc, kani, feature = "kani"))]
pub fn any_safety_standard() -> SafetyStandard {
    #[cfg(kani)]
    {
        let standard_type: u8 = kani::any();
        match standard_type % 6 {
            0 => SafetyStandard::Iso26262(any_asil_level()),
            1 => SafetyStandard::Do178c(any_dal_level()),
            2 => SafetyStandard::Iec61508(any_sil_level()),
            3 => SafetyStandard::Iec62304(any_medical_class()),
            4 => SafetyStandard::En50128(any_railway_sil()),
            _ => SafetyStandard::Iso25119(any_agriculture_level()),
        }
    }
    
    #[cfg(not(kani))]
    {
        SafetyStandard::Iso26262(AsilLevel::AsilC) // Default for traditional testing
    }
}

/// Generate valid SafetyStandardType for conversion verification
///
/// # Returns
///
/// A SafetyStandardType for testing cross-standard conversions.
#[cfg(any(doc, kani, feature = "kani"))]
pub fn any_standard_type() -> SafetyStandardType {
    #[cfg(kani)]
    {
        let type_id: u8 = kani::any();
        match type_id % 6 {
            0 => SafetyStandardType::Iso26262,
            1 => SafetyStandardType::Do178c,
            2 => SafetyStandardType::Iec61508,
            3 => SafetyStandardType::Iec62304,
            4 => SafetyStandardType::En50128,
            _ => SafetyStandardType::Iso25119,
        }
    }
    
    #[cfg(not(kani))]
    {
        SafetyStandardType::Iso26262 // Default for traditional testing
    }
}

// Helper functions for generating other safety standard levels

/// Generate valid DAL level for DO-178C verification
#[cfg(any(doc, kani, feature = "kani"))]
pub fn any_dal_level() -> DalLevel {
    #[cfg(kani)]
    {
        let level: u8 = kani::any();
        kani::assume(level <= 4);
        unsafe { core::mem::transmute(level) }
    }
    
    #[cfg(not(kani))]
    {
        DalLevel::DalC // Default for traditional testing
    }
}

/// Generate valid SIL level for IEC 61508 verification
#[cfg(any(doc, kani, feature = "kani"))]
pub fn any_sil_level() -> SilLevel {
    #[cfg(kani)]
    {
        let level: u8 = kani::any();
        kani::assume(level >= 1 && level <= 4); // SIL 1-4
        unsafe { core::mem::transmute(level) }
    }
    
    #[cfg(not(kani))]
    {
        SilLevel::Sil2 // Default for traditional testing
    }
}

/// Generate valid Medical class for IEC 62304 verification
#[cfg(any(doc, kani, feature = "kani"))]
pub fn any_medical_class() -> MedicalClass {
    #[cfg(kani)]
    {
        let class: u8 = kani::any();
        kani::assume(class >= 1 && class <= 3); // Class A, B, C
        unsafe { core::mem::transmute(class) }
    }
    
    #[cfg(not(kani))]
    {
        MedicalClass::ClassB // Default for traditional testing
    }
}

/// Generate valid Railway SIL for EN 50128 verification
#[cfg(any(doc, kani, feature = "kani"))]
pub fn any_railway_sil() -> RailwaySil {
    #[cfg(kani)]
    {
        let level: u8 = kani::any();
        kani::assume(level <= 4); // SIL 0-4
        unsafe { core::mem::transmute(level) }
    }
    
    #[cfg(not(kani))]
    {
        RailwaySil::Sil2 // Default for traditional testing
    }
}

/// Generate valid Agriculture level for ISO 25119 verification
#[cfg(any(doc, kani, feature = "kani"))]
pub fn any_agriculture_level() -> AgricultureLevel {
    #[cfg(kani)]
    {
        let level: u8 = kani::any();
        kani::assume(level >= 1 && level <= 5); // AgPL a-e
        unsafe { core::mem::transmute(level) }
    }
    
    #[cfg(not(kani))]
    {
        AgricultureLevel::AgPlc // Default for traditional testing
    }
}

/// Check if a safety standard type requires safety classification
///
/// # Arguments
///
/// * `target` - The safety standard type to check
///
/// # Returns
///
/// `true` if the standard requires some level of safety classification
/// (i.e., cannot accept QM/no-safety levels from other standards).
#[cfg(any(doc, kani, feature = "kani"))]
pub fn target_requires_safety_classification(target: SafetyStandardType) -> bool {
    matches!(target, 
        SafetyStandardType::Iec61508 |   // Industrial safety always required
        SafetyStandardType::Iec62304 |   // Medical devices inherently affect safety
        SafetyStandardType::Iso25119     // Agricultural machinery can cause harm
    )
}

/// Verify that a value is within expected bounds
///
/// # Arguments
///
/// * `value` - The value to check
/// * `min` - Minimum allowed value (inclusive)
/// * `max` - Maximum allowed value (inclusive)
///
/// # Returns
///
/// `true` if the value is within bounds, `false` otherwise.
pub fn is_within_bounds(value: usize, min: usize, max: usize) -> bool {
    value >= min && value <= max
}

/// Calculate the next power of 2 for alignment purposes
///
/// # Arguments
///
/// * `value` - Input value
///
/// # Returns
///
/// The next power of 2 greater than or equal to `value`.
pub fn next_power_of_2(value: usize) -> usize {
    if value == 0 {
        return 1;
    }
    
    let mut power = 1;
    while power < value {
        power = power.saturating_mul(2);
        if power == 0 {
            // Overflow - return maximum power of 2 for usize
            return 1 << (core::mem::size_of::<usize>() * 8 - 1);
        }
    }
    power
}

/// Verification invariant helper for common assertions
///
/// This macro provides a consistent way to express verification invariants
/// that work both in KANI mode and traditional testing.
#[macro_export]
macro_rules! verify_invariant {
    ($condition:expr, $message:expr) => {
        #[cfg(kani)]
        {
            kani::assert!($condition, $message);
        }
        
        #[cfg(not(kani))]
        {
            assert!($condition, $message);
        }
    };
}

/// KANI assumption helper for preconditions
///
/// This macro provides a consistent way to express preconditions
/// that work both in KANI mode and traditional testing.
#[macro_export]
macro_rules! assume_precondition {
    ($condition:expr) => {
        #[cfg(kani)]
        {
            kani::assume($condition);
        }
        
        #[cfg(not(kani))]
        {
            // In traditional testing, convert assumptions to assertions
            // This helps catch invalid test setup
            assert!($condition, "Precondition violation in traditional test");
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_memory_size_generation() {
        let size = any_memory_size(1024);
        assert!(size > 0);
        assert!(size <= 1024);
    }
    
    #[test]
    fn test_alignment_generation() {
        let alignment = any_alignment();
        assert!(alignment.is_power_of_two());
        assert!(alignment <= 8);
    }
    
    #[test]
    fn test_bounds_checking() {
        assert!(is_within_bounds(5, 1, 10));
        assert!(!is_within_bounds(15, 1, 10));
        assert!(!is_within_bounds(0, 1, 10));
    }
    
    #[test]
    fn test_next_power_of_2() {
        assert_eq!(next_power_of_2(0), 1);
        assert_eq!(next_power_of_2(1), 1);
        assert_eq!(next_power_of_2(3), 4);
        assert_eq!(next_power_of_2(8), 8);
        assert_eq!(next_power_of_2(9), 16);
    }
    
    #[test]
    fn test_safety_classification_requirements() {
        assert!(target_requires_safety_classification(SafetyStandardType::Iec61508));
        assert!(target_requires_safety_classification(SafetyStandardType::Iec62304));
        assert!(target_requires_safety_classification(SafetyStandardType::Iso25119));
        assert!(!target_requires_safety_classification(SafetyStandardType::Iso26262));
        assert!(!target_requires_safety_classification(SafetyStandardType::Do178c));
        assert!(!target_requires_safety_classification(SafetyStandardType::En50128));
    }
}