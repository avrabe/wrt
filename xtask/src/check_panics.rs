/*
use anyhow::{Context, Result, bail};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;
use regex::Regex;
use xshell::Shell;

const PANIC_DOC_REGEX_STR: &str = r"#+\s*Panics\s*\n(.*\n)*?(\n|\z)";
const MIN_PANIC_LINES: usize = 1; // Minimum lines of content expected under # Panics
const PANIC_TEMPLATE: &str = "# Panics\n\n- If `condition` is not met.";

pub fn run(sh: &Shell, fix: bool, only_failures: bool) -> Result<()> {
    info!("Starting panic documentation checks...");
    let mut all_crates_passed = true;
    let mut found_failures = false;

    // Example: Iterate over workspace members. This needs actual workspace parsing.
    // For now, let's assume a few key crate paths or use a simpler discovery.
    // This is a placeholder for robust crate discovery.
    let members_output = sh.cmd("cargo", vec!["pkgid", "--quiet"]).read()?;
    let crate_names: Vec<String> = members_output
        .lines()
        .filter_map(|line| line.split('#').next().map(|s| s.trim().to_string()))
        .filter(|name| !name.is_empty() && name.starts_with("wrt")) // Focus on wrt crates
        .collect();

    info!("Checking crates: {:?}", crate_names);

    for crate_name in crate_names {
        match check_crate(sh, &crate_name, fix, only_failures) {
            Ok(passed) => {
                if !passed {
                    all_crates_passed = false;
                    found_failures = true;
                }
            }
            Err(e) => {
                error!("Error checking crate {}: {}", crate_name, e);
                all_crates_passed = false;
                found_failures = true;
            }
        }
    }

    if only_failures && !found_failures {
        info!("No panic documentation failures found (searched for failures only).");
        return Ok(());
    }

    if !all_crates_passed {
        bail!("Panic documentation checks failed for one or more crates.");
    }

    info!("Panic documentation checks completed successfully.");
    Ok(())
}

fn check_crate(sh: &Shell, crate_name: &str, fix: bool, only_failures: bool) -> Result<bool> {
    info!("Checking crate: {}", crate_name);
    let mut crate_passed = true;
    let mut crate_had_failure = false;

    // Simplistic way to find Rust files; a real solution would use `cargo metadata` or similar.
    // This assumes crate_name is a directory relative to workspace root or we need to find its path.
    // Let's assume we can form a path like "./{crate_name}/src"
    let crate_src_path_str = format!("./{}/src", crate_name.replace(":", "/")); // Handle pkgid format if necessary
    let crate_src_path = Path::new(&crate_src_path_str);

    if !crate_src_path.exists() || !crate_src_path.is_dir() {
        warn!("Source directory not found for crate {}: {:?}. Skipping.", crate_name, crate_src_path);
        return Ok(true); // Skip if src dir not found
    }

    for entry in walkdir::WalkDir::new(crate_src_path).into_iter().filter_map(Result::ok) {
        if entry.file_type().is_file() && entry.path().extension().map_or(false, |ext| ext == "rs") {
            let file_path_str = entry.path().to_string_lossy().into_owned();
            match check_panic_doc_fields(&file_path_str, fix) {
                Ok(passed_file) => {
                    if !passed_file {
                        crate_passed = false;
                        crate_had_failure = true;
                        if !only_failures {
                            error!("Panic doc issue in file: {}", file_path_str);
                        }
                    }
                }
                Err(e) => {
                    error!("Error checking file {}: {}", file_path_str, e);
                    crate_passed = false;
                    crate_had_failure = true;
                }
            }
        }
    }

    if only_failures && !crate_had_failure {
        // If we only care about failures and this crate had none, it effectively passed for this mode.
        return Ok(true);
    }
    if !only_failures {
        if crate_passed {
            info!("Crate {} passed panic doc checks.", crate_name);
        } else {
            info!("Crate {} failed panic doc checks.", crate_name);
        }
    }
    Ok(crate_passed)
}

fn check_panic_doc_fields(file_path: &str, fix: bool) -> Result<bool> {
    let content = fs::read_to_string(file_path)
        .with_context(|| format!("Failed to read file: {}", file_path))?;
    let panic_regex = Regex::new(PANIC_DOC_REGEX_STR).unwrap();
    let lines: Vec<&str> = content.lines().collect();
    let mut all_good = true;
    let mut modified_content = String::new(); // For --fix
    let mut last_append_end = 0;

    // Iterate over function definitions (simplistic: looks for `fn `)
    // A proper solution would use syn or similar for Rust code parsing.
    for (i, line) in lines.iter().enumerate() {
        if line.trim_start().starts_with("pub fn") || line.trim_start().starts_with("fn") {
            // Found a function. Look for its preceding doc comments.
            let mut doc_comment_block = String::new();
            let mut doc_start_line = i;
            for j in (0..i).rev() {
                if lines[j].trim_start().starts_with("///") {
                    doc_comment_block.insert_str(0, lines[j].trim_start_matches("/// ").trim_start_matches("///"));
                    doc_comment_block.insert(0, '\n');
                    doc_start_line = j;
                } else if lines[j].trim().is_empty() && j > 0 && lines[j-1].trim_start().starts_with("///") {
                    // Allow a single empty line between doc comment blocks
                    continue;
                } else if lines[j].trim_start().starts_with("#[") { // Attributes are ok
                    continue;
                } else {
                    break; // End of doc comment block
                }
            }
            if !doc_comment_block.is_empty() {
                doc_comment_block = doc_comment_block.trim_start().to_string();
            }

            // Now check if this doc_comment_block has a # Panics section
            if !doc_comment_block.is_empty() {
                match panic_regex.captures(&doc_comment_block) {
                    Some(caps) => {
                        // Found a # Panics section. Check its content length.
                        let panic_content = caps.get(1).map_or("", |m| m.as_str().trim());
                        if panic_content.lines().filter(|l| !l.trim().is_empty()).count() < MIN_PANIC_LINES {
                            error!(
                                "File: {}\n  Function starting line ~{}: Panic section is too short or empty.\n  Found content:\n{}",
                                file_path, i + 1, panic_content
                            );
                            all_good = false;
                        }
                    }
                    None => {
                        // No # Panics section found.
                        error!(
                            "File: {}\n  Function starting line ~{}: Missing # Panics section in doc comments.",
                            file_path, i + 1
                        );
                        all_good = false;
                        if fix {
                            // Add template to doc_comment_block string (not to file directly yet)
                            // This needs to be inserted correctly into the original file's doc comment lines.
                            // The current `doc_comment_block` is a reconstruction.
                            // Simpler fix: add it at the end of the existing doc comments for this function.
                            warn!("Fix mode: Attempting to add panic template for function at line {} in {}", i+1, file_path);
                            // This fix logic is complex because we need to modify lines[doc_start_line..i]
                            // For simplicity, let's signal to add it to original lines directly.
                            // This will require another pass or more careful line manipulation here.
                            // For now, we just signal `add_panic_doc_template` can be called with original line numbers.

                            // Simplification for --fix: just append to the file content for now if it's missing.
                            // This is not ideal as it won't be in the right doc comment.
                            // A proper fix requires modifying the lines in `lines` Vec and then writing `lines.join("\n")`.
                            // This needs more robust parsing of where doc comments end for the function.
                            // The `add_panic_doc_template` function is a stub for this.

                            // For now, let's say we need to insert PANIC_TEMPLATE into the lines
                            // just before line `i`. We need to find where the doc comments for fn at line `i` end.
                            // This is tricky. The current `doc_start_line` is where they start.
                            // A simple but possibly incorrect fix:
                            if !modified_content.ends_with("\n\n") { modified_content.push_str("\n"); }
                            modified_content.push_str(PANIC_TEMPLATE);
                            modified_content.push_str("\n");
                        }
                    }
                }
            }
        }
        if fix {
            if last_append_end <= i {
                 modified_content.push_str(lines[i]);
                 modified_content.push('\n');
                 last_append_end = i + 1;
            }
        }
    }

    if fix && !all_good {
        // Remove trailing newline if added by loop
        if modified_content.ends_with('\n') { modified_content.pop(); }
        fs::write(file_path, modified_content)
            .with_context(|| format!("Failed to write fixes to file: {}", file_path))?;
        info!("Applied fixes to {}", file_path);
    }

    Ok(all_good)
}

// This function is a stub and needs proper implementation for --fix
fn add_panic_doc_template(file: &str, line: usize) -> Result<()> {
    warn!("Attempting to add panic template to {} at line {}. THIS IS A STUB.", file, line);
    // 1. Read the file content lines.
    // 2. Insert PANIC_TEMPLATE lines before the function definition at `line`,
    //    ensuring correct indentation and /// prefix.
    // 3. Write the modified lines back to the file.
    Ok(())
}
*/
