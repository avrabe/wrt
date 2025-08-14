============
Build System
============

.. note::
   This documentation has been moved and updated. Please see the current build system documentation at :doc:`../developer/build_system/index`.

The WRT build system has been completely redesigned around the unified cargo-wrt tool.

Quick Reference
---------------

**New Unified Commands:**

.. code-block:: bash

   # Install the build tool
   cargo install --path cargo-wrt

   # Core development commands
   cargo-wrt build        # Build all components
   cargo-wrt test         # Run tests
   cargo-wrt check        # Static analysis and formatting
   cargo-wrt ci           # Full CI pipeline

   # Safety verification
   cargo-wrt verify --asil d           # ASIL-D verification
   cargo-wrt kani-verify --asil-profile d  # Formal verification
   cargo-wrt verify-matrix --report    # Comprehensive verification

   # Documentation and coverage
   cargo-wrt docs --open               # Generate and open docs
   cargo-wrt coverage --html           # Coverage analysis

**Migration from Legacy Commands:**

.. list-table:: Legacy to cargo-wrt Command Mapping
   :widths: 40 40 20
   :header-rows: 1

   * - Legacy Command
     - New Command
     - Status
   * - ``just build``
     - ``cargo-wrt build``
     - ✅ Available
   * - ``just ci-test``
     - ``cargo-wrt test``
     - ✅ Available
   * - ``just ci-main``
     - ``cargo-wrt ci``
     - ✅ Available
   * - ``cargo xtask coverage``
     - ``cargo-wrt coverage --html``
     - ✅ Available
   * - ``./scripts/kani-verify.sh``
     - ``cargo-wrt kani-verify``
     - ✅ Available
   * - ``just verify-build-matrix``
     - ``cargo-wrt verify-matrix --report``
     - ✅ Available

For complete documentation, see :doc:`../developer/build_system/index`.