// Copyright (c) 2025 Ralf Anton Beier
// Licensed under the MIT license.
// SPDX-License-Identifier: MIT

//! Example demonstrating DWARF debug information parsing
//!
//! This example shows how to use the wrt-debug crate in a no_std environment.
//! For demonstration purposes, we simulate a WebAssembly module with debug
//! sections.

#![no_std]
#![no_main]

use wrt_debug::prelude::*;

// Example module bytes (in practice, this would be a real WASM module)
const MODULE_BYTES: &[u8] = &[
    // WASM magic and version
    0x00, 0x61, 0x73, 0x6D, // \0asm
    0x01, 0x00, 0x00,
    0x00, /* version 1
           * ... rest of module with debug sections */
];

#[no_mangle]
pub extern "C" fn example_debug_info() {
    // Create debug info parser
    let mut debug_info = DwarfDebugInfo::new(MODULE_BYTES);

    // Register debug sections (in practice, these offsets would come from parsing)
    debug_info.add_section(".debug_line", 0x1000, 0x500);
    debug_info.add_section(".debug_info", 0x1500, 0x800);
    debug_info.add_section(".debug_abbrev", 0x1D00, 0x200);

    // Check if debug info is available
    if debug_info.has_debug_info() {
        // Try to find line info for a specific instruction
        match debug_info.find_line_info(0x42) {
            Ok(Some(line_info)) => {
                // In a real embedded system, you might log this or send it over UART
                // For now, we just demonstrate the API usage
                let _file = line_info.file_index;
                let _line = line_info.line;
                let _column = line_info.column;
            },
            Ok(None) => {
                // No line information found
            },
            Err(_e) => {
                // Error finding line info
            },
        }
    }
}

// Panic handler removed - provided by wrt-platform crate
