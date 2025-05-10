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

The `justfile` at the root of the workspace provides convenient recipes for common development tasks and running checks.

.. _dev-formatting:

Code Formatting
~~~~~~~~~~~~~~~

*   **Tool**: `rustfmt`
*   **Configuration**: `rustfmt.toml`
*   **Usage**:
    *   To format all code: ``just fmt``
    *   To check if code is formatted: ``just fmt-check`` (run by CI)

.. _dev-linting:

Linting with Clippy
~~~~~~~~~~~~~~~~~~~

*   **Tool**: `clippy`
*   **Configuration**: `[lints.clippy]` in `Cargo.toml` files.
*   **Usage**:
    *   Run clippy checks: ``just ci-clippy`` (all warnings treated as errors)
    *   Clippy is also run as part of ``just ci-main``.

.. _dev-file-checks:

Project File & Header Checks
~~~~~~~~~~~~~~~~~~~~~~~~~~~~

*   **Tool**: Custom `xtask` commands.
*   **Usage**:
    *   Check for presence of essential project files (README, LICENSE, etc.): ``just ci-check-file-presence`` or ``cargo xtask ci-checks file-presence``
    *   Check file headers (copyright, license, SPDX) and `#![forbid(unsafe_code)]`: ``just ci-check-headers`` or ``cargo xtask ci-checks headers``
    *   These are also run as part of ``just ci-main``.

.. _dev-dependency-checks:

Dependency Management & Audit
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

*   **Dependency Policy (`cargo-deny`)**:
    *   **Tool**: `cargo-deny`
    *   **Configuration**: `deny.toml`
    *   **Usage**: ``just ci-deny`` (also part of ``just ci-main``)
*   **Unused Dependencies (`cargo-udeps`)**:
    *   **Tool**: `cargo-udeps` (requires installation: `cargo install cargo-udeps --locked`)
    *   **Setup**: ``just setup-cargo-udeps`` (installs the tool)
    *   **Usage**: ``just udeps``
*   **Security Advisories (`cargo-audit`)**:
    *   **Tool**: `cargo-audit` (requires installation: `cargo install cargo-audit --locked`)
    *   **Setup**: ``just setup-cargo-audit`` (installs the tool)
    *   **Usage**: ``just audit``

.. _dev-geiger:

Unsafe Code Detection
~~~~~~~~~~~~~~~~~~~~~

*   **Tool**: `cargo-geiger`
*   **Usage**: ``just ci-geiger`` (also part of ``just ci-main``)
    This tool scans for `unsafe` Rust code usage and provides statistics.

.. _dev-spell-check:

Spell Checking
~~~~~~~~~~~~~~

*   **Tool**: `cspell` (requires installation: `npm install -g cspell`)
*   **Configuration**: `cspell.json`
*   **Setup**: ``just setup-cspell`` (provides installation instructions)
*   **Usage**: ``just spell-check``

.. _dev-testing:

Running Tests
~~~~~~~~~~~~~

*   **Unit & Integration Tests**: ``just test`` (runs `cargo test --all-targets --all-features --workspace`)
*   **Main CI Check Suite**: ``just ci-main``
    *   Includes: `default` (build), `ci-check-toolchain`, `fmt-check`, `ci-check-file-presence`, `ci-check-headers`, `ci-clippy`, `ci-deny`, `ci-geiger`, `ci-test`, `ci-doc-check`, `ci-fetch-locked`.
*   **Full CI Check Suite**: ``just ci-full``
    *   Includes everything in `ci-main` plus:

        *   `ci-miri`: Runs tests under Miri to detect undefined behavior.
        *   `ci-kani`: Runs Kani formal verification proofs.
        *   `ci-coverage`: Generates code coverage reports.
        *   (Other checks like `udeps`, `audit`, `spell-check` might be added here or to `ci-main` as per project decision - currently added to `ci.yml` jobs directly or via `ci-main` if they are part of it)

CI Pipeline Overview
--------------------

The CI pipeline (defined in `.github/workflows/ci.yml`) automates most of these checks. Key jobs include:

*   **Check**: Basic build checks.
*   **Test Suite**: Runs `just test`.
*   **Compliance Checks**: Runs `just ci-main` which covers formatting, headers, clippy, deny, geiger, file presence, tests, doc builds, and locked fetch. Also runs `just check-imports` separately.
*   **Unused Dependencies**: Runs `just udeps`.
*   **Security Audit**: Runs `just audit`.
*   **Spell Check**: Runs `just spell-check`.
*   **Docs Build Check**: Runs `just ci-doc-check`.

This ensures that code merged into the main branch adheres to the defined quality and safety standards.