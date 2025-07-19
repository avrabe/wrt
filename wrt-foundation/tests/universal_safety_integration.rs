//! Integration test for Universal Safety System

use wrt_foundation::safety_system::*;

#[test]
fn test_universal_safety_integration() {
    // Test basic ASIL functionality
    let asil_c = SafetyStandard::Iso26262(AsilLevel::AsilC;
    assert_eq!(asil_c.severity_score().value(), 750;

    // Test cross-standard conversion
    let dal_equivalent = asil_c.convert_to(SafetyStandardType::Do178c).unwrap();
    if let SafetyStandard::Do178c(level) = dal_equivalent {
        assert_eq!(level, DalLevel::DalB); // 750 severity maps to DAL-B
    } else {
        panic!("Conversion failed";
    }

    let sil_equivalent = asil_c.convert_to(SafetyStandardType::Iec61508).unwrap();
    if let SafetyStandard::Iec61508(level) = sil_equivalent {
        assert_eq!(level, SilLevel::Sil3); // 750 severity maps to SIL-3
    } else {
        panic!("Conversion failed";
    }

    // Test compatibility checking
    let asil_b = SafetyStandard::Iso26262(AsilLevel::AsilB;
    assert!(asil_c.is_compatible_with(&asil_b);
    assert!(!asil_b.is_compatible_with(&asil_c);

    // Test Universal Safety Context
    let mut ctx = UniversalSafetyContext::new(SafetyStandard::Iso26262(AsilLevel::AsilB;
    assert_eq!(ctx.effective_severity().value(), 500;

    ctx.add_secondary_standard(SafetyStandard::Do178c(DalLevel::DalA)).unwrap();
    assert_eq!(ctx.effective_severity().value(), 1000); // Should be the highest

    // Test macro usage
    let macro_ctx = universal_safety_context!(Iso26262(AsilC;
    assert_eq!(macro_ctx.effective_severity().value(), 750;

    // Test verification behavior
    let high_severity_ctx = UniversalSafetyContext::new(SafetyStandard::Iso26262(AsilLevel::AsilD;
    assert!(high_severity_ctx.should_verify())); // ASIL-D should always verify

    let low_severity_ctx = UniversalSafetyContext::new(SafetyStandard::Iso26262(AsilLevel::QM;
    assert!(!low_severity_ctx.should_verify())); // QM should not verify
}

#[test]
fn test_cross_standard_edge_cases() {
    // Test that QM can't convert to medical (no safety class)
    let qm = SafetyStandard::Iso26262(AsilLevel::QM;
    assert!(qm.convert_to(SafetyStandardType::Iec62304).is_none();

    // Test that SIL can convert to ISO 26262
    let sil_3 = SafetyStandard::Iec61508(SilLevel::Sil3;
    let iso_equivalent = sil_3.convert_to(SafetyStandardType::Iso26262).unwrap();
    if let SafetyStandard::Iso26262(level) = iso_equivalent {
        assert_eq!(level, AsilLevel::AsilC;
    } else {
        panic!("Conversion failed";
    }
}

#[test]
fn test_severity_score_bounds() {
    // Test severity score creation
    assert!(SeverityScore::new(0).is_ok();
    assert!(SeverityScore::new(500).is_ok();
    assert!(SeverityScore::new(1000).is_ok();
    assert!(SeverityScore::new(1001).is_err();
}

#[test]
fn test_multi_standard_context() {
    let mut ctx = UniversalSafetyContext::new(SafetyStandard::Iso26262(AsilLevel::AsilA;

    // Add multiple secondary standards
    ctx.add_secondary_standard(SafetyStandard::Do178c(DalLevel::DalB)).unwrap();
    ctx.add_secondary_standard(SafetyStandard::Iec61508(SilLevel::Sil2)).unwrap();

    // Should be able to handle all of them
    assert!(ctx.can_handle(SafetyStandard::Iso26262(AsilLevel::AsilA));
    assert!(ctx.can_handle(SafetyStandard::Do178c(DalLevel::DalB));
    assert!(ctx.can_handle(SafetyStandard::Iec61508(SilLevel::Sil2));

    // Should not be able to handle higher requirements
    assert!(!ctx.can_handle(SafetyStandard::Iso26262(AsilLevel::AsilD));
}
