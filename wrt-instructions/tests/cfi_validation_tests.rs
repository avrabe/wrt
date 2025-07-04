//! Tests for CFI validation implementations
//!
//! This module tests the control flow integrity validation functions
//! to ensure they properly detect and prevent CFI violations.

#[cfg(test)]
mod tests {
    use wrt_instructions::cfi_control_ops::{
        CallingConvention, CfiExecutionContext, CfiMetrics, DefaultCfiControlFlowOps,
        ShadowStackEntry, SoftwareCfiConfig,
    };
    use wrt_error::{codes, Error, ErrorCategory};

    #[test]
    fn test_label_resolution_valid() {
        let ops = DefaultCfiControlFlowOps;
        let mut context = CfiExecutionContext::default();
        context.max_labels = 10;
        context.valid_branch_targets = Some(vec![0, 1, 2, 5, 7]);

        // Valid label should resolve successfully
        let result = ops.resolve_label_to_offset(2, &context);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 2);
    }

    #[test]
    fn test_label_resolution_out_of_bounds() {
        let ops = DefaultCfiControlFlowOps;
        let context = CfiExecutionContext {
            max_labels: 10,
            ..Default::default()
        };

        // Label index out of bounds should fail
        let result = ops.resolve_label_to_offset(15, &context);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.category(), ErrorCategory::ControlFlow);
    }

    #[test]
    fn test_label_resolution_invalid_target() {
        let ops = DefaultCfiControlFlowOps;
        let context = CfiExecutionContext {
            max_labels: 10,
            valid_branch_targets: Some(vec![0, 1, 2, 5, 7]),
            ..Default::default()
        };

        // Valid index but not in allowed targets should fail
        let result = ops.resolve_label_to_offset(3, &context);
        assert!(result.is_err());
    }

    #[test]
    fn test_type_signature_validation_success() {
        let ops = DefaultCfiControlFlowOps;
        let context = CfiExecutionContext {
            max_types: 10,
            type_signatures: vec![0x1234, 0x5678, 0xABCD],
            ..Default::default()
        };

        // Matching signature should pass
        let result = ops.validate_type_signature(1, 0x5678, &context);
        assert!(result.is_ok());
    }

    #[test]
    fn test_type_signature_validation_mismatch() {
        let ops = DefaultCfiControlFlowOps;
        let context = CfiExecutionContext {
            max_types: 10,
            type_signatures: vec![0x1234, 0x5678, 0xABCD],
            ..Default::default()
        };

        // Mismatched signature should fail
        let result = ops.validate_type_signature(1, 0x9999, &context);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.category(), ErrorCategory::Security);
    }

    #[test]
    fn test_shadow_stack_validation_overflow() {
        let ops = DefaultCfiControlFlowOps;
        let mut context = CfiExecutionContext::default();
        context.max_shadow_stack_depth = 3;
        
        // Fill shadow stack to max
        for i in 0..4 {
            context.shadow_stack.push(ShadowStackEntry {
                return_address: i * 100,
                function_index: i,
                ..Default::default()
            });
        }

        // Should detect overflow
        let result = ops.validate_shadow_stack(&context);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.code(), codes::STACK_OVERFLOW);
    }

    #[test]
    fn test_shadow_stack_validation_underflow() {
        let ops = DefaultCfiControlFlowOps;
        let context = CfiExecutionContext {
            current_function: 5, // Non-zero function
            shadow_stack: vec![], // Empty stack
            ..Default::default()
        };

        // Should detect underflow
        let result = ops.validate_shadow_stack(&context);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.category(), ErrorCategory::Security);
    }

    #[test]
    fn test_shadow_stack_return_address_mismatch() {
        let ops = DefaultCfiControlFlowOps;
        let context = CfiExecutionContext {
            current_instruction: 500,
            shadow_stack: vec![ShadowStackEntry {
                return_address: 400, // Different from current
                function_index: 1,
                ..Default::default()
            }],
            ..Default::default()
        };

        // Should detect return address mismatch
        let result = ops.validate_shadow_stack(&context);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.category(), ErrorCategory::Security);
        assert!(err.message().contains("ROP"));
    }

    #[test]
    fn test_control_flow_target_validation_valid() {
        let ops = DefaultCfiControlFlowOps;
        let context = CfiExecutionContext {
            current_instruction: 200,
            ..Default::default()
        };

        let valid_targets = vec![100, 200, 300];
        
        // Current instruction is in valid targets
        let result = ops.validate_control_flow_target(&valid_targets, &context);
        assert!(result.is_ok());
    }

    #[test]
    fn test_control_flow_target_validation_invalid() {
        let ops = DefaultCfiControlFlowOps;
        let context = CfiExecutionContext {
            current_instruction: 250,
            ..Default::default()
        };

        let valid_targets = vec![100, 200, 300];
        
        // Current instruction not in valid targets
        let result = ops.validate_control_flow_target(&valid_targets, &context);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.category(), ErrorCategory::Security);
    }

    #[test]
    fn test_control_flow_target_empty_list() {
        let ops = DefaultCfiControlFlowOps;
        let context = CfiExecutionContext::default();
        let valid_targets = vec![];
        
        // Empty target list should fail
        let result = ops.validate_control_flow_target(&valid_targets, &context);
        assert!(result.is_err());
    }

    #[test]
    fn test_calling_convention_stack_alignment() {
        let ops = DefaultCfiControlFlowOps;
        let context = CfiExecutionContext {
            current_stack_depth: 17, // Misaligned
            calling_convention: CallingConvention::WebAssembly,
            ..Default::default()
        };

        // Should detect misalignment
        let result = ops.validate_calling_convention(&context);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.code(), codes::MEMORY_ALIGNMENT_ERROR);
    }

    #[test]
    fn test_calling_convention_systemv_red_zone() {
        let ops = DefaultCfiControlFlowOps;
        let context = CfiExecutionContext {
            current_stack_depth: 64, // Less than 128
            calling_convention: CallingConvention::SystemV,
            ..Default::default()
        };

        // Should detect insufficient red zone
        let result = ops.validate_calling_convention(&context);
        assert!(result.is_err());
    }

    #[test]
    fn test_calling_convention_windows_shadow_space() {
        let ops = DefaultCfiControlFlowOps;
        let context = CfiExecutionContext {
            current_stack_depth: 16, // Less than 32
            calling_convention: CallingConvention::WindowsFastcall,
            ..Default::default()
        };

        // Should detect insufficient shadow space
        let result = ops.validate_calling_convention(&context);
        assert!(result.is_err());
    }

    #[test]
    fn test_temporal_validation_disabled() {
        let ops = DefaultCfiControlFlowOps;
        let mut context = CfiExecutionContext::default();
        context.software_config.temporal_validation = false;
        
        // Should skip validation when disabled
        let result = ops.validate_temporal_properties(1000, &context);
        assert!(result.is_ok());
    }

    #[test]
    fn test_temporal_validation_timeout() {
        let ops = DefaultCfiControlFlowOps;
        let mut context = CfiExecutionContext::default();
        context.software_config.temporal_validation = true;
        context.metrics.total_execution_time = 2000;
        
        // Should detect timeout
        let result = ops.validate_temporal_properties(1000, &context);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.code(), codes::TIMEOUT);
    }

    #[test]
    fn test_temporal_validation_timing_anomaly() {
        let ops = DefaultCfiControlFlowOps;
        let mut context = CfiExecutionContext::default();
        context.software_config.temporal_validation = true;
        context.metrics.average_instruction_time = Some(100);
        context.metrics.last_instruction_time = 1500; // 15x average
        
        // Should detect timing anomaly
        let result = ops.validate_temporal_properties(10000, &context);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.message().contains("side-channel"));
    }

    #[test]
    fn test_temporal_validation_time_regression() {
        let ops = DefaultCfiControlFlowOps;
        let mut context = CfiExecutionContext::default();
        context.software_config.temporal_validation = true;
        context.metrics.total_execution_time = 1000;
        context.last_checkpoint_time = 2000; // Time went backwards
        
        // Should detect time regression
        let result = ops.validate_temporal_properties(10000, &context);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.message().contains("clock manipulation"));
    }
}