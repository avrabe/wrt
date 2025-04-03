// Logic copied from previous fs-utils/src/main.rs
use anyhow::{Context, Result};
use fs_extra::dir;
use std::fs;
use std::path::Path;
use walkdir::{DirEntry, WalkDir};

pub fn rmrf(path: &Path) -> Result<()> {
    if !path.exists() {
        println!("Path does not exist, skipping removal: {}", path.display());
        return Ok(());
    }
    println!("Recursively removing {}...", path.display());
    if path.is_dir() {
        // Use fs_extra for reliable cross-platform directory removal
        dir::remove(path).context(format!("Failed to remove directory '{}'", path.display()))?;
    } else if path.is_file() {
        fs::remove_file(path).context(format!("Failed to remove file '{}'", path.display()))?;
    } else {
        println!("Path is neither a file nor a directory: {}", path.display());
    }
    // println!("Removal successful: {}", path.display()); // Maybe too verbose
    Ok(())
}

pub fn mkdirp(path: &Path) -> Result<()> {
    if path.exists() {
        // println!("Directory already exists: {}", path.display()); // Not an error
        return Ok(());
    }
    println!("Creating directory (and parents): {}...", path.display());
    fs::create_dir_all(path).context(format!("Failed to create directory '{}'", path.display()))?;
    // println!("Directory creation successful: {}", path.display());
    Ok(())
}

// Helper to match glob patterns (basic wildcard implementation)
fn matches_pattern(filename: &str, pattern: &str) -> bool {
    if pattern == "*" {
        return true;
    }
    if pattern.starts_with('*') && pattern.ends_with('*') {
        filename.contains(&pattern[1..pattern.len() - 1])
    } else if pattern.starts_with('*') {
        filename.ends_with(&pattern[1..])
    } else if pattern.ends_with('*') {
        filename.starts_with(&pattern[..pattern.len() - 1])
    } else {
        filename == pattern
    }
}

// Helper function to filter WalkDir entries based on pattern
fn filter_by_pattern(entry: &DirEntry, pattern: &str) -> bool {
    entry.file_type().is_file()
        && entry
            .file_name()
            .to_str()
            .map(|s| matches_pattern(s, pattern))
            .unwrap_or(false)
}

pub fn find_delete(directory: &Path, pattern: &str) -> Result<()> {
    if !directory.is_dir() {
        if directory.exists() {
            println!(
                "Warning: Path is not a directory, skipping find/delete: {}",
                directory.display()
            );
        } // else: Non-existent path is fine, nothing to delete.
        return Ok(());
    }
    println!(
        "Finding and deleting files matching '{}' in {}...",
        pattern,
        directory.display()
    );
    let mut deleted_count = 0;
    for entry in WalkDir::new(directory)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| filter_by_pattern(e, pattern))
    {
        let path = entry.path();
        match fs::remove_file(path) {
            Ok(_) => {
                // println!("Deleted: {}", path.display()); // Verbose
                deleted_count += 1;
            }
            Err(e) => eprintln!("Warning: Failed to delete {}: {}", path.display(), e),
        }
    }
    println!("Deleted {} files matching '{}'.", deleted_count, pattern);
    Ok(())
}

pub fn count_files(directory: &Path, pattern: &str) -> Result<()> {
    let count = if directory.is_dir() {
        WalkDir::new(directory)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| filter_by_pattern(e, pattern))
            .count()
    } else {
        if directory.exists() {
            println!(
                "Warning: Path is not a directory, cannot count files: {}",
                directory.display()
            );
        }
        0 // Non-existent or non-directory path means 0 files
    };
    println!("{}", count); // Print only the count for scripting
    Ok(())
}

pub fn file_size(path: &Path) -> Result<()> {
    let size = if path.is_file() {
        match fs::metadata(path) {
            Ok(metadata) => metadata.len(),
            Err(e) => {
                println!(
                    "Error: Failed to get metadata for file {}: {}",
                    path.display(),
                    e
                );
                0 // Output 0 on error
            }
        }
    } else {
        if path.exists() {
            println!("Error: Path exists but is not a file: {}", path.display());
        } else {
            println!("Error: File does not exist: {}", path.display());
        }
        0 // Output 0 if not a file or doesn't exist
    };
    println!("{}", size); // Print only the size for scripting
    Ok(())
}

pub fn copy_file(source: &Path, destination: &Path) -> Result<()> {
    println!(
        "Copying file from {} to {}...",
        source.display(),
        destination.display()
    );

    // Ensure the destination directory exists
    if let Some(parent) = destination.parent() {
        if !parent.exists() {
            mkdirp(parent)?;
        }
    }

    fs::copy(source, destination).context(format!(
        "Failed to copy file from {} to {}",
        source.display(),
        destination.display()
    ))?;

    println!(
        "File copy successful: {} -> {}",
        source.display(),
        destination.display()
    );
    Ok(())
}
