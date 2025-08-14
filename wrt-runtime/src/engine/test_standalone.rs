//! Standalone test module for capability engine
//!
//! This module tests the capability engine in isolation
//! to verify the design works correctly.

#[cfg(test)]
mod tests {
    use wrt_foundation::verification::VerificationLevel;

    use crate::engine::{
        EnginePreset,
        InstanceHandle,
        ModuleHandle,
    };

    #[test]
    fn test_engine_preset_variants() {
        // Test all preset variants exist
        let _qm = EnginePreset::QM;
        let _asil_a = EnginePreset::AsilA;
        let _asil_b = EnginePreset::AsilB;
    }

    #[test]
    fn test_module_handle_uniqueness() {
        let h1 = ModuleHandle::new();
        let h2 = ModuleHandle::new();
        assert_ne!(h1, h2);
    }

    #[test]
    fn test_instance_handle_index() {
        let h = InstanceHandle::from_index(42);
        assert_eq!(h.index(), 42);
    }

    #[test]
    fn test_preset_capability_levels() {
        use crate::engine::presets;

        // Test QM preset
        let qm_context = presets::qm().expect("QM preset should work");
        assert_eq!(
            qm_context.default_verification_level(),
            VerificationLevel::Standard
        );

        // Test ASIL-A preset
        let asil_a_context = presets::asil_a().expect("ASIL-A preset should work");
        assert_eq!(
            asil_a_context.default_verification_level(),
            VerificationLevel::Sampling
        );

        // Test ASIL-B preset
        let asil_b_context = presets::asil_b().expect("ASIL-B preset should work");
        assert_eq!(
            asil_b_context.default_verification_level(),
            VerificationLevel::Continuous
        );
    }
}
