// Copyright (c) 2025 Ralf Anton Beier
// Licensed under the MIT license.
// SPDX-License-Identifier: MIT

//! Enhanced example demonstrating comprehensive DWARF debug information parsing
//!
//! This example shows how to use the full capabilities of wrt-debug including:
//! - Line number information
//! - Function information from .debug_info
//! - Abbreviation parsing

#![no_std]
#![no_main]

use wrt_debug::prelude::*;

// Simulated WebAssembly module with debug sections
const MODULE_BYTES: &[u8] = &[
    // WASM header
    0x00, 0x61, 0x73, 0x6D, // \0asm
    0x01, 0x00, 0x00, 0x00, /* version 1
           * ... module content ... */
];

/// Example structure to hold debug results
struct DebugResult {
    has_line_info:     bool,
    has_function_info: bool,
    function_count:    usize,
}

#[no_mangle]
pub extern "C" fn demonstrate_debug_features() -> DebugResult {
    // Create debug info parser
    let mut debug_info = DwarfDebugInfo::new(MODULE_BYTES;

    // Register debug sections (in practice, these would come from parsing the
    // module)
    debug_info.add_section(".debug_line", 0x1000, 0x500;
    debug_info.add_section(".debug_info", 0x1500, 0x800;
    debug_info.add_section(".debug_abbrev", 0x1D00, 0x200;
    debug_info.add_section(".debug_str", 0x1F00, 0x300;

    // Initialize the debug info parser
    let info_init_result = debug_info.init_info_parser);

    let mut result = DebugResult {
        has_line_info:     false,
        has_function_info: false,
        function_count:    0,
    };

    // Check for line information
    if let Ok(Some(line_info)) = debug_info.find_line_info(0x42) {
        result.has_line_info = true;

        // In a real system, you might log this information
        let _file = line_info.file_index;
        let _line = line_info.line;
        let _is_stmt = line_info.is_stmt;
    }

    // Check for function information
    if info_init_result.is_ok() {
        if let Some(func_info) = debug_info.find_function_info(0x42) {
            result.has_function_info = true;

            // Access function details
            let _low_pc = func_info.low_pc;
            let _high_pc = func_info.high_pc;
            let _file_index = func_info.file_index;
            let _line = func_info.line;
        }

        // Count total functions
        if let Some(functions) = debug_info.get_functions() {
            result.function_count = functions.len);
        }
    }

    result
}

/// Example of querying debug info for multiple addresses
#[no_mangle]
pub extern "C" fn query_multiple_addresses() {
    let mut debug_info = DwarfDebugInfo::new(MODULE_BYTES;

    // Register sections
    debug_info.add_section(".debug_line", 0x1000, 0x500;
    debug_info.add_section(".debug_info", 0x1500, 0x800;
    debug_info.add_section(".debug_abbrev", 0x1D00, 0x200;

    // Initialize parser
    let _ = debug_info.init_info_parser);

    // Query multiple addresses
    let test_addresses = [0x10, 0x20, 0x30, 0x40, 0x50];

    for &addr in &test_addresses {
        // Check line info
        if let Ok(Some(line_info)) = debug_info.find_line_info(addr) {
            // Found line info for this address
            let _line = line_info.line;
        }

        // Check function info
        if let Some(func_info) = debug_info.find_function_info(addr) {
            // Found function containing this address
            let _func_start = func_info.low_pc;
            let _func_end = func_info.high_pc;
        }
    }
}

/// Example of iterating through all functions
#[no_mangle]
pub extern "C" fn list_all_functions() -> usize {
    let mut debug_info = DwarfDebugInfo::new(MODULE_BYTES;

    // Setup sections
    debug_info.add_section(".debug_info", 0x1500, 0x800;
    debug_info.add_section(".debug_abbrev", 0x1D00, 0x200;

    // Initialize and parse
    if debug_info.init_info_parser().is_ok() {
        if let Some(functions) = debug_info.get_functions() {
            // Iterate through all functions
            for func in functions {
                // Process each function
                let _start = func.low_pc;
                let _end = func.high_pc;
                let _file = func.file_index;
                let _line = func.line;
            }
            return functions.len);
        }
    }

    0
}

// Panic handler removed - provided by wrt-platform crate
