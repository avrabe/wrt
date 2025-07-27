//! Simple WASI-NN inference example module
//!
//! This module demonstrates how to use WASI-NN from WebAssembly

#![no_std]
#![no_main]

use core::panic::PanicInfo;

// WASI-NN imports
#[link(wasm_import_module = "wasi:nn/inference")]
extern "C" {
    #[link_name = "load"]
    fn nn_load(
        data_ptr: *const u8,
        data_len: u32,
        encoding: u8,
        target: u8,
        graph_ptr: *mut u32,
    ) -> u32;
    
    #[link_name = "init-execution-context"]
    fn nn_init_execution_context(graph: u32, context_ptr: *mut u32) -> u32;
    
    #[link_name = "set-input"]
    fn nn_set_input(
        context: u32,
        index: u32,
        tensor_ptr: *const u8,
        tensor_len: u32,
        dims_ptr: *const u32,
        dims_len: u32,
        tensor_type: u8,
    ) -> u32;
    
    #[link_name = "compute"]
    fn nn_compute(context: u32) -> u32;
    
    #[link_name = "get-output"]
    fn nn_get_output(
        context: u32,
        index: u32,
        tensor_ptr: *mut u8,
        tensor_len: u32,
        dims_ptr: *mut u32,
        dims_len: u32,
        type_ptr: *mut u8,
    ) -> u32;
}

// Error codes
const SUCCESS: u32 = 0;
const ERROR_INVALID_ARGUMENT: u32 = 1;

// Model encodings
const ENCODING_ONNX: u8 = 0;

// Execution targets
const TARGET_CPU: u8 = 0;

// Tensor types
const TENSOR_TYPE_F32: u8 = 1;

// Static buffers for simplicity
static mut MODEL_DATA: [u8; 1024] = [0; 1024];
static mut INPUT_TENSOR: [f32; 4] = [1.0, 2.0, 3.0, 4.0];
static mut OUTPUT_TENSOR: [f32; 2] = [0.0; 2];
static mut INPUT_DIMS: [u32; 2] = [2, 2];
static mut OUTPUT_DIMS: [u32; 2] = [1, 2];

/// Main entry point for the WASM module
#[no_mangle]
pub extern "C" fn run_inference() -> u32 {
    unsafe {
        // 1. Load a model (using dummy data for example)
        let mut graph_id: u32 = 0;
        let load_result = nn_load(
            MODEL_DATA.as_ptr(),
            MODEL_DATA.len() as u32,
            ENCODING_ONNX,
            TARGET_CPU,
            &mut graph_id,
        ;
        
        if load_result != SUCCESS {
            return load_result;
        }
        
        // 2. Create execution context
        let mut context_id: u32 = 0;
        let init_result = nn_init_execution_context(graph_id, &mut context_id;
        
        if init_result != SUCCESS {
            return init_result;
        }
        
        // 3. Set input tensor
        let set_input_result = nn_set_input(
            context_id,
            0, // input index
            INPUT_TENSOR.as_ptr() as *const u8,
            (INPUT_TENSOR.len() * 4) as u32, // 4 bytes per f32
            INPUT_DIMS.as_ptr(),
            INPUT_DIMS.len() as u32,
            TENSOR_TYPE_F32,
        ;
        
        if set_input_result != SUCCESS {
            return set_input_result;
        }
        
        // 4. Run inference
        let compute_result = nn_compute(context_id;
        
        if compute_result != SUCCESS {
            return compute_result;
        }
        
        // 5. Get output
        let mut output_type: u8 = 0;
        let get_output_result = nn_get_output(
            context_id,
            0, // output index
            OUTPUT_TENSOR.as_mut_ptr() as *mut u8,
            (OUTPUT_TENSOR.len() * 4) as u32,
            OUTPUT_DIMS.as_mut_ptr(),
            OUTPUT_DIMS.len() as u32,
            &mut output_type,
        ;
        
        if get_output_result != SUCCESS {
            return get_output_result;
        }
        
        // Success!
        SUCCESS
    }
}

/// Memory allocation functions required by WASM
#[no_mangle]
pub extern "C" fn malloc(size: usize) -> *mut u8 {
    // Simple bump allocator for demo
    static mut HEAP: [u8; 65536] = [0; 65536];
    static mut HEAP_POS: usize = 0;
    
    unsafe {
        let ptr = HEAP.as_mut_ptr().add(HEAP_POS;
        HEAP_POS += size;
        ptr
    }
}

#[no_mangle]
pub extern "C" fn free(_ptr: *mut u8) {
    // No-op for simple demo
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    // In WASM, we can't do much on panic
    loop {}
}