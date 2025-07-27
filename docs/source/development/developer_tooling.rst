====================================
Developer Tooling & Local Checks
====================================

This page provides an overview of the development tools, coding standards, and local checks configured for this project. Developers should familiarize themselves with these tools to ensure code quality, consistency, and adherence to safety guidelines.

.. contents:: On this page
   :local:
   :depth: 2

Configuration Files
-------------------

The following configuration files define standards and tool behavior across the workspace:

*   ``.editorconfig``: Ensures consistent editor settings (indentation, line endings) across different IDEs.
*   ``.gitattributes``: Enforces LF line endings and UTF-8 encoding for various file types.
*   `rust-toolchain.toml`: Pins the project to a specific Rust stable toolchain version (e.g., 1.78.0) for reproducible builds.
*   `rustfmt.toml`: Defines the code formatting rules enforced by `rustfmt`.
*   `deny.toml`: Configures `cargo-deny` for checking licenses, duplicate dependencies, security advisories, and allowed sources.
*   `cspell.json`: Contains the configuration and custom dictionary for `cspell` spell checking.
*   `Cargo.toml` (workspace and per-crate):
    *   `[profile.release]` and `[profile.test]` set `panic = "abort"`.
    *   `[lints.rust]` and `[lints.clippy]` define a strict set of allowed/denied lints. Key settings include:

        *   `rust.unsafe_code = "forbid"` (enforced by `#![forbid(unsafe_code)]` in lib/main files).
        *   `rust.missing_docs = "deny"`.
        *   `clippy::pedantic = "warn"` (most pedantic lints enabled).
        *   Many specific clippy lints are set to `deny` or `warn` (e.g., `unwrap_used`, `float_arithmetic`, `transmute_ptr_to_ref`).

Local Development Workflow & Checks
-----------------------------------

The unified `cargo-wrt` build tool provides convenient commands for common development tasks and running checks.

.. _dev-formatting:

Code Formatting
~~~~~~~~~~~~~~~

*   **Tool**: `rustfmt`
*   **Configuration**: `rustfmt.toml`
*   **Usage**:
    *   To format and check all code: ``cargo-wrt check`` (includes formatting)
    *   To check formatting only: ``cargo fmt --check`` (if needed separately)

.. _dev-linting:

Linting with Clippy
~~~~~~~~~~~~~~~~~~~

*   **Tool**: `clippy`
*   **Configuration**: `[lints.clippy]` in `Cargo.toml` files.
*   **Usage**:
    *   Run clippy checks: ``cargo-wrt check`` (all warnings treated as errors)
    *   Clippy is also run as part of ``cargo-wrt ci``.

.. _dev-file-checks:

Project File & Header Checks
~~~~~~~~~~~~~~~~~~~~~~~~~~~~

*   **Tool**: Integrated into `cargo-wrt`.
*   **Usage**:
    *   All file and header checks are integrated into: ``cargo-wrt ci``
    *   Includes checking for essential project files, file headers, copyright, license, SPDX, and ``#![forbid(unsafe_code)]``

.. _dev-dependency-checks:

Dependency Management & Audit
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

*   **Dependency Policy & Security**:
    *   **Tools**: `cargo-deny`, `cargo-udeps`, `cargo-audit` (integrated into cargo-wrt)
    *   **Configuration**: `deny.toml`
    *   **Usage**: ``cargo-wrt ci`` (includes dependency policy, unused deps, and security audit)
    *   **Strict checks**: ``cargo-wrt check --strict`` (additional dependency analysis)

.. _dev-geiger:

Unsafe Code Detection
~~~~~~~~~~~~~~~~~~~~~

*   **Tool**: `cargo-geiger` (integrated into cargo-wrt)
*   **Usage**: ``cargo-wrt ci`` (includes unsafe code detection)
    This tool scans for `unsafe` Rust code usage and provides statistics.

.. _dev-spell-check:

Spell Checking
~~~~~~~~~~~~~~

*   **Tool**: `cspell` (requires installation: `npm install -g cspell`)
*   **Configuration**: `cspell.json`
*   **Usage**: ``cargo-wrt ci`` (includes spell checking if cspell is available)
*   **External setup**: Install cspell manually with `npm install -g cspell`

.. _dev-testing:

Running Tests
~~~~~~~~~~~~~

*   **Unit & Integration Tests**: ``cargo-wrt test`` (runs comprehensive test suite)
*   **Main CI Check Suite**: ``cargo-wrt ci``
    *   Includes: build, toolchain checks, formatting, linting, file/header checks, dependency policy, unsafe code detection, tests, documentation, and more.
*   **Additional Test Options**:

        *   ``cargo-wrt test --miri``: Runs tests under Miri to detect undefined behavior.
        *   ``cargo-wrt kani-verify``: Runs Kani formal verification proofs.
        *   ``cargo-wrt coverage``: Generates code coverage reports.
        *   ``cargo-wrt verify-matrix``: Comprehensive build matrix verification.

CI Pipeline Overview
--------------------

The CI pipeline (defined in `.github/workflows/ci.yml`) automates most of these checks using the unified `cargo-wrt` build system. Key jobs include:

*   **Build & Test**: Runs ``cargo-wrt build`` and ``cargo-wrt test``.
*   **Comprehensive CI**: Runs ``cargo-wrt ci`` which covers:
    *   Code formatting and linting
    *   File and header validation
    *   Dependency policy and security audits
    *   Unsafe code detection
    *   Documentation builds
    *   Test execution
*   **Formal Verification**: Runs ``cargo-wrt kani-verify`` for safety-critical verification.
*   **Build Matrix**: Runs ``cargo-wrt verify-matrix`` for comprehensive configuration testing.

This unified approach ensures that code merged into the main branch adheres to the defined quality and safety standards while providing a consistent development experience.