//! Safety Invariants Formal Verification
//!
//! This module provides comprehensive formal verification of safety invariants
//! and ASIL-level properties in the WRT safety system.
//!
//! # Verified Properties
//!
//! - ASIL level monotonicity (levels can only increase in safety criticality)
//! - Violation count monotonicity (counts only increase, never decrease)
//! - Cross-standard safety conversion preservation (IEC 61508 <-> ISO 26262)
//! - Safety context state consistency across all operations
//!
//! # Implementation Status
//!
//! This module implements KANI Phase 3 safety invariants verification with
//! comprehensive formal verification of safety-critical properties.

#![cfg(any(doc, kani, feature = "kani"))]
#![deny(clippy::all)]
#![warn(missing_docs)]
#![forbid(unsafe_code)]

use wrt_test_registry::prelude::*;

#[cfg(kani)]
use kani;

#[cfg(feature = "kani")]
use wrt_foundation::{
    safety_system::{AsilLevel, SafetySystem},
    iec61508::{SilLevel, Iec61508System},
};

use crate::utils::{any_asil_level, any_sil_level};

/// Verify ASIL level monotonicity property
///
/// This harness verifies that ASIL levels can only increase in safety
/// criticality, never decrease. This is a fundamental safety property.
///
/// # Verified Properties
///
/// - Safety level upgrades are always valid (QM -> A -> B -> C -> D)
/// - Safety level downgrades are never allowed
/// - Safety level comparisons are consistent with criticality ordering
#[cfg(kani)]
pub fn verify_asil_monotonicity() {
    // Generate two arbitrary ASIL levels
    let level1 = any_asil_level);
    let level2 = any_asil_level);
    
    // Test the ordering relationship
    if level1.safety_criticality() <= level2.safety_criticality() {
        // If level1 has lower or equal criticality, upgrading should be allowed
        assert!(level1.can_upgrade_to(level2);
        
        // The reverse should not be allowed unless they're equal
        if level1.safety_criticality() < level2.safety_criticality() {
            assert!(!level2.can_upgrade_to(level1);
        }
    }
    
    // Verify memory protection requirements are monotonic
    if level1.requires_memory_protection() && !level2.requires_memory_protection() {
        // If level1 requires memory protection, level2 should have higher criticality
        assert!(level1.safety_criticality() > level2.safety_criticality();
    }
    
    // Verify ASIL-D is the highest level
    if level1 == AsilLevel::AsilD {
        assert!(level1.safety_criticality() >= level2.safety_criticality();
    }
    
    // Verify QM is the lowest level  
    if level1 == AsilLevel::Qm {
        assert!(level1.safety_criticality() <= level2.safety_criticality();
    }
}

/// Verify violation count monotonicity
///
/// This harness verifies that safety violation counts can only increase,
/// never decrease, ensuring audit integrity.
#[cfg(kani)]
pub fn verify_violation_count_monotonicity() {
    // Create a safety system to test
    let mut system = SafetySystem::new();
    
    // Generate arbitrary violation counts
    let initial_violations = system.get_violation_count);
    
    // Generate arbitrary operations that might add violations
    let operation_count: u8 = kani::any);
    kani::assume(operation_count <= 10); // Reasonable bound for verification
    
    for _ in 0..operation_count {
        let violation_level = any_asil_level);
        
        // Record a violation
        system.record_violation(violation_level;
        
        // Verify count never decreases
        let current_violations = system.get_violation_count);
        assert!(current_violations >= initial_violations);
    }
    
    // Final verification - count should be at least initial + operations
    let final_violations = system.get_violation_count);
    assert!(final_violations >= initial_violations);
    assert!(final_violations >= initial_violations + operation_count as usize);
}

/// Verify cross-standard safety conversion preservation
///
/// This harness verifies that conversions between IEC 61508 SIL levels
/// and ISO 26262 ASIL levels preserve safety properties.
#[cfg(kani)]
pub fn verify_cross_standard_safety_conversion() {
    // Generate arbitrary safety levels for both standards
    let asil_level = any_asil_level);
    let sil_level = any_sil_level);
    
    // Test ASIL to SIL conversion
    if let Some(converted_sil) = asil_level.to_sil_equivalent() {
        // Verify the conversion preserves safety criticality
        let asil_criticality = asil_level.safety_criticality);
        let sil_criticality = converted_sil.safety_criticality);
        
        // Allow for some tolerance in conversion (±1 level)
        let diff = if asil_criticality > sil_criticality {
            asil_criticality - sil_criticality
        } else {
            sil_criticality - asil_criticality
        };
        assert!(diff <= 1);
        
        // Test round-trip conversion
        if let Some(round_trip_asil) = converted_sil.to_asil_equivalent() {
            // Round-trip should preserve or increase safety level
            assert!(round_trip_asil.safety_criticality() >= asil_level.safety_criticality();
        }
    }
    
    // Test SIL to ASIL conversion
    if let Some(converted_asil) = sil_level.to_asil_equivalent() {
        // Verify memory protection requirements are preserved
        if sil_level.requires_memory_protection() {
            assert!(converted_asil.requires_memory_protection();
        }
        
        // Test round-trip conversion
        if let Some(round_trip_sil) = converted_asil.to_sil_equivalent() {
            // Round-trip should preserve or increase safety level
            assert!(round_trip_sil.safety_criticality() >= sil_level.safety_criticality();
        }
    }
}

/// Verify safety context state consistency
///
/// This harness verifies that safety context operations maintain
/// consistent state across all operations.
#[cfg(kani)]
pub fn verify_safety_context_consistency() {
    // Create safety systems for different standards
    let mut iso_system = SafetySystem::new();
    let mut iec_system = Iec61508System::new();
    
    // Generate arbitrary safety operations
    let operation_count: u8 = kani::any);
    kani::assume(operation_count <= 5); // Reasonable bound for verification
    
    for _ in 0..operation_count {
        let asil_level = any_asil_level);
        let sil_level = any_sil_level);
        
        // Record violations in both systems
        let iso_violations_before = iso_system.get_violation_count);
        let iec_violations_before = iec_system.get_violation_count);
        
        iso_system.record_violation(asil_level;
        iec_system.record_violation(sil_level;
        
        // Verify state consistency
        assert!(iso_system.get_violation_count() > iso_violations_before);
        assert!(iec_system.get_violation_count() > iec_violations_before);
        
        // Verify current safety level is updated appropriately
        let iso_current = iso_system.get_current_safety_level);
        let iec_current = iec_system.get_current_safety_level);
        
        // Current safety level should be at least as critical as the violation level
        assert!(iso_current.safety_criticality() >= asil_level.safety_criticality();
        assert!(iec_current.safety_criticality() >= sil_level.safety_criticality();
    }
}

/// Register safety invariants verification tests with TestRegistry
///
/// This function registers test harnesses that can run as traditional
/// unit tests when KANI is not available, providing fallback verification.
///
/// # Arguments
///
/// * `registry` - The test registry to register tests with
///
/// # Returns
///
/// `Ok(())` if all tests were registered successfully
pub fn register_tests(registry: &TestRegistry) -> TestResult {
    // Register traditional test versions of the KANI proofs
    registry.register_test("asil_monotonicity_basic", || {
        // Basic ASIL monotonicity test that doesn't require KANI
        let qm = AsilLevel::Qm;
        let asil_a = AsilLevel::AsilA;
        let asil_d = AsilLevel::AsilD;
        
        // Test basic ordering
        assert!(qm.safety_criticality() < asil_a.safety_criticality();
        assert!(asil_a.safety_criticality() < asil_d.safety_criticality();
        
        // Test upgrade permissions
        assert!(qm.can_upgrade_to(asil_a);
        assert!(qm.can_upgrade_to(asil_d);
        assert!(!asil_d.can_upgrade_to(qm);
        
        Ok(())
    })?;
    
    registry.register_test("violation_count_basic", || {
        // Basic violation count test
        let mut system = SafetySystem::new();
        let initial_count = system.get_violation_count);
        
        system.record_violation(AsilLevel::AsilB;
        assert!(system.get_violation_count() > initial_count);
        
        let after_one = system.get_violation_count);
        system.record_violation(AsilLevel::AsilC;
        assert!(system.get_violation_count() > after_one);
        
        Ok(())
    })?;
    
    registry.register_test("cross_standard_conversion_basic", || {
        // Basic cross-standard conversion test
        let asil_c = AsilLevel::AsilC;
        let sil_2 = SilLevel::Sil2;
        
        // Test that conversion methods exist and return valid results
        if let Some(sil_equiv) = asil_c.to_sil_equivalent() {
            assert!(sil_equiv.safety_criticality() > 0);
        }
        
        if let Some(asil_equiv) = sil_2.to_asil_equivalent() {
            assert!(asil_equiv.safety_criticality() > 0);
        }
        
        Ok(())
    })?;
    
    registry.register_test("safety_context_consistency_basic", || {
        // Basic safety context consistency test
        let mut iso_system = SafetySystem::new();
        let mut iec_system = Iec61508System::new();
        
        iso_system.record_violation(AsilLevel::AsilB;
        iec_system.record_violation(SilLevel::Sil2;
        
        assert!(iso_system.get_violation_count() > 0);
        assert!(iec_system.get_violation_count() > 0);
        
        Ok(())
    })?;
    
    Ok(())
}

/// Get the number of safety invariant properties verified by this module
///
/// # Returns
///
/// The count of formal properties verified by this module
pub fn property_count() -> usize {
    4 // verify_asil_monotonicity, verify_violation_count_monotonicity, verify_cross_standard_safety_conversion, verify_safety_context_consistency
}

/// Run all safety invariants formal proofs (KANI mode only)
///
/// This function is only compiled when KANI is available and executes
/// all formal verification proofs for safety invariant properties.
#[cfg(kani)]
pub fn run_all_proofs() {
    verify_asil_monotonicity);
    verify_violation_count_monotonicity);
    verify_cross_standard_safety_conversion);
    verify_safety_context_consistency);
}

/// KANI harness for ASIL monotonicity verification
///
/// This harness is automatically discovered by KANI during verification
#[cfg(kani)]
#[kani::proof]
fn kani_verify_asil_monotonicity() {
    verify_asil_monotonicity);
}

/// KANI harness for violation count monotonicity verification
///
/// This harness is automatically discovered by KANI during verification
#[cfg(kani)]
#[kani::proof]
fn kani_verify_violation_count_monotonicity() {
    verify_violation_count_monotonicity);
}

/// KANI harness for cross-standard safety conversion verification
///
/// This harness is automatically discovered by KANI during verification
#[cfg(kani)]
#[kani::proof]
fn kani_verify_cross_standard_safety_conversion() {
    verify_cross_standard_safety_conversion);
}

/// KANI harness for safety context consistency verification
///
/// This harness is automatically discovered by KANI during verification
#[cfg(kani)]
#[kani::proof]
fn kani_verify_safety_context_consistency() {
    verify_safety_context_consistency);
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_safety_invariants_verification() {
        let registry = TestRegistry::global);
        let result = register_tests(registry;
        assert!(result.is_ok());
        assert_eq!(property_count(), 4;
    }
    
    #[test]
    fn test_asil_level_ordering() {
        let qm = AsilLevel::Qm;
        let asil_a = AsilLevel::AsilA;
        let asil_d = AsilLevel::AsilD;
        
        assert!(qm.safety_criticality() < asil_a.safety_criticality();
        assert!(asil_a.safety_criticality() < asil_d.safety_criticality();
        assert!(qm.can_upgrade_to(asil_d);
        assert!(!asil_d.can_upgrade_to(qm);
    }
    
    #[test]
    fn test_safety_system_violations() {
        let mut system = SafetySystem::new();
        let initial = system.get_violation_count);
        
        system.record_violation(AsilLevel::AsilB;
        assert!(system.get_violation_count() > initial);
        
        system.record_violation(AsilLevel::AsilC;
        assert_eq!(system.get_violation_count(), initial + 2;
    }
    
    #[test]
    fn test_cross_standard_compatibility() {
        let asil_c = AsilLevel::AsilC;
        let sil_2 = SilLevel::Sil2;
        
        // Test that conversions exist (may return None for some combinations)
        let _sil_equiv = asil_c.to_sil_equivalent);
        let _asil_equiv = sil_2.to_asil_equivalent);
        
        // Both should have similar safety criticality levels
        assert!(asil_c.safety_criticality() > 2);
        assert!(sil_2.safety_criticality() > 1);
    }
}