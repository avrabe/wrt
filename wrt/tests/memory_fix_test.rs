use std::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use wat;
/// Test file to demonstrate memory persistence fix
use wrt::memory::DefaultMemory;
use wrt::module::{ExportKind, Module};
use wrt::types::MemoryType;
use wrt::{Error, Result, Value};

// Helper struct to simulate the memory fix
struct MemoryFixSimulator {
    module: Module,
    instance: Option<Instance>,
    memories: Vec<DefaultMemory>,
}

// Simple instance struct for our simulation
#[derive(Clone)]
struct Instance {
    module: Module,
    memories: Vec<DefaultMemory>,
}

impl Instance {
    fn new(module: Module) -> Self {
        Self {
            module,
            memories: Vec::new(),
        }
    }
}

impl MemoryFixSimulator {
    pub fn new(wat: &str) -> Result<Self> {
        // Parse the WAT string to a module
        let wasm = wat::parse_str(wat).map_err(|e| Error::Parse(e.to_string()))?;
        let module = Module::from_bytes(&wasm).map_err(|e| Error::Parse(e.to_string()))?;

        println!(
            "Module has {} memory definitions",
            module.memories.read().unwrap().len()
        );

        // Create memory instances based on module memory definitions
        let mut memories = Vec::new();
        for mem_arc in module.memories.read().unwrap().iter() {
            let mem_type = (**mem_arc).type_(); // Explicitly dereference Arc and then call type_()
            println!(
                "Creating memory with type: min={}, max={:?}",
                mem_type.min, mem_type.max
            );
            let memory = DefaultMemory::new(mem_type.clone());
            memories.push(memory);
        }

        // Create the simulator without an instance initially
        Ok(MemoryFixSimulator {
            module,
            instance: None,
            memories,
        })
    }

    pub fn instantiate(&mut self) -> Result<()> {
        // Create a new instance with proper memory
        let mut instance = Instance::new(self.module.clone());

        // Manually initialize the instance memories with our persistent memories
        instance.memories = self.memories.clone();

        println!("Created instance with {} memories", instance.memories.len());
        self.instance = Some(instance);
        Ok(())
    }

    pub fn call_function(&mut self, func_name: &str, _args: Vec<Value>) -> Result<Vec<Value>> {
        // First make sure we have an instance
        if self.instance.is_none() {
            return Err(Error::Execution("No instance available".to_string()));
        }

        // Important: We use our persistent memories initially instead of the instance's memories
        // This ensures we start with the correct memory state
        let mut instance_clone = self.instance.as_ref().unwrap().clone();

        // CRITICAL FIX: We need to use our persisted memories
        instance_clone.memories = self.memories.clone();

        // Find the function export
        let export = instance_clone
            .module
            .exports
            .iter()
            .find(|e| e.name == func_name)
            .ok_or_else(|| Error::Execution(format!("Function {} not found", func_name)))?;

        // Get function index
        let func_idx = match export.kind {
            ExportKind::Function => export.index,
            _ => {
                return Err(Error::Execution(format!(
                    "Export {} is not a function",
                    func_name
                )))
            }
        };

        // Debug: Log the memory state before execution
        println!("Executing function {} in instance 0", func_idx);

        if !instance_clone.memories.is_empty() {
            println!("Instance has {} memories", instance_clone.memories.len());

            // Print a section of memory around address 100
            let memory = &instance_clone.memories[0];
            let addr = 100u32;
            let start = addr.saturating_sub(4);
            let end = std::cmp::min(addr + 8, memory.data.read().unwrap().len() as u32);

            println!("Memory data around address {}:", addr);
            for i in start..end {
                println!("  [{:>3}]: {}", i, memory.data.read().unwrap()[i as usize]);
            }
        }

        // Get the function details
        let function = &instance_clone.module.functions[func_idx as usize];
        println!("Function has {} instructions", function.code.len());

        // Print function instructions for debugging
        for (i, instr) in function.code.iter().enumerate() {
            println!("Instruction {}: {:?}", i, instr);
        }

        // Get function type for debugging
        let func_type = &instance_clone.module.types[function.type_idx as usize];
        println!(
            "Function type: params={:?}, results={:?}",
            func_type.params, func_type.results
        );

        // Simulate function execution with a simple interpreter
        let mut results = Vec::new();
        let mut stack = Vec::new();

        for (idx, instruction) in function.code.iter().enumerate() {
            println!("Executing instruction {}: {:?}", idx, instruction);

            // Basic instruction handling
            match instruction {
                wrt::instructions::Instruction::I32Const(value) => {
                    stack.push(Value::I32(*value)); // Dereference value
                }
                wrt::instructions::Instruction::I32Store(_align, offset) => {
                    println!("I32STORE: Beginning execution of I32Store instruction");
                    if stack.len() < 2 {
                        return Err(Error::Execution("Stack underflow for I32Store".to_string()));
                    }

                    let value = stack.pop().unwrap();
                    let address = stack.pop().unwrap();

                    println!(
                        "I32STORE: Popped value {:?} and address {:?} from stack",
                        value, address
                    );

                    if instance_clone.memories.is_empty() {
                        return Err(Error::Execution(
                            "No memory available for store operation".to_string(),
                        ));
                    }

                    println!(
                        "I32STORE: Module has {} memories",
                        instance_clone.memories.len()
                    );

                    let memory = &mut instance_clone.memories[0];
                    println!(
                        "I32STORE: Memory size is {} bytes",
                        memory.data.read().unwrap().len()
                    );

                    let addr = match address {
                        Value::I32(addr) => addr as usize,
                        _ => return Err(Error::Execution("Address must be an i32".to_string())),
                    };

                    let offset_value = *offset as usize;

                    let effective_addr = addr + offset_value;
                    println!(
                        "I32STORE: Storing value {} at address {} (with offset {}, effective: {})",
                        match value {
                            Value::I32(v) => v,
                            _ => 0,
                        },
                        addr,
                        offset_value,
                        effective_addr
                    );

                    // Print memory before store
                    println!(
                        "I32STORE: Memory before store (address range {}..{}):",
                        effective_addr.saturating_sub(4),
                        effective_addr + 8
                    );
                    for i in effective_addr.saturating_sub(4)
                        ..std::cmp::min(effective_addr + 8, memory.data.read().unwrap().len())
                    {
                        println!("  [{}]: {}", i, memory.data.read().unwrap()[i]);
                    }

                    if effective_addr + 4 > memory.data.read().unwrap().len() {
                        return Err(Error::Execution("Memory access out of bounds".to_string()));
                    }

                    // Store the i32 value (little endian)
                    match value {
                        Value::I32(val) => {
                            let bytes = val.to_le_bytes();
                            println!(
                                "I32STORE: Writing bytes {:?} to memory at address {}",
                                bytes, effective_addr
                            );
                            let mut data = memory.data.write().unwrap();
                            data[effective_addr] = bytes[0];
                            data[effective_addr + 1] = bytes[1];
                            data[effective_addr + 2] = bytes[2];
                            data[effective_addr + 3] = bytes[3];
                        }
                        _ => {
                            return Err(Error::Execution(
                                "Value must be an i32 for I32Store".to_string(),
                            ))
                        }
                    }

                    // Print memory after store
                    println!(
                        "I32STORE: Memory after store (address range {}..{}):",
                        effective_addr.saturating_sub(4),
                        effective_addr + 8
                    );
                    for i in effective_addr.saturating_sub(4)
                        ..std::cmp::min(effective_addr + 8, memory.data.read().unwrap().len())
                    {
                        println!("  [{}]: {}", i, memory.data.read().unwrap()[i]);
                    }

                    println!(
                        "I32STORE: Successfully stored value {} at address {}",
                        match value {
                            Value::I32(v) => v,
                            _ => 0,
                        },
                        effective_addr
                    );
                }
                wrt::instructions::Instruction::I32Load(_align, offset) => {
                    println!("I32LOAD: Beginning execution of I32Load instruction");
                    if stack.is_empty() {
                        return Err(Error::Execution("Stack underflow for I32Load".to_string()));
                    }

                    let address = stack.pop().unwrap();
                    println!("I32LOAD: Popped address {:?} from stack", address);

                    if instance_clone.memories.is_empty() {
                        return Err(Error::Execution(
                            "No memory available for load operation".to_string(),
                        ));
                    }

                    println!(
                        "I32LOAD: Module has {} memories",
                        instance_clone.memories.len()
                    );

                    let memory = &instance_clone.memories[0];
                    println!(
                        "I32LOAD: Memory size is {} bytes",
                        memory.data.read().unwrap().len()
                    );

                    let addr = match address {
                        Value::I32(addr) => addr as usize,
                        _ => return Err(Error::Execution("Address must be an i32".to_string())),
                    };

                    let offset_value = *offset as usize;

                    let effective_addr = addr + offset_value;
                    println!(
                        "I32LOAD: Loading value from address {} (with offset {}, effective: {})",
                        addr, offset_value, effective_addr
                    );

                    // Print memory section
                    println!(
                        "I32LOAD: Memory data (address range {}..{}):",
                        effective_addr.saturating_sub(4),
                        effective_addr + 8
                    );
                    for i in effective_addr.saturating_sub(4)
                        ..std::cmp::min(effective_addr + 8, memory.data.read().unwrap().len())
                    {
                        println!("  [{}]: {}", i, memory.data.read().unwrap()[i]);
                    }

                    if effective_addr + 4 > memory.data.read().unwrap().len() {
                        return Err(Error::Execution("Memory access out of bounds".to_string()));
                    }

                    // Load the i32 value (little endian)
                    let data = memory.data.read().unwrap();
                    let mut bytes = [0u8; 4];
                    bytes[0] = data[effective_addr];
                    bytes[1] = data[effective_addr + 1];
                    bytes[2] = data[effective_addr + 2];
                    bytes[3] = data[effective_addr + 3];

                    let value = i32::from_le_bytes(bytes);
                    println!(
                        "I32LOAD: Read bytes {:?} from memory, parsed as i32 value {}",
                        bytes, value
                    );

                    stack.push(Value::I32(value));
                    println!("I32LOAD: Pushed result value I32({}) to stack", value);
                }
                wrt::instructions::Instruction::End => {
                    println!("End of function reached after {} instructions", idx + 1);
                    // For functions that return a value, we would extract it from the stack
                    if !func_type.results.is_empty()
                        && func_type.results[0] == wrt::types::ValueType::I32
                    {
                        if stack.is_empty() {
                            return Err(Error::Execution(
                                "Stack underflow when getting results".to_string(),
                            ));
                        }
                        results.push(stack.pop().unwrap());
                    }
                    break;
                }
                _ => {
                    return Err(Error::Execution(format!(
                        "Unsupported instruction: {:?}",
                        instruction
                    )))
                }
            }
        }

        println!(
            "Function completed successfully with results: {:?}",
            results
        );

        // *** This is the key part that simulates our fix: ***
        // Copy any memory changes back to our persistent memory
        for (i, memory) in instance_clone.memories.iter().enumerate() {
            if i < self.memories.len() {
                // Very important: copy back the modified memory to our persistent store
                println!("Copying back memory changes to our persistent store...");
                self.memories[i]
                    .data
                    .write()
                    .unwrap()
                    .copy_from_slice(&memory.data.read().unwrap());

                // Debug: verify memory was actually copied
                if i == 0 {
                    let addr = 100usize;
                    let value_bytes = [
                        self.memories[i].data.read().unwrap()[addr],
                        self.memories[i].data.read().unwrap()[addr + 1],
                        self.memories[i].data.read().unwrap()[addr + 2],
                        self.memories[i].data.read().unwrap()[addr + 3],
                    ];
                    let value = i32::from_le_bytes(value_bytes);
                    println!(
                        "After memory persistence fix: Value at address 100 = {}",
                        value
                    );
                }

                println!("Simulating memory persistence fix: Copied memory changes back to persistent store");
            }
        }

        Ok(results)
    }
}

#[test]
fn test_memory_fix_simulation() {
    // Define a simple WebAssembly module in WAT format
    let wat = r#"
    (module
      (memory (export "memory") 1)
      
      ;; Function to store value 42 at address 100
      (func (export "store")
        i32.const 100  ;; address
        i32.const 42   ;; value
        i32.store      ;; store 42 at address 100
      )
      
      ;; Function to load value from address 100
      (func (export "load") (result i32)
        i32.const 100  ;; address
        i32.load       ;; load value from address 100
      )
    )
    "#;

    // Create our memory fix simulator
    let mut simulator = MemoryFixSimulator::new(wat).unwrap();

    // Instantiate the module
    simulator.instantiate().unwrap();

    // Execute the store function
    let store_results = simulator.call_function("store", vec![]).unwrap();
    println!("Store executed with results: {:?}", store_results);

    // Execute the load function
    let load_results = simulator.call_function("load", vec![]).unwrap();
    println!("Load executed with results: {:?}", load_results);

    // Verify the value was persisted between function calls
    assert_eq!(load_results, vec![Value::I32(42)]);
    println!("TEST PASSED: Memory persistence verification successful!");
}
