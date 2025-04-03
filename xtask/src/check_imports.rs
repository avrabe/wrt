// Logic copied from previous check-imports/src/main.rs
use anyhow::Result;
use std::fs;
use std::path::Path;
use walkdir::WalkDir;

pub fn run(dirs_to_check: &[&Path]) -> Result<()> {
    println!("Checking import organization...");
    let mut warnings = 0;

    for dir in dirs_to_check {
        for entry in WalkDir::new(dir)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().map_or(false, |ext| ext == "rs"))
        {
            let path = entry.path();
            if check_file_imports(path)? {
                warnings += 1;
            }
        }
    }

    if warnings > 0 {
        println!(
            "Found {} files with potential import order issues.",
            warnings
        );
        // Optionally, exit with an error code if warnings were found
        // std::process::exit(1);
    } else {
        println!("Import organization looks good.");
    }

    Ok(())
}

fn check_file_imports(path: &Path) -> Result<bool> {
    let content = fs::read_to_string(path)?;
    let mut first_import: Option<&str> = None;
    let mut has_imports = false;

    // Find the first non-comment, non-empty line that starts with "use "
    for line in content.lines() {
        let trimmed_line = line.trim();
        if trimmed_line.is_empty() || trimmed_line.starts_with("//") {
            continue; // Skip empty lines and comments
        }
        if trimmed_line.starts_with("#!") || trimmed_line.starts_with("#[") {
            continue; // Skip shebangs and outer attributes
        }

        if trimmed_line.starts_with("use ") {
            has_imports = true;
            first_import = Some(line); // Keep original line formatting
            break; // Found the first import, no need to check further
        } else {
            // We found code (mod, fn, struct, etc.) before any 'use' statement.
            // This means either no imports or they appear after some code (unlikely for std).
            break;
        }
    }

    if has_imports {
        if let Some(import_line) = first_import {
            let trimmed_first_import = import_line.trim();
            if !trimmed_first_import.starts_with("use std")
                && !trimmed_first_import.starts_with("use core")
                && !trimmed_first_import.starts_with("use alloc")
            {
                println!(
                    "WARN: {} should have standard library imports (std, core, alloc) first.",
                    path.display()
                );
                println!("  First import found: {}", import_line);
                return Ok(true); // Found a warning
            }
        } else {
            // This case should technically not happen if has_imports is true
            eprintln!("Internal logic error checking file: {}", path.display());
        }
    }

    Ok(false) // No warning
}
