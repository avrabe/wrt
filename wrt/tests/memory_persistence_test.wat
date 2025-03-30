(module
  (memory (export "memory") 1)
  (global $stored_value (mut i32) (i32.const 0))
  
  (func $store (export "store")
    i32.const 42     ;; value to store
    global.set $stored_value
  
    ;; Also store in memory
    i32.const 100    ;; address
    i32.const 42     ;; value
    i32.store)       ;; store in memory
  
  (func $load (export "load") (result i32)
    ;; Load from both global and memory to verify they match
    global.get $stored_value  ;; load from global
    
    i32.const 100            ;; address
    i32.load                 ;; load from memory
    
    ;; They should be equal - if not, return 1 (failure)
    i32.ne
    (if (result i32)
      (then i32.const 1) ;; failure - values don't match
      (else global.get $stored_value) ;; success - return the value
    )
  )
  
  (func $run (export "run") (result i32)
    call $store
    call $load
    i32.const 42
    i32.ne
  )
) 