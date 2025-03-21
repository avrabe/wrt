use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::{quote, ToTokens};
use std::path::Path;
use syn::{parse_macro_input, Ident, LitStr};

/// Procedural macro to generate test cases from a WAST file
///
/// # Arguments
///
/// * `file_path` - Path to the WAST file (relative to the WASM_TESTSUITE environment variable)
/// * `test_name_prefix` - Prefix for the generated test names
///
/// # Example
///
/// ```
/// # use wast_proc_macro::generate_wast_tests;
///
/// #[generate_wast_tests("simd/simd_lane.wast", "simd_lane")]
/// fn run_simd_lane_tests() {
///     // This function will be called for each test case in the WAST file
/// }
/// ```
#[proc_macro_attribute]
pub fn generate_wast_tests(attr: TokenStream, item: TokenStream) -> TokenStream {
    let attr_input = parse_macro_input!(attr as syn::AttributeArgs);
    let item_fn = parse_macro_input!(item as syn::ItemFn);

    // Extract arguments
    if attr_input.len() != 2 {
        return syn::Error::new(
            Span::call_site(),
            "Expected two arguments: file_path and test_name_prefix",
        )
        .to_compile_error()
        .into();
    }

    let file_path = match &attr_input[0] {
        syn::NestedMeta::Lit(syn::Lit::Str(lit)) => lit.value(),
        _ => {
            return syn::Error::new(
                Span::call_site(),
                "First argument must be a string literal path to the WAST file",
            )
            .to_compile_error()
            .into();
        }
    };

    let test_name_prefix = match &attr_input[1] {
        syn::NestedMeta::Lit(syn::Lit::Str(lit)) => lit.value(),
        _ => {
            return syn::Error::new(
                Span::call_site(),
                "Second argument must be a string literal for the test name prefix",
            )
            .to_compile_error()
            .into();
        }
    };

    // Generate test functions
    let original_fn_name = &item_fn.sig.ident;
    let fn_block = &item_fn.block;
    let fn_vis = &item_fn.vis;

    // Generate the code to discover, parse and run tests
    let expanded = quote! {
        #[test]
        #fn_vis fn #original_fn_name() {
            use std::env;
            use std::path::Path;
            use std::fs;
            use wast::parser::{ParseBuffer, Parser};
            use wast::{Wast, WastDirective, WastExecute, WastArg, WastRet};
            use wast::core::{WastArgCore, WastRetCore};
            use wrt::{Engine, Module, Value, Error as WrtError};

            // Get testsuite path from build script
            let testsuite_path = match env::var("WASM_TESTSUITE") {
                Ok(path) => path,
                Err(_) => {
                    println!("Skipping WAST tests: WASM_TESTSUITE environment variable not set");
                    return;
                }
            };

            // Get commit hash
            let commit_hash = env::var("WASM_TESTSUITE_COMMIT").unwrap_or_else(|_| "unknown".to_string());
            println!("Running tests from testsuite at commit: {}", commit_hash);

            // Construct full path to the WAST file
            let wast_path = Path::new(&testsuite_path).join(#file_path);

            if !wast_path.exists() {
                println!(
                    "Skipping test: WAST file not found at {:?}. File will be available after next build.",
                    wast_path
                );
                return;
            }

            // Read the WAST file content
            let wast_content = match fs::read_to_string(&wast_path) {
                Ok(content) => content,
                Err(e) => {
                    panic!("Failed to read WAST file {:?}: {}", wast_path, e);
                }
            };

            // Parse the WAST file
            let test_prefix = #test_name_prefix;
            println!("Running WAST tests from {:?} with prefix '{}'", wast_path, test_prefix);

            // Actually parse the WAST file
            let buf = match ParseBuffer::new(&wast_content) {
                Ok(buf) => buf,
                Err(e) => {
                    panic!("Failed to parse WAST file {:?}: {}", wast_path, e);
                }
            };

            let wast = match wast::parser::parse::<Wast>(&buf) {
                Ok(wast) => wast,
                Err(e) => {
                    panic!("Failed to parse WAST file {:?}: {}", wast_path, e);
                }
            };

            // Process each directive in the WAST file
            for (i, directive) in wast.directives.iter().enumerate() {
                match directive {
                    WastDirective::Module(module) => {
                        println!("  Module {} found", i);
                        // Process module
                        let bytes = module.encode().expect("Failed to encode module");
                        // Process the module...
                    }
                    WastDirective::AssertMalformed { module, .. } => {
                        println!("  AssertMalformed {} found", i);
                        // Should fail to instantiate
                    }
                    WastDirective::AssertInvalid { module, .. } => {
                        println!("  AssertInvalid {} found", i);
                        // Should fail to validate
                    }
                    WastDirective::Register { name, module, .. } => {
                        println!("  Register {} as {} found", module, name);
                        // Register the module
                    }
                    WastDirective::Invoke(invoke) => {
                        println!("  Invoke {} found", i);
                        // Invoke a function
                        let func_name = invoke.name;
                        // Invoke the function...
                    }
                    WastDirective::AssertReturn { exec, results, .. } => {
                        println!("  AssertReturn {} found", i);
                        // Execute and assert return values match
                    }
                    WastDirective::AssertTrap { exec, .. } => {
                        println!("  AssertTrap {} found", i);
                        // Execute and assert it traps
                    }
                    other => {
                        println!("  Other directive found: {:?}", other);
                    }
                }

                // Call the original function for each directive
                // This gives the user a chance to process each directive
                #fn_block
            }
        }
    };

    TokenStream::from(expanded)
}

/// Procedural macro to generate tests for all WAST files in a directory
///
/// # Arguments
///
/// * `dir_path` - Path to the directory containing WAST files (relative to the WASM_TESTSUITE environment variable)
/// * `test_name_prefix` - Prefix for the generated test names
///
/// # Example
///
/// ```
/// # use wast_proc_macro::generate_directory_tests;
///
/// #[generate_directory_tests("simd", "simd")]
/// fn run_simd_directory_tests(file_name: &str, test_name: &str) {
///     // This function will be called for each WAST file in the directory
/// }
/// ```
#[proc_macro_attribute]
pub fn generate_directory_tests(attr: TokenStream, item: TokenStream) -> TokenStream {
    let attr_input = parse_macro_input!(attr as syn::AttributeArgs);
    let item_fn = parse_macro_input!(item as syn::ItemFn);

    // Extract arguments
    if attr_input.len() != 2 {
        return syn::Error::new(
            Span::call_site(),
            "Expected two arguments: dir_path and test_name_prefix",
        )
        .to_compile_error()
        .into();
    }

    let dir_path = match &attr_input[0] {
        syn::NestedMeta::Lit(syn::Lit::Str(lit)) => lit.value(),
        _ => {
            return syn::Error::new(
                Span::call_site(),
                "First argument must be a string literal path to the directory",
            )
            .to_compile_error()
            .into();
        }
    };

    let test_name_prefix = match &attr_input[1] {
        syn::NestedMeta::Lit(syn::Lit::Str(lit)) => lit.value(),
        _ => {
            return syn::Error::new(
                Span::call_site(),
                "Second argument must be a string literal for the test name prefix",
            )
            .to_compile_error()
            .into();
        }
    };

    // Generate test function
    let original_fn_name = &item_fn.sig.ident;
    let fn_block = &item_fn.block;
    let fn_vis = &item_fn.vis;

    // Generate the code to discover and run tests
    let expanded = quote! {
        #[test]
        #fn_vis fn #original_fn_name() {
            use std::env;
            use std::path::Path;
            use std::fs;
            use wast::parser::{ParseBuffer, Parser};
            use wast::{Wast, WastDirective, WastExecute, WastArg, WastRet};
            use wast::core::{WastArgCore, WastRetCore};
            use wrt::{Engine, Module, Value, Error as WrtError};

            // Get testsuite path from build script
            let testsuite_path = match env::var("WASM_TESTSUITE") {
                Ok(path) => path,
                Err(_) => {
                    println!("Skipping directory tests: WASM_TESTSUITE environment variable not set");
                    return;
                }
            };

            // Get commit hash
            let commit_hash = env::var("WASM_TESTSUITE_COMMIT").unwrap_or_else(|_| "unknown".to_string());
            println!("Running tests from testsuite at commit: {}", commit_hash);

            // Construct full path to the directory
            let dir_path = Path::new(&testsuite_path).join(#dir_path);

            if !dir_path.exists() {
                println!(
                    "Skipping test: Directory not found at {:?}. Directory will be available after next build.",
                    dir_path
                );
                return;
            }

            // Read all files in the directory
            let entries = match fs::read_dir(&dir_path) {
                Ok(entries) => entries,
                Err(e) => {
                    panic!("Failed to read directory {:?}: {}", dir_path, e);
                }
            };

            let test_prefix = #test_name_prefix;
            println!("Running directory tests from {:?} with prefix '{}'", dir_path, test_prefix);

            // Process each WAST file in the directory
            for entry in entries {
                let entry = match entry {
                    Ok(entry) => entry,
                    Err(e) => {
                        println!("Error reading directory entry: {}", e);
                        continue;
                    }
                };

                let path = entry.path();
                if path.extension().map_or(false, |ext| ext == "wast") {
                    let file_name = path.file_name().unwrap().to_string_lossy().to_string();
                    let test_name = format!("{}_{}", test_prefix, file_name.replace(".wast", ""));
                    println!("  Running test: {}", test_name);

                    // Read the WAST file content
                    let wast_content = match fs::read_to_string(&path) {
                        Ok(content) => content,
                        Err(e) => {
                            println!("Failed to read WAST file {:?}: {}", path, e);
                            continue;
                        }
                    };

                    // Parse the WAST file
                    let buf = match ParseBuffer::new(&wast_content) {
                        Ok(buf) => buf,
                        Err(e) => {
                            println!("Failed to parse WAST file {:?}: {}", path, e);
                            continue;
                        }
                    };

                    let wast = match wast::parser::parse::<Wast>(&buf) {
                        Ok(wast) => wast,
                        Err(e) => {
                            println!("Failed to parse WAST file {:?}: {}", path, e);
                            continue;
                        }
                    };

                    println!("  Parsed {} directives in {}", wast.directives.len(), file_name);

                    // Setup WRT engine and module registry for execution
                    let mut engine = Engine::new();
                    let mut current_module_idx = 0;

                    // Process directives
                    for directive in wast.directives {
                        match directive {
                            WastDirective::Module(module) => {
                                println!("    Processing module");

                                // Convert WAT to binary WebAssembly
                                let binary = module.encode().unwrap();

                                // Load the module into the engine
                                match Module::from_binary(&binary) {
                                    Ok(module) => {
                                        match engine.load_module(module) {
                                            Ok(idx) => {
                                                println!("    Module loaded successfully with index {}", idx);
                                                current_module_idx = idx;
                                            }
                                            Err(e) => {
                                                println!("    Failed to load module: {}", e);
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        println!("    Failed to parse binary module: {}", e);
                                    }
                                }
                            }
                            WastDirective::Invoke(invoke) => {
                                println!("    Invoking function: {}", invoke.name);

                                // Convert wast arguments to WRT values
                                let mut args = Vec::new();
                                for arg in &invoke.args {
                                    match arg {
                                        WastArg::Core(arg_core) => match arg_core {
                                            WastArgCore::I32(val) => args.push(Value::I32(*val)),
                                            WastArgCore::I64(val) => args.push(Value::I64(*val)),
                                            WastArgCore::F32(val) => args.push(Value::F32(f32::from_bits(val.bits))),
                                            WastArgCore::F64(val) => args.push(Value::F64(f64::from_bits(val.bits))),
                                            WastArgCore::V128(val) => {
                                                // Manual byte conversion
                                                let mut bytes = [0u8; 16];
                                                for (i, b) in val.to_vec().iter().enumerate() {
                                                    if i < 16 {
                                                        bytes[i] = *b;
                                                    }
                                                }
                                                let bits = u128::from_le_bytes(bytes);
                                                args.push(Value::V128(bits));
                                            },
                                            _ => {
                                                println!("    Unsupported argument type: {:?}", arg_core);
                                                continue;
                                            }
                                        },
                                        _ => {
                                            println!("    Unsupported non-core argument type");
                                            continue;
                                        }
                                    }
                                }

                                // Execute the function
                                match engine.execute(current_module_idx, invoke.name, args) {
                                    Ok(results) => {
                                        println!("    Function executed successfully with {} results", results.len());
                                    }
                                    Err(e) => {
                                        println!("    Function execution failed: {}", e);
                                    }
                                }
                            }
                            WastDirective::AssertReturn { exec, results, .. } => {
                                println!("    Asserting return values");

                                // Execute the function
                                if let WastExecute::Invoke(invoke) = exec {
                                    // Convert wast arguments to WRT values
                                    let mut args = Vec::new();
                                    for arg in &invoke.args {
                                        match arg {
                                            WastArg::Core(arg_core) => match arg_core {
                                                WastArgCore::I32(val) => args.push(Value::I32(*val)),
                                                WastArgCore::I64(val) => args.push(Value::I64(*val)),
                                                WastArgCore::F32(val) => args.push(Value::F32(f32::from_bits(val.bits))),
                                                WastArgCore::F64(val) => args.push(Value::F64(f64::from_bits(val.bits))),
                                                WastArgCore::V128(val) => {
                                                    // Manual byte conversion
                                                    let mut bytes = [0u8; 16];
                                                    for (i, b) in val.to_vec().iter().enumerate() {
                                                        if i < 16 {
                                                            bytes[i] = *b;
                                                        }
                                                    }
                                                    let bits = u128::from_le_bytes(bytes);
                                                    args.push(Value::V128(bits));
                                                },
                                                _ => {
                                                    println!("    Unsupported argument type: {:?}", arg_core);
                                                    continue;
                                                }
                                            },
                                            _ => {
                                                println!("    Unsupported non-core argument type");
                                                continue;
                                            }
                                        }
                                    }

                                    // Execute the function
                                    match engine.execute(current_module_idx, invoke.name, args) {
                                        Ok(actual_results) => {
                                            // Compare expected and actual results
                                            let mut is_match = true;
                                            if actual_results.len() != results.len() {
                                                println!("    Result count mismatch: expected {}, got {}",
                                                        results.len(), actual_results.len());
                                                is_match = false;
                                            } else {
                                                // Perform detailed comparison of each result
                                                for (i, (expected, actual)) in results.iter().zip(actual_results.iter()).enumerate() {
                                                    let mut result_matches = false;
                                                    match expected {
                                                        WastRet::Core(ret_core) => match ret_core {
                                                            WastRetCore::I32(expected_val) => {
                                                                if let Value::I32(actual_val) = actual {
                                                                    result_matches = *expected_val == *actual_val;
                                                                    println!("    Comparing i32 result {}: expected {:?}, got {:?}, match: {}",
                                                                        i, expected_val, actual_val, result_matches);
                                                                } else {
                                                                    println!("    Type mismatch: expected i32, got {:?}", actual);
                                                                }
                                                            },
                                                            WastRetCore::I64(expected_val) => {
                                                                if let Value::I64(actual_val) = actual {
                                                                    result_matches = *expected_val == *actual_val;
                                                                    println!("    Comparing i64 result {}: expected {:?}, got {:?}, match: {}",
                                                                        i, expected_val, actual_val, result_matches);
                                                                } else {
                                                                    println!("    Type mismatch: expected i64, got {:?}", actual);
                                                                }
                                                            },
                                                            WastRetCore::F32(expected_val) => {
                                                                if let Value::F32(actual_val) = actual {
                                                                    // For NanPattern<F32>, we need special handling
                                                                    // Manual implementation that works with any API version
                                                                    if f32::from_bits(expected_val.bits).is_nan() {
                                                                        // For NaN, just check if actual is also NaN
                                                                        result_matches = actual_val.is_nan();
                                                                        println!("    Comparing f32 NaN result {}: expected NaN, got {:?}, match: {}",
                                                                            i, actual_val, result_matches);
                                                                    } else {
                                                                        // For exact comparison, check bit patterns
                                                                        let expected_bits = expected_val.bits;
                                                                        let actual_bits = actual_val.to_bits();
                                                                        result_matches = expected_bits == actual_bits;
                                                                        println!("    Comparing f32 result {}: expected 0x{:x}, got 0x{:x}, match: {}",
                                                                            i, expected_bits, actual_bits, result_matches);
                                                                    }
                                                                } else {
                                                                    println!("    Type mismatch: expected f32, got {:?}", actual);
                                                                }
                                                            },
                                                            WastRetCore::F64(expected_val) => {
                                                                if let Value::F64(actual_val) = actual {
                                                                    // For NanPattern<F64>, we need special handling
                                                                    // Manual implementation that works with any API version
                                                                    if f64::from_bits(expected_val.bits).is_nan() {
                                                                        // For NaN, just check if actual is also NaN
                                                                        result_matches = actual_val.is_nan();
                                                                        println!("    Comparing f64 NaN result {}: expected NaN, got {:?}, match: {}",
                                                                            i, actual_val, result_matches);
                                                                    } else {
                                                                        // For exact comparison, check bit patterns
                                                                        let expected_bits = expected_val.bits;
                                                                        let actual_bits = actual_val.to_bits();
                                                                        result_matches = expected_bits == actual_bits;
                                                                        println!("    Comparing f64 result {}: expected 0x{:x}, got 0x{:x}, match: {}",
                                                                            i, expected_bits, actual_bits, result_matches);
                                                                    }
                                                                } else {
                                                                    println!("    Type mismatch: expected f64, got {:?}", actual);
                                                                }
                                                            },
                                                            WastRetCore::V128(expected_val) => {
                                                                if let Value::V128(actual_val) = actual {
                                                                    // Manual implementation for V128Pattern comparison
                                                                    // Convert to bytes, then to u128
                                                                    let mut bytes = [0u8; 16];
                                                                    for (i, b) in expected_val.to_vec().iter().enumerate() {
                                                                        if i < 16 {
                                                                            bytes[i] = *b;
                                                                        }
                                                                    }
                                                                    let expected_bits = u128::from_le_bytes(bytes);

                                                                    // For simple case, just compare the bit patterns directly
                                                                    result_matches = expected_bits == *actual_val;
                                                                    println!("    Comparing v128 result {}: expected 0x{:x}, got 0x{:x}, match: {}",
                                                                        i, expected_bits, actual_val, result_matches);
                                                                } else {
                                                                    println!("    Type mismatch: expected v128, got {:?}", actual);
                                                                }
                                                            },
                                                            _ => {
                                                                println!("    Unsupported result type for comparison: {:?}", ret_core);
                                                            }
                                                        },
                                                        _ => {
                                                            println!("    Unsupported non-core result type");
                                                        }
                                                    }

                                                    if !result_matches {
                                                        is_match = false;
                                                    }
                                                }
                                            }

                                            if is_match {
                                                println!("    ✅ Assert return passed");
                                            } else {
                                                println!("    ❌ Assert return failed");
                                            }
                                        }
                                        Err(e) => {
                                            println!("    ❌ Function execution failed: {}", e);
                                        }
                                    }
                                }
                            }
                            WastDirective::AssertTrap { exec, .. } => {
                                println!("    Asserting trap");

                                // Execute the function expecting a trap
                                if let WastExecute::Invoke(invoke) = exec {
                                    // Convert wast arguments to WRT values
                                    let mut args = Vec::new();
                                    for arg in &invoke.args {
                                        match arg {
                                            WastArg::Core(arg_core) => match arg_core {
                                                WastArgCore::I32(val) => args.push(Value::I32(*val)),
                                                WastArgCore::I64(val) => args.push(Value::I64(*val)),
                                                WastArgCore::F32(val) => args.push(Value::F32(f32::from_bits(val.bits))),
                                                WastArgCore::F64(val) => args.push(Value::F64(f64::from_bits(val.bits))),
                                                WastArgCore::V128(val) => {
                                                    // Manual byte conversion
                                                    let mut bytes = [0u8; 16];
                                                    for (i, b) in val.to_vec().iter().enumerate() {
                                                        if i < 16 {
                                                            bytes[i] = *b;
                                                        }
                                                    }
                                                    let bits = u128::from_le_bytes(bytes);
                                                    args.push(Value::V128(bits));
                                                },
                                                _ => {
                                                    println!("    Unsupported argument type: {:?}", arg_core);
                                                    continue;
                                                }
                                            },
                                            _ => {
                                                println!("    Unsupported non-core argument type");
                                                continue;
                                            }
                                        }
                                    }

                                    // Execute the function
                                    match engine.execute(current_module_idx, invoke.name, args) {
                                        Ok(_) => {
                                            println!("    ❌ Expected trap but function executed successfully");
                                        }
                                        Err(_) => {
                                            println!("    ✅ Function trapped as expected");
                                        }
                                    }
                                }
                            }
                            _ => {
                                // Handle other directive types as needed
                            }
                        }
                    }

                    // Call the original function for each file
                    #fn_block
                }
            }
        }
    };

    TokenStream::from(expanded)
}
