// See: https://lib.rs/crates/dagger-sdk and https://github.com/dagger/dagger/blob/main/sdk/rust/crates/dagger-sdk/examples/first-pipeline/main.rs
use crate::Query;
use anyhow::{anyhow, bail, Result as AnyhowResult};
use dagger_sdk::HostDirectoryOpts;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;
use tracing::{error, info, instrument, warn};

// const MDBOOK_IMAGE: &str = "localhost/obrunsm/mdbook-extended:latest";
// const NETLIFY_IMAGE: &str = "netlify/cli:latest";

// const OFFICIAL_MDBOOK_IMAGE: &str = "peaceiris/mdbook:latest";

#[instrument(name = "docs_pipeline", skip_all, err)]
pub async fn run_docs_pipeline(
    client: &Query,
    base_path: PathBuf,
    output_dir: PathBuf,
    versions: Vec<String>,
) -> AnyhowResult<()> {
    info!(
        "Starting docs pipeline. Base path: {:?}, Output dir: {:?}, Versions: {:?}",
        base_path, output_dir, versions
    );

    if versions.is_empty() {
        bail!("No versions specified for the documentation pipeline.");
    }

    anyhow::Context::context(
        fs::create_dir_all(&output_dir),
        format!("Failed to create output directory: {:?}", output_dir),
    )?;

    let mut build_futures = Vec::new();
    for (i, version) in versions.iter().enumerate() {
        let client_clone = client.clone();
        let output_dir_clone = output_dir.clone();
        let base_path_clone = base_path.clone();
        let version_clone = version.clone();
        let is_main = i == 0;

        build_futures.push(tokio::spawn(async move {
            run_docs_version_pipeline(
                &client_clone,
                &base_path_clone,
                &output_dir_clone,
                &version_clone,
                is_main,
            )
            .await
        }));
    }

    let mut all_successful = true;
    for future in build_futures {
        match future.await {
            Ok(Ok(_)) => { /* Version processed successfully */ }
            Ok(Err(e)) => {
                error!("A documentation version failed: {:?}", e);
                all_successful = false;
            }
            Err(e) => {
                error!("A documentation task panicked: {:?}", e);
                all_successful = false;
            }
        }
    }

    if !all_successful {
        bail!("One or more documentation versions failed to build/export. See logs for details.");
    }

    info!("All documentation versions built and exported successfully.");
    Ok(())
}

#[instrument(name = "docs_version_pipeline", skip_all, fields(version = % version), err)]
async fn run_docs_version_pipeline(
    client: &Query,
    _base_path: &Path,
    output_dir: &Path,
    version: &str,
    is_main_branch: bool,
) -> AnyhowResult<()> {
    info!("Running docs pipeline for version: {}", version);
    let version_docs_path = output_dir.join(version);
    anyhow::Context::context(
        fs::create_dir_all(&version_docs_path),
        format!("Failed to create version directory for {}", version),
    )?;

    let worktree_path = anyhow::Context::context(
        TempDir::new(),
        "Failed to create temporary directory for git worktree",
    )?;
    info!("Created temporary worktree at: {:?} for version: {}", worktree_path.path(), version);

    let worktree_path_str = worktree_path
        .path()
        .to_str()
        .ok_or_else(|| anyhow!("Invalid worktree path: not valid UTF-8"))?;

    let git_checkout_cmd_args = ["worktree", "add", "--detach", worktree_path_str, version];

    info!("Running git command: git {:?}", git_checkout_cmd_args);
    let checkout_output = anyhow::Context::context(
        std::process::Command::new("git").args(&git_checkout_cmd_args).output(),
        format!("Failed to execute git worktree command for version {}", version),
    )?;

    if !checkout_output.status.success() {
        let stderr = String::from_utf8_lossy(&checkout_output.stderr);
        error!(
            "Git worktree command failed for version {}: {}\\nStdout: {}\\nStderr: {}",
            version,
            checkout_output.status,
            String::from_utf8_lossy(&checkout_output.stdout),
            stderr
        );
        bail!(
            "Git worktree add command failed with status {} for version: {}. Stderr: {}",
            checkout_output.status,
            version,
            stderr
        );
    }
    info!("Successfully checked out version {} to {:?}", version, worktree_path.path());

    let docs_src_path_buf = worktree_path.path().join("docs/source");
    let docs_src_path_in_worktree_str = docs_src_path_buf
        .to_str()
        .ok_or_else(|| anyhow!("Invalid docs source path in worktree: not valid UTF-8"))?;

    let sphinx_source_dir = client.host().directory_opts(
        docs_src_path_in_worktree_str,
        HostDirectoryOpts {
            exclude: Some(vec!["./.git", "**/target", "**/.DS_Store"]),
            include: None,
        },
    );

    let docs_container = client
        .container()
        .from("sphinxdoc/sphinx:latest")
        .with_mounted_directory("/docs_src", sphinx_source_dir)
        .with_workdir("/docs_src")
        .with_exec(vec!["pip", "install", "-r", "requirements.txt"])
        .with_exec(vec!["sphinx-build", "-b", "html", ".", "../_build/html"]);

    let exit_code = anyhow::Context::context(
        docs_container.exit_code().await,
        format!("Failed to get Sphinx build exit code for version {}", version),
    )?;

    if exit_code != 0 {
        let stderr = docs_container
            .stderr()
            .await
            .unwrap_or_else(|e| format!("Failed to get Sphinx stderr: {:?}", e));
        error!(
            "Sphinx build failed for version {} with exit code {}. Stderr: {}",
            version, exit_code, stderr
        );
        bail!(
            "Sphinx build failed for version {} with exit code {}. Stderr: {}",
            version,
            exit_code,
            stderr
        );
    }
    info!("Sphinx build successful for version: {}", version);

    let built_docs_dir = docs_container.directory("../_build/html");
    let export_path_str = version_docs_path
        .to_str()
        .ok_or_else(|| anyhow!("Invalid version_docs_path: not valid UTF-8"))?;

    anyhow::Context::context(
        built_docs_dir.export(export_path_str).await,
        format!(
            "Failed to export generated docs from container for version {} to {}",
            version, export_path_str
        ),
    )?;
    info!("Exported docs for version {} to {}", version, export_path_str);

    if is_main_branch {
        info!("Version {} is main branch, copying to root output dir: {:?}", version, output_dir);
        let main_branch_output_path_str = output_dir
            .to_str()
            .ok_or_else(|| anyhow!("Invalid root output path: not valid UTF-8"))?;
        anyhow::Context::context(
            built_docs_dir.export(main_branch_output_path_str).await,
            format!(
                "Failed to export main branch docs to root for version {} to {}",
                version, main_branch_output_path_str
            ),
        )?;
        info!(
            "Exported main branch docs for version {} to {}",
            version, main_branch_output_path_str
        );
    }

    let path_for_logging = worktree_path.path().to_path_buf();
    if let Err(e) = worktree_path.close() {
        warn!("Failed to remove temporary worktree at {:?}: {}", path_for_logging, e);
    }

    Ok(())
}

#[instrument(name = "check_docs_strict_pipeline", skip_all, err)]
pub async fn check_docs_strict_pipeline(client: &Query) -> AnyhowResult<()> {
    info!("Starting Daggerized strict documentation check pipeline...");

    let base_path = anyhow::Context::context(
        std::env::current_dir(),
        "Failed to get current directory for check_docs_strict_pipeline",
    )?;
    let mount_dir_path = base_path.clone();

    let host_dir_path_str = mount_dir_path.to_str().ok_or_else(|| {
        anyhow!("Mount directory path for Dagger is not valid UTF-8: {:?}", mount_dir_path)
    })?;

    let project_src_dir = client.host().directory_opts(
        host_dir_path_str,
        HostDirectoryOpts {
            exclude: Some(vec![
                "./.git",
                ".idea",
                ".DS_Store",
                "target",
                ".cargo/git",
                ".cargo/registry",
                "**/*.crc",
                ".vscode",
                ".zephyr-venv",
                ".zephyrproject",
                "docs/_build",
            ]),
            include: None,
        },
    );

    let requirements_path = "docs/source/requirements.txt";

    let build_cmd = vec![
        "sphinx-build",
        "-W",
        "-E",
        "-a",
        "-b",
        "html",
        "docs/source",
        "docs/_build/html_strict_check",
    ];

    let container = client
        .container()
        .from("sphinxdoc/sphinx:latest")
        .with_mounted_directory("/src", project_src_dir)
        .with_workdir("/src")
        .with_exec(vec!["pip", "install", "-r", requirements_path])
        .with_exec(build_cmd);

    let exit_code = anyhow::Context::context(
        container.exit_code().await,
        "Failed to get exit code from Sphinx strict check container",
    )?;

    if exit_code == 0 {
        info!("Sphinx strict check passed.");
        Ok(())
    } else {
        let stderr = container
            .stderr()
            .await
            .unwrap_or_else(|e| format!("Failed to fetch stderr after Sphinx failure: {:?}", e));
        error!("Sphinx strict check failed with exit code {}. Stderr: {}", exit_code, stderr);
        bail!("Sphinx strict check failed. Exit code: {}. Stderr: {}", exit_code, stderr)
    }
}

// Helper function to manage temporary git worktree removal
// Not strictly necessary with TempDir.close() but can be expanded for more robust cleanup
// fn cleanup_worktree(worktree_path: &Path, version: &str) -> AnyhowResult<()> {
//     info!(\"Cleaning up worktree for version {}: {:?}\", version, worktree_path);
//     std::process::Command::new(\"git\")
//         .args(&[\"worktree\", \"remove\", \"--force\", worktree_path.to_str().context(\"Invalid worktree path for cleanup\")?])
//         .status()
//         .context(format!(\"Failed to cleanup git worktree at {:?}\", worktree_path))
//         .and_then(|status| {
//             if status.success() {
//                 Ok(())
//             } else {
//                 Err(anyhow!(\"Git worktree remove command failed for {:?} with status {}\", worktree_path, status))
//             }
//         })?;
//     // TempDir will also attempt to remove its directory when it goes out of scope.
//     Ok(())
// }

// TODO: Add unit tests for this pipeline
// TODO: Add logic for docs coverage (if applicable/desired)

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[tokio::test]
    #[ignore] // Test requires Docker/Dagger, network access for git clone, and a valid repo_url.
    async fn test_run_docs_pipeline_main_branch_mocked_env() {
        let temp_output_dir = tempdir().unwrap();
        let output_dir = temp_output_dir.path().to_path_buf();

        let temp_base_path_dir = tempdir().unwrap();
        let base_path = temp_base_path_dir.path().to_path_buf();

        let dummy_docs_dir = base_path.join("docs").join("_build").join("html");
        fs::create_dir_all(&dummy_docs_dir).unwrap();
        let dummy_html_content = "dummy main content for test";
        fs::write(dummy_docs_dir.join("index.html"), dummy_html_content).unwrap();

        // Create a justfile that copies the pre-made dummy content
        fs::write(base_path.join("justfile"), 
            format!("docs:\n    @echo 'Building dummy docs for main branch'\n    @mkdir -p docs/_build/html\n    @cp -r {}/* docs/_build/html/", dummy_docs_dir.parent().unwrap().to_str().unwrap())
        ).unwrap();

        let versions = vec!["main".to_string()];

        // Ensure DOCKER_HOST is set if not using default, e.g., for Colima:
        // std::env::set_var("DOCKER_HOST", "unix:///Users/YOUR_USER/.colima/default/docker.sock");

        let result = run_docs_pipeline(base_path.clone(), output_dir.clone(), versions).await;

        if result.is_ok() {
            let main_output_html_path = output_dir.join("main").join("index.html");
            assert!(
                main_output_html_path.exists(),
                "Output file for main branch not found: {:?}",
                main_output_html_path
            );
            let content = fs::read_to_string(main_output_html_path).unwrap();
            assert!(content.contains(dummy_html_content), "Output HTML content mismatch");
            println!("Test test_run_docs_pipeline_main_branch_mocked_env PASSED (with Dagger)");
        } else {
            let err_msg = format!("Test test_run_docs_pipeline_main_branch_mocked_env failed (Dagger connection or build error): {:?}", result.err().unwrap());
            eprintln!("{}", err_msg);
            // For CI, we'd want this to panic to fail the test run clearly.
            panic!("{}", err_msg);
        }
    }

    // TODO: Add tests for specific versions (git clone branch) using a mock git server or a public test repo.
    // TODO: Add tests for error handling (e.g., git clone fails, Dagger export fails)
}
