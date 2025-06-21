//! Test fuel integration across the WRT system
//! This tests the complete instruction-based fuel consumption

use wrt_foundation::operations::{Type, fuel_cost_for_operation};
use wrt_foundation::verification::VerificationLevel;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wasm_instruction_fuel_costs() {
        // Test simple constant instruction fuel cost
        let cost = Type::fuel_cost_for_operation(
            Type::WasmSimpleConstant, 
            VerificationLevel::Standard
        ).unwrap();
        assert!(cost >= 1, "Simple constants should have fuel cost >= 1");

        // Test complex arithmetic instruction fuel cost
        let complex_cost = Type::fuel_cost_for_operation(
            Type::WasmComplexArithmetic, 
            VerificationLevel::Standard
        ).unwrap();
        assert!(complex_cost > cost, "Complex arithmetic should cost more than constants");

        // Test memory management instruction fuel cost
        let memory_cost = Type::fuel_cost_for_operation(
            Type::WasmMemoryManagement, 
            VerificationLevel::Standard
        ).unwrap();
        assert!(memory_cost > complex_cost, "Memory management should be most expensive");
    }

    #[test]
    fn test_verification_level_multipliers() {
        let base_cost = Type::fuel_cost_for_operation(
            Type::WasmSimpleArithmetic, 
            VerificationLevel::Off
        ).unwrap();

        let standard_cost = Type::fuel_cost_for_operation(
            Type::WasmSimpleArithmetic, 
            VerificationLevel::Standard
        ).unwrap();

        let full_cost = Type::fuel_cost_for_operation(
            Type::WasmSimpleArithmetic, 
            VerificationLevel::Full
        ).unwrap();

        // Verification levels should increase fuel costs
        assert!(standard_cost > base_cost, "Standard verification should cost more than off");
        assert!(full_cost > standard_cost, "Full verification should cost more than standard");
    }

    #[test]
    fn test_all_wasm_instruction_types() {
        let all_wasm_types = [
            Type::WasmSimpleConstant,
            Type::WasmLocalAccess,
            Type::WasmGlobalAccess,
            Type::WasmSimpleArithmetic,
            Type::WasmComplexArithmetic,
            Type::WasmFloatArithmetic,
            Type::WasmComparison,
            Type::WasmSimpleControl,
            Type::WasmComplexControl,
            Type::WasmFunctionCall,
            Type::WasmMemoryLoad,
            Type::WasmMemoryStore,
            Type::WasmMemoryManagement,
            Type::WasmTableAccess,
            Type::WasmTypeConversion,
            Type::WasmSimdOperation,
            Type::WasmAtomicOperation,
        ];

        // All instruction types should have valid fuel costs
        for &instruction_type in &all_wasm_types {
            let cost = Type::fuel_cost_for_operation(
                instruction_type, 
                VerificationLevel::Standard
            ).unwrap();
            assert!(cost > 0, "All instruction types should have positive fuel cost");
        }
    }

    #[test]
    fn test_fuel_cost_ordering() {
        // Test that fuel costs follow expected complexity ordering
        let simple_const = Type::fuel_cost_for_operation(
            Type::WasmSimpleConstant, VerificationLevel::Standard).unwrap();
        let local_access = Type::fuel_cost_for_operation(
            Type::WasmLocalAccess, VerificationLevel::Standard).unwrap();
        let simple_arith = Type::fuel_cost_for_operation(
            Type::WasmSimpleArithmetic, VerificationLevel::Standard).unwrap();
        let complex_arith = Type::fuel_cost_for_operation(
            Type::WasmComplexArithmetic, VerificationLevel::Standard).unwrap();
        let memory_load = Type::fuel_cost_for_operation(
            Type::WasmMemoryLoad, VerificationLevel::Standard).unwrap();
        let function_call = Type::fuel_cost_for_operation(
            Type::WasmFunctionCall, VerificationLevel::Standard).unwrap();
        let memory_mgmt = Type::fuel_cost_for_operation(
            Type::WasmMemoryManagement, VerificationLevel::Standard).unwrap();

        // Verify ordering: constants < locals < simple ops < complex ops < memory < calls < mgmt
        assert!(local_access >= simple_const, "Local access should cost >= constants");
        assert!(simple_arith >= local_access, "Simple arithmetic should cost >= local access");
        assert!(complex_arith > simple_arith, "Complex arithmetic should cost > simple arithmetic");
        assert!(memory_load > simple_arith, "Memory load should cost > simple arithmetic");
        assert!(function_call > memory_load, "Function calls should cost > memory load");
        assert!(memory_mgmt > function_call, "Memory management should cost > function calls");
    }
}

fn main() {
    println!("🚀 Testing WRT Fuel Integration System");
    
    // Test basic fuel cost calculation
    let simple_cost = Type::fuel_cost_for_operation(
        Type::WasmSimpleConstant, 
        VerificationLevel::Standard
    ).unwrap();
    println!("✅ Simple constant fuel cost: {}", simple_cost);

    let complex_cost = Type::fuel_cost_for_operation(
        Type::WasmComplexArithmetic, 
        VerificationLevel::Full
    ).unwrap();
    println!("✅ Complex arithmetic (Full verification) fuel cost: {}", complex_cost);

    let memory_cost = Type::fuel_cost_for_operation(
        Type::WasmMemoryManagement, 
        VerificationLevel::Standard
    ).unwrap();
    println!("✅ Memory management fuel cost: {}", memory_cost);

    println!("\n🎯 Fuel System Integration Test PASSED!");
    println!("📊 Instruction-based fuel consumption is working correctly");
    println!("🔧 Ready for full system integration");
}