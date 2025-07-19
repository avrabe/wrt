#![cfg(test)]

use wrt_decoder::component::parse_core_instance_section;
use wrt_error::Result;

#[test]
fn test_core_instance_with_multiple_arguments() -> Result<()> {
    // Mock binary data for a core instance section with multiple arguments
    // Format: tag(0x00) | module_idx(0) | arg_count(3) | 
    //         arg1_name_len(3) | arg1_name("env") | arg1_kind(0x12) | arg1_idx(4) |
    //         arg2_name_len(5) | arg2_name("wasi1") | arg2_kind(0x12) | arg2_idx(5) |
    //         arg3_name_len(5) | arg3_name("wasi2") | arg3_kind(0x12) | arg3_idx(6)
    
    let bytes = vec![
        // Section count (1 instance)
        0x01,
        
        // Instance 1: tag 0x00 (instantiate)
        0x00,
        
        // Module index 0
        0x00,
        
        // Argument count (3)
        0x03,
        
        // Argument 1: name="env", kind=0x12 (instance), idx=4
        0x03,                   // name length (3)
        b'e', b'n', b'v',       // name "env"
        0x12,                   // kind (instance)
        0x04,                   // instance index 4
        
        // Argument 2: name="wasi1", kind=0x12 (instance), idx=5
        0x05,                   // name length (5)
        b'w', b'a', b's', b'i', b'1', // name "wasi1"
        0x12,                   // kind (instance)
        0x05,                   // instance index 5
        
        // Argument 3: name="wasi2", kind=0x12 (instance), idx=6  
        0x05,                   // name length (5)
        b'w', b'a', b's', b'i', b'2', // name "wasi2"
        0x12,                   // kind (instance)
        0x06,                   // instance index 6
    ];
    
    // Parse the section
    let (instances, bytes_read) = parse_core_instance_section(&bytes)?;
    
    // Verify section was parsed correctly
    assert_eq!(bytes_read, bytes.len);
    assert_eq!(instances.len(), 1;
    
    // Check the argument parsing
    use wrt_format::component::CoreInstanceExpr;
    if let CoreInstanceExpr::Instantiate { module_idx, args } = &instances[0].instance_expr {
        assert_eq!(*module_idx, 0;
        assert_eq!(args.len(), 3;
        
        // Check first argument
        assert_eq!(args[0].name, "env";
        assert_eq!(args[0].instance_idx, 4;
        
        // Check second argument 
        assert_eq!(args[1].name, "wasi1";
        assert_eq!(args[1].instance_idx, 5;
        
        // Check third argument
        assert_eq!(args[2].name, "wasi2";
        assert_eq!(args[2].instance_idx, 6;
    } else {
        panic!("Expected Instantiate variant";
    }
    
    Ok(())
} 