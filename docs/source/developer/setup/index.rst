=======================
Development Environment
=======================

This guide covers setting up a complete development environment for contributing to WRT.

.. toctree::
   :maxdepth: 2

   environment_setup
   toolchain_requirements
   ide_setup
   debugging_setup

Prerequisites
=============

For basic prerequisites and installation instructions, see the :doc:`/getting_started/installation` guide.

Additional Development Requirements
-----------------------------------

* **Docker or Podman** (for Dagger CI)
* **4GB RAM minimum** (8GB recommended for development)
* **Python 3.8+** (for documentation generation)

Platform Support
-----------------

* **Linux**: Primary development platform
* **macOS**: Full support with native tools
* **Windows**: WSL2 recommended

Development Tools
=================

Beyond the basic installation requirements, developers need additional tools:

.. code-block:: bash

   # Install Dagger (CI tool)
   curl -fsSL https://dl.dagger.io/dagger/install.sh | sh

   # Code quality tools
   cargo install cargo-llvm-cov
   cargo install cargo-deny

   # Documentation dependencies
   pip install -r docs/requirements.txt

   # Optional: Additional development utilities
   cargo install cargo-watch  # For automatic rebuilds
   cargo install cargo-expand # For macro expansion debugging

Quick Setup
===========

.. code-block:: bash

   # Clone repository
   git clone https://github.com/pulseengine/wrt.git
   cd wrt

   # Verify build
   just build

   # Run tests
   just ci-test

   # Format code
   just fmt

Next Steps
==========

* See :doc:`ide_setup` for editor configuration
* See :doc:`../contributing/index` for contribution guidelines
* See :doc:`../build_system/index` for build system details