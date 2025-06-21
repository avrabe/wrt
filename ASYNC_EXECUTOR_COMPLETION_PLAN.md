# Async Executor Completion Plan

## Current Status

### ✅ Completed Components
1. **Fuel Infrastructure** - All fuel tracking, enforcement, and debt/credit systems
2. **Async Task Management** - Task creation, scheduling, and state management
3. **ASIL Execution Modes** - Different execution policies for each ASIL level
4. **Resource Management** - Lifetime tracking, handle tables, and cleanup
5. **Stream & Future Support** - Async streams and future combinators
6. **Error Propagation** - Context-aware error handling

### 🔄 Partially Complete
1. **WebAssembly Execution** - Structure exists but using simulation
   - Location: `fuel_async_executor.rs:1651` - `execute_wasm_function_with_fuel()`
   - Currently simulates execution instead of calling real WebAssembly functions

### ❌ Missing Components
1. **Main Executor Run Loop** - Top-level executor that orchestrates everything
2. **Component Function Resolution** - Getting actual functions from ComponentInstance
3. **Yield Point Restoration** - Resuming execution from saved state
4. **Integration Tests** - End-to-end tests with real WebAssembly modules

## Implementation Plan

### Phase 3.1: Complete WebAssembly Execution Integration
```rust
// In execute_wasm_function_with_fuel()
// 1. Get function index from task's execution context
let func_idx = task.execution_context.current_function;

// 2. Get module instance from component
let module_instance = component_instance.get_module_instance()?;

// 3. Execute using StacklessEngine
let result = engine.execute_function(
    module_instance,
    func_idx,
    &task.execution_context.locals,
)?;

// 4. Update task state based on result
match result {
    ExecutionResult::Completed(values) => {
        ExecutionStepResult::Completed(serialize_values(values))
    },
    ExecutionResult::Yielded(yield_point) => {
        task.execution_context.save_yield_point(yield_point)?;
        ExecutionStepResult::Yielded
    },
    ExecutionResult::Waiting(resource) => {
        ExecutionStepResult::Waiting
    },
}
```

### Phase 3.2: Implement Main Executor
```rust
pub struct FuelAsyncRuntime {
    executor: FuelAsyncExecutor,
    component_registry: ComponentRegistry,
    global_fuel_budget: u64,
}

impl FuelAsyncRuntime {
    pub fn run(&mut self) -> Result<(), Error> {
        while self.executor.has_tasks() {
            // Poll ready tasks
            let polled = self.executor.poll_ready_tasks(100)?;
            
            // Check global fuel budget
            if self.executor.total_fuel_consumed() > self.global_fuel_budget {
                return Err(Error::new(
                    ErrorCategory::Resource,
                    codes::FUEL_EXHAUSTED,
                    "Global fuel budget exhausted",
                ));
            }
            
            // Yield to other system tasks if needed
            if polled == 0 {
                // No tasks ready, wait for wakers
                self.wait_for_wakers()?;
            }
        }
        
        Ok(())
    }
}
```

### Phase 3.3: Component Function Resolution
```rust
impl ComponentInstance {
    /// Get function for async execution
    pub fn get_async_function(
        &self,
        export_name: &str,
    ) -> Result<(ModuleInstanceId, FunctionIndex), Error> {
        // Resolve export to function
        let export = self.exports.get(export_name)?;
        
        match export {
            Export::Function { module_id, func_idx } => {
                Ok((*module_id, *func_idx))
            },
            _ => Err(Error::new(
                ErrorCategory::Type,
                codes::TYPE_MISMATCH,
                "Export is not a function",
            )),
        }
    }
}
```

### Phase 3.4: Yield Point Implementation
```rust
impl ExecutionContext {
    /// Save yield point for later restoration
    pub fn save_yield_point(&mut self, yield_point: YieldPoint) -> Result<()> {
        self.yield_points.push(yield_point)?;
        self.has_yielded = true;
        Ok(())
    }
    
    /// Restore from yield point
    pub fn restore_from_yield(&mut self) -> Result<Option<YieldPoint>> {
        if let Some(yield_point) = self.yield_points.pop() {
            // Restore instruction pointer
            self.instruction_pointer = yield_point.ip;
            
            // Restore stack
            self.stack = yield_point.stack;
            
            // Restore locals
            self.locals = yield_point.locals;
            
            Ok(Some(yield_point))
        } else {
            Ok(None)
        }
    }
}
```

### Phase 3.5: Integration Tests
```rust
#[cfg(test)]
mod integration_tests {
    use super::*;
    
    #[test]
    fn test_async_wasm_execution() {
        // Create runtime
        let mut runtime = FuelAsyncRuntime::new(10000);
        
        // Load test component with async function
        let component = load_test_component("async_test.wasm")?;
        runtime.register_component(component)?;
        
        // Spawn async task
        let task_id = runtime.spawn_task(
            "test_component",
            "async_function",
            vec![Value::I32(42)],
            1000, // fuel budget
            VerificationLevel::Basic,
        )?;
        
        // Run until completion
        runtime.run()?;
        
        // Verify result
        let result = runtime.get_task_result(task_id)?;
        assert_eq!(result, vec![Value::I32(84)]);
    }
}
```

## Priority Order

1. **High Priority**: Complete WebAssembly execution (Phase 3.1)
   - Most critical for making the executor functional
   - Enables real async WebAssembly execution

2. **Medium Priority**: Main executor loop (Phase 3.2)
   - Needed for practical usage
   - Orchestrates the entire async system

3. **Medium Priority**: Component resolution (Phase 3.3)
   - Required for executing specific functions
   - Bridges component model to execution

4. **Low Priority**: Yield points (Phase 3.4)
   - Advanced feature for suspending/resuming
   - Can work without it initially

5. **Low Priority**: Integration tests (Phase 3.5)
   - Important for validation
   - Can be added incrementally

## Next Steps

The most impactful next step would be completing the WebAssembly execution integration (Phase 3.1), which would make the async executor actually functional with real WebAssembly code rather than simulations.