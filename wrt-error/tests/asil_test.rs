//! Tests for ASIL-specific error handling features

use wrt_error::{codes, Error, ErrorCategory};

#[cfg(any(feature = "asil-b", feature = "asil-c", feature = "asil-d"))]
use wrt_error::{AsilErrorContext, AsilLevel};

#[test]
fn test_basic_error_creation() {
    let error = Error::new(
        ErrorCategory::Memory,
        codes::MEMORY_OUT_OF_BOUNDS,
        "Out of bounds",
    );
    assert_eq!(error.code, codes::MEMORY_OUT_OF_BOUNDS);
    assert_eq!(error.category, ErrorCategory::Memory);
}

#[cfg(any(feature = "asil-b", feature = "asil-c", feature = "asil-d"))]
#[test]
fn test_asil_level_detection() {
    let error = Error::new(
        ErrorCategory::Safety,
        codes::SAFETY_VIOLATION,
        "Safety violation",
    );
    let asil_level = error.asil_level();

    // Safety errors should be ASIL-D
    assert_eq!(asil_level, "ASIL-D");

    // Memory errors should be ASIL-C
    let mem_error = Error::new(
        ErrorCategory::Memory,
        codes::MEMORY_OUT_OF_BOUNDS,
        "Memory error",
    );
    assert_eq!(mem_error.asil_level(), "ASIL-C");

    // Validation errors should be ASIL-B
    let val_error = Error::new(
        ErrorCategory::Validation,
        codes::VALIDATION_ERROR,
        "Validation error",
    );
    assert_eq!(val_error.asil_level(), "ASIL-B");
}

#[cfg(any(feature = "asil-c", feature = "asil-d"))]
#[test]
fn test_safe_state_requirements() {
    // Safety errors require safe state
    let safety_error = Error::new(
        ErrorCategory::Safety,
        codes::SAFETY_VIOLATION,
        "Critical error",
    );
    assert!(safety_error.requires_safe_state());

    // Memory errors require safe state
    let memory_error = Error::new(
        ErrorCategory::Memory,
        codes::MEMORY_CORRUPTION_DETECTED,
        "Memory corruption",
    );
    assert!(memory_error.requires_safe_state());

    // Runtime trap errors require safe state
    let trap_error = Error::new(
        ErrorCategory::RuntimeTrap,
        codes::RUNTIME_TRAP_ERROR,
        "Trap occurred",
    );
    assert!(trap_error.requires_safe_state());

    // Component errors do not require immediate safe state
    let comp_error = Error::new(
        ErrorCategory::Component,
        codes::COMPONENT_LINKING_ERROR,
        "Component issue",
    );
    assert!(!comp_error.requires_safe_state());
}

#[cfg(feature = "asil-d")]
#[test]
fn test_error_integrity_validation() {
    // Valid error
    let valid_error = Error::new(
        ErrorCategory::Memory,
        codes::MEMORY_OUT_OF_BOUNDS,
        "Valid error",
    );
    assert!(valid_error.validate_integrity());

    // Error with valid code for category
    let type_error = Error::new(
        ErrorCategory::Type,
        codes::TYPE_MISMATCH_ERROR,
        "Type mismatch",
    );
    assert!(type_error.validate_integrity());

    // Note: We can't easily test invalid errors in const fn context
}

#[cfg(any(feature = "asil-b", feature = "asil-c", feature = "asil-d"))]
#[test]
fn test_asil_error_context() {
    let error = Error::new(ErrorCategory::Safety, codes::SAFETY_VIOLATION, "Test error");
    let context = AsilErrorContext::new(error.clone())
        .with_timestamp(123456789)
        .with_module_id(42);

    assert_eq!(context.error.code, error.code);
    assert_eq!(context.timestamp, Some(123456789));
    assert_eq!(context.module_id, Some(42));
    assert!(context.requires_immediate_action());
}

#[cfg(any(feature = "asil-c", feature = "asil-d"))]
#[test]
fn test_safety_monitor() {
    use wrt_error::SafetyMonitor;

    let monitor = SafetyMonitor::new();
    assert_eq!(monitor.error_count(), 0);

    // Record some errors
    let error1 = Error::new(
        ErrorCategory::Memory,
        codes::MEMORY_OUT_OF_BOUNDS,
        "Error 1",
    );
    let error2 = Error::new(ErrorCategory::Safety, codes::SAFETY_VIOLATION, "Error 2");

    monitor.record_error(&error1);
    monitor.record_error(&error2);

    assert_eq!(monitor.error_count(), 2);

    // Reset monitor
    monitor.reset();
    assert_eq!(monitor.error_count(), 0);
}

#[test]
fn test_error_display_format() {
    let error = Error::new(
        ErrorCategory::Memory,
        codes::MEMORY_OUT_OF_BOUNDS,
        "Test error",
    );
    let display = format!("{}", error);

    #[cfg(any(feature = "asil-c", feature = "asil-d"))]
    {
        // ASIL-C/D includes ASIL level in display
        assert!(display.contains("[Memory]"));
        assert!(display.contains("[E0FA0]")); // MEMORY_OUT_OF_BOUNDS = 4000 = 0x0FA0
        assert!(display.contains("[ASIL-C]")); // Memory errors are ASIL-C
        assert!(display.contains("Test error"));
    }

    #[cfg(not(any(feature = "asil-c", feature = "asil-d")))]
    {
        // Standard format without ASIL level
        assert!(display.contains("[Memory]"));
        assert!(display.contains("[E0FA0]")); // MEMORY_OUT_OF_BOUNDS = 4000 = 0x0FA0
        assert!(display.contains("Test error"));
        assert!(!display.contains("ASIL"));
    }
}

#[cfg(any(feature = "asil-b", feature = "asil-c", feature = "asil-d"))]
#[test]
fn test_current_asil_level() {
    let current = AsilLevel::current();

    #[cfg(feature = "asil-d")]
    assert_eq!(current, AsilLevel::AsilD);

    #[cfg(all(feature = "asil-c", not(feature = "asil-d")))]
    assert_eq!(current, AsilLevel::AsilC);

    #[cfg(all(feature = "asil-b", not(feature = "asil-c")))]
    assert_eq!(current, AsilLevel::AsilB);

    // Test requirement checking
    assert!(AsilLevel::meets_requirement(AsilLevel::QM));

    #[cfg(feature = "asil-d")]
    {
        assert!(AsilLevel::meets_requirement(AsilLevel::AsilD));
        assert!(AsilLevel::meets_requirement(AsilLevel::AsilC));
        assert!(AsilLevel::meets_requirement(AsilLevel::AsilB));
    }
}
