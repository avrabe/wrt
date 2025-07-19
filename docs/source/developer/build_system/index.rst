============
Build System
============

This section documents the WRT unified build system powered by cargo-wrt.

.. contents:: Table of Contents
   :local:
   :depth: 2

Overview
--------

WRT uses a modern, unified build system that consolidates all build operations:

1. **cargo-wrt**: Unified build tool providing all development commands
2. **wrt-build-core**: Core library implementing build system functionality
3. **Cargo**: Standard Rust build tool for basic compilation

The new architecture replaces the previous fragmented approach (justfile, xtask, shell scripts) with a single, AI-friendly tool.

Installation
------------

Install the unified build tool::

    cargo install --path cargo-wrt

Verify installation::

    cargo-wrt --help

Core Commands
-------------

All development tasks are available through cargo-wrt:

Available Commands
~~~~~~~~~~~~~~~~~~

Run ``cargo-wrt --help`` to see available commands::

    cargo-wrt --help

Essential commands include:

**Build Operations**:

- ``cargo-wrt build``: Build all WRT components
- ``cargo-wrt clean``: Clean build artifacts
- ``cargo-wrt wrtd``: Build WRTD daemon binaries

**Testing**:

- ``cargo-wrt test``: Run comprehensive test suite
- ``cargo-wrt test --filter pattern``: Run filtered tests
- ``cargo-wrt coverage --html``: Generate test coverage reports

**Quality Assurance**:

- ``cargo-wrt check``: Run static analysis and formatting
- ``cargo-wrt check --strict``: Run strict linting and checks
- ``cargo-wrt ci``: Run full CI pipeline locally

**Safety Verification**:

- ``cargo-wrt verify --asil d``: Run ASIL-D safety verification
- ``cargo-wrt kani-verify --asil-profile d``: Formal verification with KANI
- ``cargo-wrt verify-matrix --report``: Comprehensive build matrix verification

**Documentation**:

- ``cargo-wrt docs``: Generate documentation
- ``cargo-wrt docs --open``: Generate and open documentation
- ``cargo-wrt docs --private``: Include private items

**Development Utilities**:

- ``cargo-wrt no-std``: Verify no_std compatibility
- ``cargo-wrt simulate-ci``: Simulate CI workflow locally

**Tool Management**:

- ``cargo-wrt setup --check``: Check all tool dependencies
- ``cargo-wrt setup --install``: Install optional development tools
- ``cargo-wrt setup --all``: Complete environment setup
- ``cargo-wrt tool-versions check``: Verify tool versions against requirements
- ``cargo-wrt tool-versions generate``: Generate tool version configuration

Architecture
------------

The build system is built on three key components:

cargo-wrt CLI
~~~~~~~~~~~~~

The main command-line interface that provides:

- Unified command structure
- Consistent argument handling
- Progress reporting and logging
- Error handling and diagnostics

wrt-build-core Library
~~~~~~~~~~~~~~~~~~~~~~

The core build system library that implements:

- Workspace management
- Build orchestration
- Test execution
- Safety verification
- Documentation generation
- Coverage analysis

Build Operations
~~~~~~~~~~~~~~~~

All build operations follow a consistent pattern:

1. **Initialization**: Detect workspace and load configuration
2. **Validation**: Check prerequisites and dependencies
3. **Execution**: Run the requested operation with progress reporting
4. **Verification**: Validate results and generate reports
5. **Cleanup**: Clean up temporary files and resources

Configuration
-------------

The build system uses multiple configuration sources:

**Cargo.toml**:
  Workspace configuration, dependencies, and build profiles

**ASIL Levels**:
  Safety verification profiles (QM, A, B, C, D)

**Environment Variables**:
  CI detection, custom paths, and feature flags

**tool-versions.toml**:
  Tool version requirements and installation commands for reproducible development environments

Common Workflows
----------------

**Development Workflow**::

    # Start development
    cargo-wrt build
    cargo-wrt test
    
    # Make changes...
    
    # Verify changes
    cargo-wrt check
    cargo-wrt test --filter new_feature
    
    # Before commit
    cargo-wrt ci

**Safety-Critical Development**::

    # ASIL-D verification
    cargo-wrt verify --asil d
    cargo-wrt kani-verify --asil-profile d
    cargo-wrt verify-matrix --report
    
    # Generate compliance reports
    cargo-wrt simulate-ci --verbose

**Documentation Workflow**::

    # Generate and preview docs
    cargo-wrt docs --open
    
    # Verify documentation
    cargo-wrt docs --private
    cargo-wrt verify --detailed

Migration from Legacy System
-----------------------------

If you're migrating from the legacy build system:

**Command Mapping**:

.. list-table:: Legacy to cargo-wrt Command Mapping
   :widths: 40 40 20
   :header-rows: 1

   * - Legacy Command
     - New Command
     - Notes
   * - ``just build``
     - ``cargo-wrt build``
     - Direct replacement
   * - ``just ci-test``
     - ``cargo-wrt test``
     - Enhanced test reporting
   * - ``just ci-main``
     - ``cargo-wrt ci``
     - Comprehensive CI checks
   * - ``cargo xtask coverage``
     - ``cargo-wrt coverage --html``
     - Improved coverage reporting
   * - ``./scripts/kani-verify.sh``
     - ``cargo-wrt kani-verify``
     - Rust-based implementation
   * - ``just verify-build-matrix``
     - ``cargo-wrt verify-matrix --report``
     - Enhanced reporting

**Benefits of Migration**:

- Unified command interface
- Better error messages and diagnostics
- Consistent progress reporting
- AI-friendly architecture
- Cross-platform compatibility
- Integrated safety verification

Troubleshooting
---------------

**Common Issues**:

**Build Failures**:
  Run ``cargo-wrt build --verbose`` for detailed output

**Test Failures**:
  Use ``cargo-wrt test --nocapture`` to see test output

**Verification Issues**:
  Run ``cargo-wrt simulate-ci`` to reproduce CI environment locally

**Getting Help**:
  Use ``cargo-wrt <command> --help`` for command-specific help

For more detailed troubleshooting, see the :doc:`../troubleshooting/index` section.