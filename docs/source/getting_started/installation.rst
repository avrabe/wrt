============
Installation
============

This page provides detailed installation instructions for PulseEngine (WRT Edition) across all supported platforms.

.. warning::
   **Source-Only Installation**: PulseEngine is currently available only as source code. 
   Pre-built binaries and package manager distributions are not yet available.

.. contents:: On this page
   :local:
   :depth: 2

System Requirements
===================

Hardware Requirements
---------------------

**Minimum:**

* 64 MB RAM (for basic embedded usage)
* 10 MB storage for runtime
* Any 32-bit or 64-bit processor with WebAssembly support

**Recommended for Development:**

* 4 GB RAM
* 1 GB storage for full toolchain
* x86_64 or ARM64 processor

Software Dependencies
=====================

Core Dependencies
-----------------

All platforms require:

1. **Rust Toolchain**: Version 1.86.0 or newer (stable)
2. **just**: Build automation tool
3. **Git**: Source code management

Install Rust
~~~~~~~~~~~~~

.. tabs::

   .. tab:: Linux/macOS

      .. code-block:: bash

         curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
         source ~/.cargo/env

   .. tab:: Windows

      Download and run the installer from `rustup.rs <https://rustup.rs/>`_

   .. tab:: Manual

      Download from the `official Rust website <https://forge.rust-lang.org/infra/channel-layout.html#archives>`_

Install just
~~~~~~~~~~~~

.. code-block:: bash

   cargo install just

Verify installation:

.. code-block:: bash

   just --version
   rustc --version

WebAssembly Targets
~~~~~~~~~~~~~~~~~~~

WRT requires WebAssembly compilation targets:

.. code-block:: bash

   rustup target add wasm32-unknown-unknown
   rustup target add wasm32-wasip1
   rustup target add wasm32-wasip2

Additional Development Tools
----------------------------

For full development workflow:

.. code-block:: bash

   # Component tooling
   cargo install cargo-component

   # WebAssembly tools
   cargo install wasm-tools
   
   # PulseEngine command-line interface (from source)
   cargo install --path wrtd

   # Code coverage (optional)
   cargo install cargo-llvm-cov

Installation Methods
====================

Source Installation
-------------------

**Recommended for most users**

1. Clone the repository:

   .. code-block:: bash

      git clone https://github.com/pulseengine/wrt
      cd wrt

2. Build from source:

   .. code-block:: bash

      just build

3. Run tests to verify:

   .. code-block:: bash

      just ci-test

4. (Optional) Install system-wide:

   .. code-block:: bash

      cargo install --path wrtd

Binary Installation
-------------------

.. warning::
   **Not Available**: Pre-built binaries are not currently available. 
   Please use source installation method above.

Package Manager Installation
----------------------------

.. warning::
   **Not Available**: PulseEngine is not currently published to package managers including:
   
   - crates.io (Cargo)
   - Homebrew
   - APT repositories
   - Other package managers
   
   Please use source installation method above.

Configuration
=============

Environment Variables
---------------------

Set these for optimal performance:

.. code-block:: bash

   # Runtime configuration
   export WRT_STACK_SIZE=1048576
   export WRT_FUEL_LIMIT=1000000

   # Development options
   export WRT_LOG_LEVEL=info
   export WRT_DEBUG_MODE=1

Build Configuration
-------------------

Create a ``.wrt/config.toml`` file in your project:

.. code-block:: toml

   [runtime]
   stack_size = 1048576
   fuel_limit = 1000000
   
   [security]
   enable_cfi = true
   sandbox_memory = true
   
   [performance]
   optimize_for_size = false
   enable_simd = true

Verification
============

Verify your installation works correctly:

.. code-block:: bash

   # Check PulseEngine installation (if installed from source)
   cargo run --bin wrtd -- --version

   # Build and run example
   just test-wrtd-example

   # Run comprehensive tests
   just ci-main

Expected output should show successful compilation and test execution.

Troubleshooting
===============

Common Issues
-------------

**Rust version mismatch:**

.. code-block:: bash

   rustup update stable
   rustup default stable

**Missing WebAssembly targets:**

.. code-block:: bash

   just setup-rust-targets

**Build failures:**

.. code-block:: bash

   cargo clean
   just build

**Permission errors:**

.. code-block:: bash

   # Use cargo install without sudo
   cargo install --path wrtd

Platform-Specific Notes
=======================

For detailed platform-specific instructions, see:

* :doc:`linux` - Linux distributions
* :doc:`macos` - macOS and Apple Silicon
* :doc:`qnx` - QNX Neutrino real-time systems
* :doc:`zephyr` - Zephyr RTOS embedded systems
* :doc:`bare_metal` - Bare-metal and custom hardware

Next Steps
==========

After installation:

1. Try the :doc:`../examples/hello_world` example
2. Read the :doc:`../architecture/index` overview
3. Explore :doc:`../examples/index` for your use case