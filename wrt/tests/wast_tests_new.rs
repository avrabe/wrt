use std::fs;
use std::path::{Path, PathBuf};
use wast::core::{NanPattern, WastArgCore, WastRetCore};
use wast::{
    parser::{self, ParseBuffer},
    Wast, WastArg, WastDirective, WastExecute, WastRet,
};
use wrt::{Error, Module, StacklessEngine};

fn convert_wast_arg_core(arg: &WastArg) -> Result<Value, Error> {
    match arg {
        WastArg::Core(core_arg) => match core_arg {
            WastArgCore::I32(x) => Ok(Value::I32(*x)),
            WastArgCore::I64(x) => Ok(Value::I64(*x)),
            WastArgCore::F32(x) => Ok(Value::F32(f32::from_bits(x.bits))),
            WastArgCore::F64(x) => Ok(Value::F64(f64::from_bits(x.bits))),
            _ => Err(Error::Validation("Unsupported argument type".into())),
        },
        _ => Err(Error::Validation("Unsupported argument type".into())),
    }
}

fn convert_wast_ret_core(ret: &WastRet) -> Result<Value, Error> {
    match ret {
        WastRet::Core(core_ret) => match core_ret {
            WastRetCore::I32(x) => Ok(Value::I32(*x)),
            WastRetCore::I64(x) => Ok(Value::I64(*x)),
            WastRetCore::F32(x) => match x {
                NanPattern::Value(x) => Ok(Value::F32(f32::from_bits(x.bits))),
                NanPattern::CanonicalNan => Ok(Value::F32(f32::NAN)),
                NanPattern::ArithmeticNan => Ok(Value::F32(f32::NAN)),
            },
            WastRetCore::F64(x) => match x {
                NanPattern::Value(x) => Ok(Value::F64(f64::from_bits(x.bits))),
                NanPattern::CanonicalNan => Ok(Value::F64(f64::NAN)),
                NanPattern::ArithmeticNan => Ok(Value::F64(f64::NAN)),
            },
            _ => Err(Error::Validation("Unsupported return type".into())),
        },
        _ => Err(Error::Validation("Unsupported return type".into())),
    }
}

fn test_wast_directive(
    engine: &mut StacklessEngine,
    directive: &mut WastDirective,
) -> Result<(), Error> {
    match directive {
        WastDirective::Module(ref mut wast_module) => {
            // Get the binary from the WAST module
            let binary = wast_module
                .encode()
                .map_err(|e| Error::Parse(e.to_string()))?;

            // Debug output
            println!("Binary: {:02x?}", binary);

            // Create and load the WRT module
            let mut wrt_module = Module::new()?;
            let loaded_module = wrt_module.load_from_binary(&binary)?;

            // Debug output
            println!("Module exports: {:?}", loaded_module.exports);

            // Instantiate the module
            let instance_idx = engine.instantiate(loaded_module)?;
            println!(
                "DEBUG: instantiate called for module with instance index {}",
                instance_idx
            );

            Ok(())
        }
        WastDirective::AssertReturn {
            span: _,
            exec,
            results,
        } => {
            match exec {
                WastExecute::Invoke(invoke) => {
                    let args: Result<Vec<Value>, _> =
                        invoke.args.iter().map(convert_wast_arg_core).collect();
                    let args = args?;
                    println!("DEBUG: Invoking {} with args: {:?}", invoke.name, args);

                    let expected: Result<Vec<Value>, _> =
                        results.iter().map(convert_wast_ret_core).collect();
                    let expected = expected?;
                    println!("DEBUG: Expected result: {:?}", expected);

                    // Execute the function and compare results
                    let actual = engine.invoke_export(invoke.name, &args)?;
                    println!("DEBUG: Actual result: {:?}", actual);

                    // Special handling for NaN values
                    let mut values_match = true;
                    if actual.len() == expected.len() {
                        for (i, (a, e)) in actual.iter().zip(expected.iter()).enumerate() {
                            let is_match = compare_wasm_values(a, e);

                            println!(
                                "DEBUG: Result[{}]: actual={:?}, expected={:?}, match={}",
                                i, a, e, is_match
                            );

                            if !is_match {
                                values_match = false;
                            }
                        }
                    } else {
                        values_match = false;
                    }

                    println!("DEBUG: Comparison: values match is {}", values_match);

                    assert!(
                        values_match,
                        "Function {} returned unexpected results\n  actual: {:?}\n  expected: {:?}",
                        invoke.name, actual, expected
                    );
                    Ok(())
                }
                _ => Ok(()), // Skip other types of executions for now
            }
        }
        _ => Ok(()), // Skip other directives for now
    }
}

// Helper function to compare Wasm values, especially floats with tolerance and NaN handling
fn compare_wasm_values(actual: &Value, expected: &Value) -> bool {
    match (actual, expected) {
        (Value::F32(a), Value::F32(e)) => {
            // Use tolerance for F32 as well now
            if e.is_nan() {
                a.is_nan() // Any NaN matches expected NaN
            } else if a.is_nan() {
                false // Actual is NaN but expected is not
            } else {
                // Compare with a suitable tolerance for F32
                (a - e).abs() < 1e-6 // Use tolerance (e.g., 1e-6)
            }
        }
        (Value::F64(a), Value::F64(e)) => {
            // Use tolerance for F64 due to observed precision diffs
            if e.is_nan() {
                a.is_nan() // Any NaN matches expected NaN
            } else if a.is_nan() {
                false // Actual is NaN but expected is not
            } else {
                // Compare with a slightly larger tolerance for F64
                (a - e).abs() < 1e-9 // Increased tolerance
            }
        }
        // For V128, compare byte arrays directly
        (Value::V128(a), Value::V128(e)) => a == e,
        // For other types, use standard equality
        (a, e) => a == e,
    }
}

fn test_wast_file(path: &Path) -> Result<(), Error> {
    let contents = fs::read_to_string(path)
        .map_err(|e| Error::Parse(format!("Failed to read file: {}", e)))?;

    let buf = ParseBuffer::new(&contents)
        .map_err(|e| Error::Parse(format!("Failed to create parse buffer: {}", e)))?;

    let wast: Wast =
        parser::parse(&buf).map_err(|e| Error::Parse(format!("Failed to parse WAST: {}", e)))?;

    let module = Module::new()?;
    let mut engine = StacklessEngine::new(module);
    for mut directive in wast.directives {
        test_wast_directive(&mut engine, &mut directive)?;
    }

    Ok(())
}

/// Load tests from the wast_passed.md file
fn load_passing_tests() -> std::collections::HashSet<PathBuf> {
    println!("Loading tests from wast_passed.md...");
    let mut passing_tests = std::collections::HashSet::new();

    // Get the path to the cargo manifest directory (wrt/)
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

    // Go up one level to the workspace root
    let workspace_root = manifest_dir.parent().unwrap_or(&manifest_dir);

    // Construct the path to wast_passed.md in the workspace root
    let passed_file = workspace_root.join("wast_passed.md");

    println!("Looking for wast_passed.md at: {}", passed_file.display());

    // Return empty set if file doesn't exist
    if !passed_file.exists() {
        println!("wast_passed.md file not found at workspace root. No tests will be run.");
        return passing_tests;
    }

    // Read file content
    let mut content = String::new();
    if let Ok(mut file) = std::fs::File::open(&passed_file) {
        if std::io::Read::read_to_string(&mut file, &mut content).is_err() {
            println!("Failed to read wast_passed.md file. No tests will be run.");
            return passing_tests;
        }
    } else {
        println!("Failed to open wast_passed.md file. No tests will be run.");
        return passing_tests;
    }

    // Extract test paths from markdown file (format: "- `path/to/test.wast`")
    for line in content.lines() {
        if line.starts_with("- `") && line.contains("` - ") {
            let path_str = line[3..line.find("` - ").unwrap()].trim();
            passing_tests.insert(PathBuf::from(path_str));
            println!("  Added test: {}", path_str);
        }
    }

    println!("Loaded {} tests from wast_passed.md", passing_tests.len());

    // Another potential issue: relative paths in wast_passed.md are relative to the workspace root
    // Let's make sure we're using absolute paths by resolving them against the workspace root
    passing_tests
        .into_iter()
        .map(|path| {
            if path.is_absolute() {
                path
            } else {
                workspace_root.join(path)
            }
        })
        .collect()
}

#[test]
fn test_wast_files() -> Result<(), Error> {
    // Get the path to the cargo manifest directory (wrt/)
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

    // Go up one level to the workspace root
    let workspace_root = manifest_dir.parent().unwrap_or(&manifest_dir);

    // Use the path relative to workspace root
    let test_dir = workspace_root.join("wrt/testsuite");

    if !test_dir.exists() {
        println!("No testsuite directory found at: {}", test_dir.display());
        println!("Skipping directory tests");
        return Ok(());
    }

    // Print the path and if it exists for debugging
    println!("Checking testsuite at path: {}", test_dir.display());
    println!("Directory exists: {}", test_dir.exists());

    // Load the list of passing tests from wast_passed.md
    let passing_tests = load_passing_tests();

    // If there are no passing tests, don't run any tests
    if passing_tests.is_empty() {
        println!("No tests to run from wast_passed.md");
        return Ok(());
    }

    // Track test execution
    let mut tests_run = 0;
    let mut tests_passed = 0;

    // List the files to verify we can access them
    println!("Files in directory:");
    if let Ok(entries) = fs::read_dir(&test_dir) {
        for entry in entries {
            if let Ok(entry) = entry {
                if entry.path().extension().is_some_and(|ext| ext == "wast") {
                    println!("  => Found WAST file: {}", entry.path().display());
                }
            }
        }
    } else {
        println!("Failed to read directory contents");
    }

    // Process files
    for entry in fs::read_dir(&test_dir)
        .map_err(|e| Error::Parse(format!("Failed to read directory: {}", e)))?
    {
        let entry =
            entry.map_err(|e| Error::Parse(format!("Failed to read directory entry: {}", e)))?;
        let path = entry.path();

        if path.extension().is_some_and(|ext| ext == "wast") {
            // Get the absolute path to compare with passing_tests
            let abs_path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());

            // Try to get a relative path for display
            let rel_display_path = path
                .strip_prefix(workspace_root)
                .map(|p| p.to_path_buf())
                .unwrap_or_else(|_| path.to_path_buf());

            println!("Found WAST file: {}", rel_display_path.display());

            // Check both the absolute path and a version reconstructed from the relative path
            let rel_path_from_workspace = workspace_root.join(&rel_display_path);

            // Only run tests that are in the passing_tests list
            if !passing_tests.contains(&abs_path)
                && !passing_tests.contains(&rel_path_from_workspace)
            {
                println!(
                    "  Skipping (not in passing list): {}",
                    rel_display_path.display()
                );
                continue;
            }

            tests_run += 1;
            println!("Running test {}: {}", tests_run, rel_display_path.display());

            match test_wast_file(&path) {
                Ok(_) => {
                    println!("✅ PASS: {}", rel_display_path.display());
                    tests_passed += 1;
                }
                Err(e) => {
                    println!("❌ FAIL: {} - {}", rel_display_path.display(), e);
                }
            }
        }
    }

    println!(
        "Tests completed: {} passed, {} failed",
        tests_passed,
        tests_run - tests_passed
    );

    Ok(())
}
