//! Tests for the stackless execution engine
//!
//! This module tests the core functionality of the stackless engine,
//! including memory operations and type conversions.

#[cfg(test)]
mod tests {
    use crate::stackless::engine::StacklessEngine;
    use wrt_foundation::Value;
    
    #[test]
    fn test_engine_creation() {
        let engine = StacklessEngine::new();
        assert_eq!(engine.remaining_fuel(), None;
        assert_eq!(engine.stats().instructions_executed, 0);
    }
    
    #[test]
    fn test_engine_fuel_management() {
        let mut engine = StacklessEngine::new();
        
        // Set fuel
        engine.set_fuel(Some(1000;
        assert_eq!(engine.remaining_fuel(), Some(1000;
        
        // Consume fuel
        let result = engine.consume_fuel(wrt_foundation::operations::Type::BoundedVecPush;
        assert!(result.is_ok());
        
        // Fuel should be reduced
        assert!(engine.remaining_fuel().unwrap() < 1000);
    }
    
    #[test]
    fn test_stack_operations() {
        let mut engine = StacklessEngine::new();
        
        // Push values
        assert!(engine.push_control_value(Value::I32(42)).is_ok());
        assert!(engine.push_control_value(Value::I64(100)).is_ok());
        
        // Pop values
        let val = engine.pop_control_value().unwrap();
        assert_eq!(val, Value::I64(100;
        
        let val = engine.pop_control_value().unwrap();
        assert_eq!(val, Value::I32(42;
    }
    
    #[test]
    fn test_stack_underflow() {
        let mut engine = StacklessEngine::new();
        
        // Pop from empty stack should error
        assert!(engine.pop_control_value().is_err();
    }
    
    #[test]
    fn test_local_variables() {
        let mut engine = StacklessEngine::new();
        
        // Initialize locals
        let locals_count = 5;
        assert!(engine.init_locals(locals_count).is_ok());
        
        // Set local
        assert!(engine.set_local(0, Value::I32(123)).is_ok());
        
        // Get local
        let val = engine.get_local(0).unwrap();
        assert_eq!(val, Value::I32(123;
    }
    
    #[test]
    fn test_local_out_of_bounds() {
        let mut engine = StacklessEngine::new();
        
        // Initialize with 3 locals
        assert!(engine.init_locals(3).is_ok());
        
        // Access out of bounds should error
        assert!(engine.get_local(5).is_err();
        assert!(engine.set_local(5, Value::I32(0)).is_err();
    }
    
    #[test]
    fn test_gas_metering() {
        let mut engine = StacklessEngine::new();
        engine.set_fuel(Some(100;
        
        // Consume fuel multiple times
        for _ in 0..5 {
            let result = engine.consume_fuel(wrt_foundation::operations::Type::BoundedVecPush;
            assert!(result.is_ok());
        }
        
        // Fuel should be reduced
        let remaining = engine.remaining_fuel().unwrap();
        assert!(remaining < 100);
        assert!(remaining > 0);
    }
    
    #[test]
    fn test_execution_stats() {
        let mut engine = StacklessEngine::new();
        
        // Push some values to trigger stats
        engine.push_control_value(Value::I32(1)).unwrap();
        engine.push_control_value(Value::I32(2)).unwrap();
        
        // Update stats manually (normally done by instruction execution)
        engine.stats.increment_instructions(2;
        engine.stats.update_stack_depth(2;
        
        // Check stats
        assert_eq!(engine.stats().instructions_executed, 2;
        assert_eq!(engine.stats().max_stack_depth, 2;
    }
}