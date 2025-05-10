// Logic copied from previous wasm-utils/src/main.rs
use anyhow::{bail, Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use walkdir::WalkDir;

/// Builds all .wat files found recursively in the given directory.
pub fn build_all_wat(dir: &Path) -> Result<()> {
    println!("Building WAT files in {}", dir.display());
    let mut count = 0;
    let mut found_any = false;
    for wat_file in find_wat_files(dir)? {
        found_any = true;
        let wasm_file = wat_to_wasm_path(&wat_file)?;
        if convert_wat(&wat_file, &wasm_file, true)? {
            count += 1;
        }
    }
    if !found_any {
        println!("No WAT files found in {}.", dir.display());
    } else {
        println!("Successfully built/updated {} WASM files.", count);
    }
    Ok(())
}

/// Checks if all .wasm files are up-to-date relative to their .wat counterparts.
pub fn check_all_wat(dir: &Path) -> Result<()> {
    println!("Checking WASM files are up-to-date in {}", dir.display());
    let mut needs_rebuild = false;
    let mut found_wat = false;

    for wat_file in find_wat_files(dir)? {
        found_wat = true;
        let wasm_file = wat_to_wasm_path(&wat_file)?;
        if needs_conversion(&wat_file, &wasm_file)? {
            println!("WARNING: WASM file needs to be rebuilt: {}", wasm_file.display());
            needs_rebuild = true;
        }
    }

    if !found_wat {
        println!("No WAT files found in {}.", dir.display());
        return Ok(());
    }

    if needs_rebuild {
        bail!(
            "Some WASM files need to be rebuilt. Run 'cargo xtask wasm build {}' to update them.",
            dir.display()
        );
    } else {
        println!("All WASM files are up-to-date.");
    }
    Ok(())
}

/// Converts a single WAT file to WASM, optionally skipping if up-to-date.
/// Returns true if conversion happened, false otherwise.
pub fn convert_wat(wat_path: &Path, wasm_path: &Path, skip_if_fresh: bool) -> Result<bool> {
    if skip_if_fresh && !needs_conversion(wat_path, wasm_path)? {
        // Only print skipping message if the WASM file actually exists
        if wasm_path.exists() {
            println!(
                "Skipping {} (WASM file {} is up to date)",
                wat_path.display(),
                wasm_path.display()
            );
        }
        return Ok(false);
    }

    println!("Converting {} to {}...", wat_path.display(), wasm_path.display());

    let wat_text = fs::read_to_string(wat_path)
        .with_context(|| format!("Failed to read WAT file: {}", wat_path.display()))?;

    let wasm_bytes = wat::parse_str(&wat_text)
        .with_context(|| format!("Failed to parse WAT file: {}", wat_path.display()))?;

    // Ensure target directory exists
    if let Some(parent) = wasm_path.parent() {
        fs::create_dir_all(parent).with_context(|| {
            format!("Failed to create directory for WASM file: {}", parent.display())
        })?;
    }

    fs::write(wasm_path, wasm_bytes)
        .with_context(|| format!("Failed to write WASM file: {}", wasm_path.display()))?;

    println!("Conversion successful: {}", wasm_path.display());
    Ok(true)
}

/// Finds all .wat files recursively in the given directory.
fn find_wat_files(dir: &Path) -> Result<Vec<PathBuf>> {
    if !dir.is_dir() {
        // Allow non-existent directories, just return empty list
        if !dir.exists() {
            return Ok(Vec::new());
        }
        bail!("Provided path is not a directory: {}", dir.display());
    }
    let mut wat_files = Vec::new();
    for entry in WalkDir::new(dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "wat"))
    {
        wat_files.push(entry.path().to_path_buf());
    }
    Ok(wat_files)
}

/// Determines the corresponding .wasm path for a given .wat path.
pub fn wat_to_wasm_path(wat_path: &Path) -> Result<PathBuf> {
    if wat_path.extension().is_none_or(|ext| ext != "wat") {
        bail!("Input file does not have a .wat extension: {}", wat_path.display());
    }
    Ok(wat_path.with_extension("wasm"))
}

/// Checks if the WAT file needs conversion (WASM doesn't exist or WAT is newer).
fn needs_conversion(wat_path: &Path, wasm_path: &Path) -> Result<bool> {
    if !wasm_path.exists() {
        return Ok(true);
    }
    let wat_meta = fs::metadata(wat_path)?;
    let wasm_meta = fs::metadata(wasm_path)?;
    // Handle potential errors getting modified time
    let wat_modified = wat_meta.modified().unwrap_or_else(|_| SystemTime::now());
    let wasm_modified = wasm_meta.modified().unwrap_or(SystemTime::UNIX_EPOCH);

    Ok(wat_modified > wasm_modified)
}
