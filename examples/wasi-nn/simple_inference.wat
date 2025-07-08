;; Simple WASI-NN inference example in WAT format
;; This demonstrates loading a model and running inference

(module
  ;; Import WASI-NN functions
  (import "wasi:nn/inference" "load" 
    (func $nn_load (param i32 i32 i32 i32 i32) (result i32)))
  (import "wasi:nn/inference" "init-execution-context" 
    (func $nn_init_context (param i32) (result i32)))
  (import "wasi:nn/inference" "set-input" 
    (func $nn_set_input (param i32 i32 i32 i32 i32 i32 i32) (result i32)))
  (import "wasi:nn/inference" "compute" 
    (func $nn_compute (param i32) (result i32)))
  (import "wasi:nn/inference" "get-output" 
    (func $nn_get_output (param i32 i32 i32 i32 i32) (result i32)))
  
  ;; Memory for data
  (memory (export "memory") 1)
  
  ;; Model data (placeholder - would be actual ONNX model)
  (data (i32.const 0) "\00\01\02\03\04\05\06\07")
  
  ;; Input tensor data (placeholder)
  (data (i32.const 1024) "\00\00\00\00")
  
  ;; Main inference function
  (func (export "run_inference") (result i32)
    (local $graph i32)
    (local $context i32)
    (local $result i32)
    
    ;; Load model (data_ptr=0, data_len=8, encoding=0 (ONNX), target=0 (CPU))
    (call $nn_load
      (i32.const 0)    ;; data pointer
      (i32.const 8)    ;; data length
      (i32.const 0)    ;; encoding (ONNX)
      (i32.const 0)    ;; target (CPU)
      (local.get $graph))
    (local.set $result)
    
    ;; Check if load succeeded
    (if (i32.ne (local.get $result) (i32.const 0))
      (then (return (local.get $result))))
    
    ;; Initialize execution context
    (call $nn_init_context
      (local.get $graph))
    (local.set $context)
    
    ;; Set input tensor
    ;; context, index=0, data_ptr, data_len, dims_ptr, dims_len, type=1 (f32)
    (call $nn_set_input
      (local.get $context)
      (i32.const 0)       ;; input index
      (i32.const 1024)    ;; tensor data pointer
      (i32.const 16)      ;; tensor data length (4 floats)
      (i32.const 2048)    ;; dimensions pointer
      (i32.const 2)       ;; dimensions count
      (i32.const 1))      ;; tensor type (f32)
    (local.set $result)
    
    ;; Run inference
    (call $nn_compute
      (local.get $context))
    (local.set $result)
    
    ;; Get output
    (call $nn_get_output
      (local.get $context)
      (i32.const 0)       ;; output index
      (i32.const 3072)    ;; output buffer pointer
      (i32.const 4096)    ;; dims buffer pointer
      (i32.const 4100))   ;; type buffer pointer
    (local.set $result)
    
    ;; Return result
    (local.get $result)
  )
  
  ;; Dimensions data [2, 2] for 2x2 tensor
  (data (i32.const 2048) "\02\00\00\00\02\00\00\00")
)