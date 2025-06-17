//! WebAssembly module parser using wrt-decoder
//!
//! This module provides functionality to parse WebAssembly modules
//! using the project's own wrt-decoder implementation.

use wrt_error::kinds::DecodingError;

#[cfg(feature = "std")]
use std::collections::HashSet;
#[cfg(not(feature = "std"))]
use alloc::collections::BTreeSet as HashSet;

use crate::{prelude::*, builtins::BuiltinType};

/// Scan a WebAssembly module for built-in imports
///
/// # Arguments
///
/// * `binary` - The WebAssembly module binary
///
/// # Returns
///
/// A Result containing a vector of built-in names found in the import section
pub fn scan_for_builtins(binary: &[u8]) -> Result<Vec<String>> {
    use wrt_decoder::sections::parsers::parse_import_section;
    use wrt_format::binary;
    
    // Validate WebAssembly magic number and version
    if binary.len() < 8 {
        return Err(Error::new(
            ErrorCategory::Parse,
            wrt_error::codes::PARSE_ERROR,
            "Binary too short to be a valid WebAssembly module"
        ));
    }
    
    // Check magic number
    if &binary[0..4] != b"\0asm" {
        return Err(Error::new(
            ErrorCategory::Parse,
            wrt_error::codes::PARSE_ERROR,
            "Invalid WebAssembly magic number"
        ));
    }
    
    let mut builtin_names = Vec::new();
    let mut offset = 8; // Skip magic number and version
    
    // Parse sections to find the import section
    while offset < binary.len() {
        // Read section ID
        let section_id = binary[offset];
        offset += 1;
        
        // Read section size
        let (section_size, new_offset) = binary::read_leb128_u32(binary, offset)
            .map_err(|e| Error::new(
                ErrorCategory::Parse,
                wrt_error::codes::PARSE_ERROR,
                &format!("Failed to read section size: {}", e)
            ))?;
        offset = new_offset;
        
        let section_end = offset + section_size as usize;
        
        // Check if this is the import section (ID = 2)
        if section_id == 2 {
            // Parse imports
            let section_data = &binary[offset..section_end];
            let imports = parse_import_section(section_data)
                .map_err(|e| Error::new(
                    ErrorCategory::Parse,
                    wrt_error::codes::PARSE_ERROR,
                    &format!("Failed to parse import section: {}", e)
                ))?;
            
            // Look for wasi_builtin imports
            for import in imports {
                if import.module == "wasi_builtin" {
                    builtin_names.push(import.name.clone());
                }
            }
            
            break; // No need to continue after import section
        }
        
        // Skip to next section
        offset = section_end;
    }
    
    Ok(builtin_names)
}

/// Scan a WebAssembly binary for built-in imports and map them to built-in
/// types
///
/// # Arguments
///
/// * `binary` - The WebAssembly module binary
///
/// # Returns
///
/// A Result containing a set of required built-in types
pub fn get_required_builtins(binary: &[u8]) -> Result<HashSet<BuiltinType>> {
    let builtin_names = scan_for_builtins(binary)?;
    let mut required_builtins = HashSet::new();

    for name in builtin_names {
        if let Some(builtin_type) = map_import_to_builtin(&name) {
            required_builtins.insert(builtin_type);
        }
    }

    Ok(required_builtins)
}

/// Map an import name to a built-in type
///
/// # Arguments
///
/// * `import_name` - The name of the import function
///
/// # Returns
///
/// An Option containing the corresponding built-in type if recognized
pub fn map_import_to_builtin(import_name: &str) -> Option<BuiltinType> {
    match import_name {
        // Generic resource operations
        "resource.create" => Some(BuiltinType::ResourceCreate),
        "resource.drop" => Some(BuiltinType::ResourceDrop),
        "resource.rep" => Some(BuiltinType::ResourceRep),
        "resource.get" => Some(BuiltinType::ResourceGet),

        // Feature-gated async operations
        #[cfg(feature = "component-model-async")]
        "async.new" => Some(BuiltinType::AsyncNew),
        #[cfg(feature = "component-model-async")]
        "async.get" => Some(BuiltinType::AsyncGet),
        #[cfg(feature = "component-model-async")]
        "async.poll" => Some(BuiltinType::AsyncPoll),
        #[cfg(feature = "component-model-async")]
        "async.wait" => Some(BuiltinType::AsyncWait),

        // Feature-gated error context operations
        #[cfg(feature = "component-model-error-context")]
        "error.new" => Some(BuiltinType::ErrorNew),
        #[cfg(feature = "component-model-error-context")]
        "error.trace" => Some(BuiltinType::ErrorTrace),

        // Feature-gated threading operations
        #[cfg(feature = "component-model-threading")]
        "threading.spawn" => Some(BuiltinType::ThreadingSpawn),
        #[cfg(feature = "component-model-threading")]
        "threading.join" => Some(BuiltinType::ThreadingJoin),
        #[cfg(feature = "component-model-threading")]
        "threading.sync" => Some(BuiltinType::ThreadingSync),

        // Unknown import name (including "random_get_bytes" which is handled separately)
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Helper to create a minimal test module with an import
    fn create_test_module(module_name: &str, import_name: &str) -> Vec<u8> {
        // WebAssembly module header
        let mut module = vec![0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00];

        // Type section (empty)
        module.extend_from_slice(&[0x01, 0x04, 0x01, 0x60, 0x00, 0x00]);

        // Import section with one import
        let module_name_len = module_name.len() as u8;
        let import_name_len = import_name.len() as u8;

        // Import section header
        module.push(0x02); // Import section ID
        module.push(0x07 + module_name_len + import_name_len); // Section size
        module.push(0x01); // Number of imports

        // Import entry
        module.push(module_name_len); // Module name length
        module.extend_from_slice(module_name.as_bytes()); // Module name
        module.push(import_name_len); // Import name length
        module.extend_from_slice(import_name.as_bytes()); // Import name
        module.push(0x00); // Import kind (function)
        module.push(0x00); // Type index

        module
    }

    #[test]
    fn test_scan_for_builtins() {
        // Create a test module with a wasi_builtin import for resource.create
        let module = create_test_module("wasi_builtin", "resource.create");

        // Test that we can find the built-in import
        let builtin_names = scan_for_builtins(&module).unwrap();
        assert_eq!(builtin_names.len(), 1);
        assert_eq!(builtin_names[0], "resource.create");

        // Test the mapping to built-in types
        let required_builtins = get_required_builtins(&module).unwrap();
        assert!(required_builtins.contains(&BuiltinType::ResourceCreate));
        assert_eq!(required_builtins.len(), 1);
    }

    #[test]
    fn test_random_builtin_import() {
        // Create a test module with a random_get_bytes import
        let module = create_test_module("wasi_builtin", "random_get_bytes");

        // We should be able to identify the import
        let builtin_names = scan_for_builtins(&module).unwrap();
        assert_eq!(builtin_names.len(), 1);
        assert_eq!(builtin_names[0], "random_get_bytes");

        // Since random_get_bytes is not in our map_import_to_builtin function,
        // we should see no builtin types when we call get_required_builtins
        let required_builtins = get_required_builtins(&module).unwrap();
        assert_eq!(required_builtins.len(), 0);
    }

    #[test]
    fn test_non_builtin_imports() {
        // Create a test module with an import that is not from wasi_builtin
        let module = create_test_module("other_module", "other_import");

        // We should not find any built-in imports
        let builtin_names = scan_for_builtins(&module).unwrap();
        assert_eq!(builtin_names.len(), 0);

        // No built-in types should be required
        let required_builtins = get_required_builtins(&module).unwrap();
        assert_eq!(required_builtins.len(), 0);
    }

    #[test]
    fn test_multiple_builtin_imports() {
        // Create test modules with different wasi_builtin imports
        let resource_create_module = create_test_module("wasi_builtin", "resource.create");
        let resource_drop_module = create_test_module("wasi_builtin", "resource.drop");
        let resource_rep_module = create_test_module("wasi_builtin", "resource.rep");
        let resource_get_module = create_test_module("wasi_builtin", "resource.get");

        // Verify all are correctly identified
        assert_eq!(scan_for_builtins(&resource_create_module).unwrap(), vec!["resource.create"]);
        assert_eq!(scan_for_builtins(&resource_drop_module).unwrap(), vec!["resource.drop"]);
        assert_eq!(scan_for_builtins(&resource_rep_module).unwrap(), vec!["resource.rep"]);
        assert_eq!(scan_for_builtins(&resource_get_module).unwrap(), vec!["resource.get"]);

        // Verify all map to correct builtin types
        assert!(get_required_builtins(&resource_create_module)
            .unwrap()
            .contains(&BuiltinType::ResourceCreate));
        assert!(get_required_builtins(&resource_drop_module)
            .unwrap()
            .contains(&BuiltinType::ResourceDrop));
        assert!(get_required_builtins(&resource_rep_module)
            .unwrap()
            .contains(&BuiltinType::ResourceRep));
        assert!(get_required_builtins(&resource_get_module)
            .unwrap()
            .contains(&BuiltinType::ResourceGet));
    }
}
