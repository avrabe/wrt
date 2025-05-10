use anyhow::{anyhow, Context, Result};
use std::fs::{self, File};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use xshell::{cmd, Shell};

/// Run Bazel build for the specified target
pub fn run_build(sh: &Shell, target: &str) -> Result<()> {
    println!("Building with Bazel: {}", target);
    cmd!(sh, "bazel build {target}")
        .run()
        .with_context(|| format!("Failed to build target {}", target))?;
    println!("Build successful.");
    Ok(())
}

/// Run Bazel tests for the specified target
pub fn run_test(sh: &Shell, target: &str) -> Result<()> {
    println!("Testing with Bazel: {}", target);
    cmd!(sh, "bazel test {target}")
        .run()
        .with_context(|| format!("Failed to test target {}", target))?;
    println!("Tests successful.");
    Ok(())
}

/// Generate a BUILD file for the specified package directory
pub fn generate_build_file(_sh: &Shell, directory: &Path) -> Result<()> {
    let dir_str = directory.to_string_lossy();

    // Check if directory exists
    if !directory.exists() {
        return Err(anyhow!("Directory does not exist: {}", dir_str));
    }

    // Check if there's a Cargo.toml in this directory
    let cargo_toml = directory.join("Cargo.toml");
    if !cargo_toml.exists() {
        return Err(anyhow!("No Cargo.toml found in {}", dir_str));
    }

    println!("Generating BUILD file for {}", dir_str);

    // Check if the directory already has a BUILD file
    let build_file = directory.join("BUILD");
    if build_file.exists() {
        println!("BUILD file already exists. Backing up to BUILD.bak");
        fs::copy(&build_file, directory.join("BUILD.bak"))?;
    }

    // Get the package name from Cargo.toml
    let package_name = extract_package_name(&cargo_toml)?;

    // Extract source files
    let mut src_files = Vec::new();
    let src_dir = directory.join("src");
    if src_dir.exists() {
        visit_dirs(&src_dir, &mut |file| {
            if file.extension().is_some_and(|ext| ext == "rs") {
                src_files.push(file.strip_prefix(directory).unwrap().to_path_buf());
            }
        })?;
    }

    // Create BUILD file content
    let content = create_build_file_content(&package_name, &src_files);

    // Write the BUILD file
    let mut file = File::create(&build_file)?;
    file.write_all(content.as_bytes())?;

    println!("Successfully generated BUILD file at {}", build_file.display());

    Ok(())
}

/// Migrate a just command to Bazel
pub fn migrate_just_command(_sh: &Shell, command: &str) -> Result<()> {
    // Read the justfile
    let justfile_path = Path::new("justfile");
    if !justfile_path.exists() {
        return Err(anyhow!("justfile not found in current directory"));
    }

    let file = File::open(justfile_path)?;
    let reader = BufReader::new(file);

    // Find the command in the justfile
    let mut found = false;
    let mut command_lines = Vec::new();
    let mut in_command = false;
    let command_pattern = format!("{}:", command);

    for line in reader.lines() {
        let line = line?;
        let trimmed = line.trim();

        if trimmed.starts_with(&command_pattern) {
            in_command = true;
            found = true;
            command_lines.push(line.clone());
        } else if in_command {
            if trimmed.is_empty() || (!line.starts_with("    ") && !line.starts_with("\t")) {
                in_command = false;
            } else {
                command_lines.push(line.clone());
            }
        }
    }

    if !found {
        return Err(anyhow!("Command '{}' not found in justfile", command));
    }

    println!("Found command '{}' in justfile:", command);
    for line in &command_lines {
        println!("  {}", line);
    }

    // Convert to Bazel BUILD rule
    let bazel_rule = convert_to_bazel_rule(command, &command_lines)?;

    // Create a directory if it doesn't exist
    let bazel_dir = Path::new("bazel");
    if !bazel_dir.exists() {
        fs::create_dir(bazel_dir)?;
    }

    // Write to a new .bzl file
    let bazel_file = bazel_dir.join(format!("{}.bzl", command));
    let mut file = File::create(&bazel_file)?;
    file.write_all(bazel_rule.as_bytes())?;

    println!("Successfully migrated command '{}' to {}", command, bazel_file.display());
    println!("You can now use this rule in your BUILD files or update the root BUILD.");

    Ok(())
}

/// Helper function to extract package name from Cargo.toml
fn extract_package_name(cargo_toml: &Path) -> Result<String> {
    let file = File::open(cargo_toml)?;
    let reader = BufReader::new(file);

    for line in reader.lines() {
        let line = line?;
        if line.trim().starts_with("name") {
            let parts: Vec<&str> = line.split('=').collect();
            if parts.len() >= 2 {
                let name = parts[1].trim().trim_matches('"').trim_matches('\'');
                return Ok(name.to_string());
            }
        }
    }

    Err(anyhow!("Could not find package name in Cargo.toml"))
}

/// Helper function to recursively visit directories
fn visit_dirs(dir: &Path, cb: &mut dyn FnMut(&Path)) -> Result<()> {
    if dir.is_dir() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                visit_dirs(&path, cb)?;
            } else {
                cb(&path);
            }
        }
    }
    Ok(())
}

/// Create Bazel BUILD file content
fn create_build_file_content(package_name: &str, src_files: &[PathBuf]) -> String {
    let mut content = String::new();

    // Add load statements
    content.push_str(
        "load(\"@rules_rust//rust:defs.bzl\", \"rust_library\", \"rust_test\", \"rust_binary\")\n",
    );
    content.push_str("load(\"@crates//:defs.bzl\", \"aliases\", \"all_crate_deps\")\n\n");

    // Determine if there's a main.rs (binary) or lib.rs (library)
    let has_main = src_files.iter().any(|p| p.ends_with("src/main.rs"));
    let has_lib = src_files.iter().any(|p| p.ends_with("src/lib.rs"));

    if has_lib {
        // Create library target
        content.push_str(&format!("rust_library(\n    name = \"{}\",\n", package_name));
        content.push_str("    srcs = glob([\"src/**/*.rs\"]),\n");
        content.push_str("    edition = \"2021\",\n");
        content.push_str("    deps = all_crate_deps(\n        normal = True,\n    ),\n");
        content.push_str(
            "    proc_macro_deps = all_crate_deps(\n        proc_macro = True,\n    ),\n",
        );
        content.push_str("    visibility = [\"//visibility:public\"],\n");
        content.push_str(")\n\n");
    }

    if has_main {
        // Create binary target
        content.push_str(&format!("rust_binary(\n    name = \"{}\",\n", package_name));
        content.push_str("    srcs = glob([\"src/**/*.rs\"]),\n");
        content.push_str("    edition = \"2021\",\n");
        content.push_str("    deps = all_crate_deps(\n        normal = True,\n    ),\n");
        content.push_str(
            "    proc_macro_deps = all_crate_deps(\n        proc_macro = True,\n    ),\n",
        );
        content.push_str("    visibility = [\"//visibility:public\"],\n");
        content.push_str(")\n\n");
    }

    // Add test target
    content.push_str(&format!("rust_test(\n    name = \"{}_test\",\n", package_name));
    content.push_str("    srcs = glob([\"src/**/*.rs\"]),\n");
    content.push_str("    edition = \"2021\",\n");
    content.push_str("    deps = all_crate_deps(\n        normal = True,\n    ),\n");
    content.push_str("    proc_macro_deps = all_crate_deps(\n        proc_macro = True,\n    ),\n");
    content.push_str(")\n");

    content
}

/// Convert just command to Bazel rule
fn convert_to_bazel_rule(command: &str, command_lines: &[String]) -> Result<String> {
    let mut content = String::new();

    // Add header
    content.push_str("\"\"\"Rules converted from justfile.\"\"\"\n\n");
    content.push_str("load(\"@bazel_skylib//lib:shell.bzl\", \"shell\")\n\n");

    // Create a shell script rule
    content.push_str(&format!("def _{}_impl(ctx):\n", command));
    content.push_str("    script_content = \"\"\"\n");

    // Extract the command body (skipping the first line with the command name)
    let mut command_body = Vec::new();
    for line in command_lines.iter().skip(1) {
        // Remove leading whitespace (4 spaces or tab)
        let clean_line = if line.starts_with("    ") {
            &line[4..]
        } else if line.starts_with('\t') {
            &line[1..]
        } else {
            line
        };
        command_body.push(clean_line);
    }

    // Add content as shell script
    for line in command_body {
        content.push_str(&format!("    {}\n", line));
    }

    content.push_str("    \"\"\"\n\n");

    // Create script file
    content.push_str("    script_file = ctx.actions.declare_file(ctx.label.name + \".sh\")\n");
    content
        .push_str("    ctx.actions.write(script_file, script_content, is_executable = True)\n\n");

    // Create wrapper script for execution
    content.push_str("    wrapper = ctx.actions.declare_file(ctx.label.name + \"_wrapper.sh\")\n");
    content.push_str("    ctx.actions.write(\n");
    content.push_str("        wrapper,\n");
    content.push_str("        \"\"\"#!/bin/bash\n");
    content.push_str("        set -euo pipefail\n");
    content.push_str("        exec \"$1\" \"$@\"\n");
    content.push_str("        \"\"\",\n");
    content.push_str("        is_executable = True,\n");
    content.push_str("    )\n\n");

    // Define runfiles and return structure
    content.push_str("    runfiles = ctx.runfiles(files = [script_file])\n");
    content.push_str("    return DefaultInfo(\n");
    content.push_str("        executable = wrapper,\n");
    content.push_str("        runfiles = runfiles,\n");
    content.push_str("    )\n\n");

    // Define the rule
    content.push_str(&format!("{} = rule(\n", command));
    content.push_str(&format!("    implementation = _{}_impl,\n", command));
    content.push_str("    executable = True,\n");
    content.push_str(")\n");

    Ok(content)
}
