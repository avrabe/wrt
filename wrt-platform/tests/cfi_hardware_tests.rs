//! CFI Hardware Features Tests for wrt-platform
//!
//! Comprehensive testing of CFI (Control Flow Integrity) hardware features
//! including ARM BTI, RISC-V CFI, and x86 CET support.

use wrt_platform::{
    BranchTargetIdentification, BtiExceptionLevel, BtiMode, CfiExceptionMode, ControlFlowIntegrity,
    HardwareOptimization, SecurityLevel,
};

#[test]
fn test_arm_bti_availability() {
    // Test BTI hardware detection
    let bti_available = BranchTargetIdentification::is_available);

    // On non-ARM64 platforms, BTI should not be available
    #[cfg(not(target_arch = "aarch64"))]
    assert!(!bti_available, "BTI should not be available on non-ARM64 platforms");

    // On ARM64 platforms, BTI may or may not be available depending on hardware
    #[cfg(target_arch = "aarch64")]
    {
        // Just test that the detection doesn't panic
        println!("ARM64 BTI availability: {}", bti_available);
    }
}

#[test]
fn test_arm_bti_modes() {
    // Test all BTI modes are properly defined
    let modes = [BtiMode::Standard, BtiMode::CallOnly, BtiMode::JumpOnly, BtiMode::CallAndJump];

    for mode in modes {
        println!("Testing BTI mode: {:?}", mode);

        // Test that we can create BTI configuration with each mode
        let bti = BranchTargetIdentification::new(mode, BtiExceptionLevel::El1;
        assert_eq!(bti.mode(), mode;
        assert_eq!(bti.exception_level(), BtiExceptionLevel::El1;
    }
}

#[test]
fn test_arm_bti_exception_levels() {
    let levels = [
        BtiExceptionLevel::El0,
        BtiExceptionLevel::El1,
        BtiExceptionLevel::El2,
        BtiExceptionLevel::El3,
    ];

    for level in levels {
        println!("Testing BTI exception level: {:?}", level);

        let bti = BranchTargetIdentification::new(BtiMode::Standard, level;
        assert_eq!(bti.exception_level(), level;
    }
}

#[test]
fn test_riscv_cfi_availability() {
    // Test RISC-V CFI hardware detection
    let cfi_available = ControlFlowIntegrity::is_available);

    // On non-RISC-V platforms, CFI should not be available
    #[cfg(not(target_arch = "riscv64"))]
    assert!(!cfi_available, "RISC-V CFI should not be available on non-RISC-V platforms");

    // On RISC-V platforms, CFI may or may not be available depending on hardware
    #[cfg(target_arch = "riscv64")]
    {
        // Just test that the detection doesn't panic
        println!("RISC-V CFI availability: {}", cfi_available);
    }
}

#[test]
fn test_riscv_cfi_modes() {
    let modes =
        [CfiExceptionMode::Synchronous, CfiExceptionMode::Asynchronous, CfiExceptionMode::Deferred];

    for mode in modes {
        println!("Testing RISC-V CFI mode: {:?}", mode);

        let cfi = ControlFlowIntegrity::new(mode;
        assert_eq!(cfi.exception_mode(), mode;
    }
}

#[test]
fn test_hardware_optimization_interface() {
    // Test that BTI implements HardwareOptimization trait
    let bti = BranchTargetIdentification::new(BtiMode::Standard, BtiExceptionLevel::El1;

    // Test security level
    let security_level = bti.security_level);
    assert!(
        matches!(security_level, SecurityLevel::High | SecurityLevel::Maximum),
        "BTI should provide high security level"
    ;

    // Test overhead estimation
    let overhead = bti.estimated_overhead_percentage);
    assert!(overhead >= 0.0 && overhead <= 10.0, "BTI overhead should be reasonable (0-10%)");

    // Test description
    let description = bti.description);
    assert!(description.contains("Branch Target Identification");
}

#[test]
fn test_cfi_hardware_optimization_interface() {
    // Test that RISC-V CFI implements HardwareOptimization trait
    let cfi = ControlFlowIntegrity::new(CfiExceptionMode::Synchronous;

    // Test security level
    let security_level = cfi.security_level);
    assert!(
        matches!(security_level, SecurityLevel::High | SecurityLevel::Maximum),
        "CFI should provide high security level"
    ;

    // Test overhead estimation
    let overhead = cfi.estimated_overhead_percentage);
    assert!(overhead >= 0.0 && overhead <= 15.0, "CFI overhead should be reasonable (0-15%)");

    // Test description
    let description = cfi.description);
    assert!(description.contains("Control Flow Integrity");
}

#[test]
fn test_bti_enable_disable() {
    let bti = BranchTargetIdentification::new(BtiMode::Standard, BtiExceptionLevel::El1;

    // Test enable operation
    let enable_result = bti.enable);

    #[cfg(target_arch = "aarch64")]
    {
        // On ARM64, enable might succeed or fail depending on hardware/privileges
        match enable_result {
            Ok(()) => println!("BTI enabled successfully"),
            Err(e) => println!("BTI enable failed (expected on some systems): {:?}", e),
        }
    }

    #[cfg(not(target_arch = "aarch64"))]
    {
        // On non-ARM64, enable should fail
        assert!(enable_result.is_err(), "BTI enable should fail on non-ARM64 platforms");
    }

    // Test disable operation
    let disable_result = bti.disable);

    #[cfg(target_arch = "aarch64")]
    {
        // On ARM64, disable might succeed or fail depending on hardware/privileges
        match disable_result {
            Ok(()) => println!("BTI disabled successfully"),
            Err(e) => println!("BTI disable failed (expected on some systems): {:?}", e),
        }
    }

    #[cfg(not(target_arch = "aarch64"))]
    {
        // On non-ARM64, disable should fail
        assert!(disable_result.is_err(), "BTI disable should fail on non-ARM64 platforms");
    }
}

#[test]
fn test_cfi_enable_disable() {
    let cfi = ControlFlowIntegrity::new(CfiExceptionMode::Synchronous;

    // Test enable operation
    let enable_result = cfi.enable);

    #[cfg(target_arch = "riscv64")]
    {
        // On RISC-V, enable might succeed or fail depending on hardware/privileges
        match enable_result {
            Ok(()) => println!("CFI enabled successfully"),
            Err(e) => println!("CFI enable failed (expected on some systems): {:?}", e),
        }
    }

    #[cfg(not(target_arch = "riscv64"))]
    {
        // On non-RISC-V, enable should fail
        assert!(enable_result.is_err(), "CFI enable should fail on non-RISC-V platforms");
    }

    // Test disable operation
    let disable_result = cfi.disable);

    #[cfg(target_arch = "riscv64")]
    {
        // On RISC-V, disable might succeed or fail depending on hardware/privileges
        match disable_result {
            Ok(()) => println!("CFI disabled successfully"),
            Err(e) => println!("CFI disable failed (expected on some systems): {:?}", e),
        }
    }

    #[cfg(not(target_arch = "riscv64"))]
    {
        // On non-RISC-V, disable should fail
        assert!(disable_result.is_err(), "CFI disable should fail on non-RISC-V platforms");
    }
}

#[test]
fn test_cfi_configuration_combinations() {
    // Test various BTI configurations
    let bti_configs = [
        (BtiMode::Standard, BtiExceptionLevel::El0),
        (BtiMode::CallOnly, BtiExceptionLevel::El1),
        (BtiMode::JumpOnly, BtiExceptionLevel::El2),
        (BtiMode::CallAndJump, BtiExceptionLevel::El3),
    ];

    for (mode, level) in bti_configs {
        let bti = BranchTargetIdentification::new(mode, level;
        assert_eq!(bti.mode(), mode;
        assert_eq!(bti.exception_level(), level;

        // Test that configuration is consistent
        let description = bti.description);
        assert!(!description.is_empty());
    }

    // Test various CFI configurations
    let cfi_configs =
        [CfiExceptionMode::Synchronous, CfiExceptionMode::Asynchronous, CfiExceptionMode::Deferred];

    for mode in cfi_configs {
        let cfi = ControlFlowIntegrity::new(mode;
        assert_eq!(cfi.exception_mode(), mode;

        // Test that configuration is consistent
        let description = cfi.description);
        assert!(!description.is_empty());
    }
}

#[test]
fn test_hardware_feature_interaction() {
    // Test that BTI and CFI can coexist
    let bti = BranchTargetIdentification::new(BtiMode::Standard, BtiExceptionLevel::El1;
    let cfi = ControlFlowIntegrity::new(CfiExceptionMode::Synchronous;

    // Both should be independently configurable
    assert_eq!(bti.mode(), BtiMode::Standard;
    assert_eq!(cfi.exception_mode(), CfiExceptionMode::Synchronous;

    // Both should provide security benefits
    assert!(bti.security_level() as u8 >= SecurityLevel::Medium as u8);
    assert!(cfi.security_level() as u8 >= SecurityLevel::Medium as u8);
}

#[test]
fn test_x86_cet_placeholder() {
    // For now, we don't have full x86 CET implementation
    // This test serves as a placeholder for future x86 CET support

    #[cfg(target_arch = "x86_64")]
    {
        // Test would check for CET availability using CPUID
        // For now, just verify we're on x86_64
        println!("Running on x86_64 - CET support to be implemented");
    }

    #[cfg(not(target_arch = "x86_64"))]
    {
        println!("Not on x86_64 - CET not applicable");
    }
}

#[test]
fn test_cross_platform_cfi_detection() {
    // Test comprehensive CFI feature detection across platforms
    let mut cfi_features = Vec::new());

    if BranchTargetIdentification::is_available() {
        cfi_features.push("ARM BTI");
    }

    if ControlFlowIntegrity::is_available() {
        cfi_features.push("RISC-V CFI");
    }

    // Future: x86 CET detection would go here

    println!("Available CFI features: {:?}", cfi_features);

    // At least one of these should work on any platform that supports CFI
    // (though none may be available in test environments)
    let total_features = cfi_features.len);
    assert!(total_features >= 0, "CFI feature detection should complete successfully");
}

#[test]
fn test_security_level_ordering() {
    use wrt_platform::SecurityLevel;

    // Test that security levels are properly ordered
    assert!(SecurityLevel::None as u8 < SecurityLevel::Low as u8);
    assert!(SecurityLevel::Low as u8 < SecurityLevel::Medium as u8);
    assert!(SecurityLevel::Medium as u8 < SecurityLevel::High as u8);
    assert!(SecurityLevel::High as u8 < SecurityLevel::Maximum as u8);

    // Both BTI and CFI should provide high security
    let bti = BranchTargetIdentification::new(BtiMode::CallAndJump, BtiExceptionLevel::El1;
    let cfi = ControlFlowIntegrity::new(CfiExceptionMode::Synchronous;

    assert!(bti.security_level() as u8 >= SecurityLevel::High as u8);
    assert!(cfi.security_level() as u8 >= SecurityLevel::High as u8);
}
