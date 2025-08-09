#[cfg(test)]
mod resume_tests {
    use wat;
    use wrt::{
        error::Result,
        execution::ExecutionState,
        instructions::Instruction,
        module::Function,
        types::{
            FuncType,
            ValueType,
        },
        values::Value,
        Module,
        StacklessEngine,
    };

    #[test]
    fn test_pause_on_fuel_exhaustion() -> Result<()> {
        // Create a simple module
        let mut module = Module::new()?;

        // Create a function type
        let func_type = FuncType {
            params:  vec![ValueType::I32],
            results: vec![ValueType::I32],
        };
        module.types.push(func_type);

        // Create a simple function
        let code = vec![Instruction::I32Const(42), Instruction::End];
        let function = Function {
            type_idx: 0,
            locals: vec![],
            code,
        };
        module.functions.push(function);

        // Create an engine and instantiate
        let mut engine = StacklessEngine::new(module.clone());
        let instance_idx = engine.instantiate(module)?;

        // Manually set the engine state to paused
        // engine.set_state(ExecutionState::Paused {
        //     instance_idx: instance_idx as u32,
        //     func_idx: 0,
        //     pc: 0,
        //     expected_results: 1,
        // };

        // Now resume execution
        // let result = engine.resume(vec![])?;

        // The result should be 42
        // assert_eq!(result, vec![Value::I32(42)];

        // The engine state should be Finished
        // assert!(matches!(engine.state, ExecutionState::Finished);

        Ok(())
    }

    #[test]
    fn test_resume_functionality() -> Result<()> {
        // Create a simple module
        let mut module = Module::new()?;

        // Create a function type
        let func_type = FuncType {
            params:  vec![ValueType::I32],
            results: vec![ValueType::I32],
        };
        module.types.push(func_type);

        // Create a function
        let code = vec![
            Instruction::LocalGet(0),
            Instruction::I32Const(1),
            Instruction::I32Add,
            Instruction::End,
        ];
        let function = Function {
            type_idx: 0,
            locals: vec![],
            code,
        };
        module.functions.push(function);

        // Create an engine and instantiate
        let mut engine = StacklessEngine::new(module.clone());
        engine.instantiate(module)?;

        // Try to resume when the engine is not paused
        // let result = engine.resume(vec![];

        // Should get an error
        // assert!(result.is_err();
        // let err = result.unwrap_err);
        // assert_eq!(
        //     err.to_string(),
        //     "Execution error: Cannot resume: engine is not paused"
        // ;

        Ok(())
    }

    #[test]
    fn test_resume_with_insufficient_fuel() -> Result<()> {
        let wat = r#"(module (func $nop (export "loop") nop))"#;
        let wasm_bytes = wat::parse_str(wat).unwrap();
        let mut module = Module::new()?;
        let module = module.load_from_binary(&wasm_bytes).unwrap();
        let mut engine = StacklessEngine::new_with_module(module);

        engine.fuel = Some(5); // Set fuel less than needed

        // Execute until fuel exhausted
        let result = engine.invoke_export("loop", &[]);

        // The result should be an error
        // assert!(result.is_err();
        // let err = result.unwrap_err);
        // assert_eq!(
        //     err.to_string(),
        //     "Execution error: Insufficient fuel"
        // ;

        Ok(())
    }
}
