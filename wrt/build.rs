//! Build script for the WRT crate.

use std::{
    env,
    fs,
    io,
    path::{
        Path,
        PathBuf,
    },
    process::Command,
};

const TESTSUITE_REPO_URL: &str = "https://github.com/WebAssembly/testsuite.git";
const TESTSUITE_DIR: &str = "testsuite";
const COMMIT_HASH_FILE: &str = "testsuite_commit.txt";

fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    // Get the output directory
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR environment variable not set"));
    let testsuite_path = out_dir.join(TESTSUITE_DIR);
    let commit_hash_path = out_dir.join(COMMIT_HASH_FILE);

    // Check internet connection
    let has_internet = check_internet_connection();

    // First run or needs update
    if !testsuite_path.exists() {
        // Clone repository
        if has_internet {
            println!("Cloning WebAssembly testsuite repository...");
            if let Err(e) = clone_testsuite(&testsuite_path) {
                println!("cargo:warning=Failed to clone testsuite: {e}");
                return;
            }
        } else {
            println!("cargo:warning=No internet connection. Skipping testsuite download.");
            return;
        }
    } else if has_internet {
        // Update repository if we have internet
        println!("Updating WebAssembly testsuite repository...");
        if let Err(e) = update_testsuite(&testsuite_path) {
            println!("cargo:warning=Failed to update testsuite: {e}");
            // Continue with existing version
        }
    } else {
        println!("cargo:warning=No internet connection. Using existing testsuite.");
    }

    // Get commit hash and save it
    if testsuite_path.exists() {
        match get_commit_hash(&testsuite_path) {
            Ok(hash) => {
                println!("Testsuite at commit: {hash}");
                if let Err(e) = fs::write(&commit_hash_path, &hash) {
                    println!("cargo:warning=Failed to write commit hash: {e}");
                }

                // Make the commit hash available to the test code
                println!("cargo:rustc-env=WASM_TESTSUITE_COMMIT={hash}");
            },
            Err(e) => {
                println!("cargo:warning=Failed to get commit hash: {e}");
            },
        }

        // Make the testsuite path available to the test code
        let path_str = testsuite_path.to_string_lossy();
        println!("cargo:rustc-env=WASM_TESTSUITE={path_str}");
        println!("cargo:warning=Setting WASM_TESTSUITE to {path_str}");
    }

    // Create a symbolic link in the current directory for easier access
    let workspace_testsuite = PathBuf::from("./testsuite");
    if !workspace_testsuite.exists() {
        // Remove any existing symlink if present
        drop(std::fs::remove_file(&workspace_testsuite));

        #[cfg(unix)]
        {
            use std::os::unix::fs as unix_fs;
            if let Err(e) = unix_fs::symlink(&testsuite_path, &workspace_testsuite) {
                println!("cargo:warning=Failed to create symlink: {e}");
            } else {
                println!("cargo:warning=Created symlink to testsuite at ./testsuite");
            }
        }

        #[cfg(windows)]
        {
            use std::os::windows::fs as windows_fs;
            if let Err(e) = windows_fs::symlink_dir(&testsuite_path, &workspace_testsuite) {
                println!("cargo:warning=Failed to create symlink: {}", e);
            } else {
                println!("cargo:warning=Created symlink to testsuite at ./testsuite");
            }
        }
    }
}

fn check_internet_connection() -> bool {
    let output = Command::new("ping").args(["-c", "1", "8.8.8.8"]).output();

    match output {
        Ok(output) => output.status.success(),
        Err(_) => false,
    }
}

fn clone_testsuite(path: &Path) -> io::Result<()> {
    let status = Command::new("git")
        .args([
            "clone",
            TESTSUITE_REPO_URL,
            path.to_str().expect("Path conversion failed"),
        ])
        .status()?;

    if !status.success() {
        return Err(io::Error::other(format!(
            "Failed to clone repository, exit code: {:?}",
            status.code()
        )));
    }

    Ok(())
}

fn update_testsuite(path: &Path) -> io::Result<()> {
    let status = Command::new("git")
        .args(["pull", "origin", "master"])
        .current_dir(path)
        .status()?;

    if !status.success() {
        return Err(io::Error::other(format!(
            "Failed to update repository, exit code: {:?}",
            status.code()
        )));
    }

    Ok(())
}

fn get_commit_hash(path: &Path) -> io::Result<String> {
    let output = Command::new("git").args(["rev-parse", "HEAD"]).current_dir(path).output()?;

    if !output.status.success() {
        return Err(io::Error::other(format!(
            "Failed to get commit hash, exit code: {:?}",
            output.status.code()
        )));
    }

    let hash = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Ok(hash)
}
