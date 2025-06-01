===============
Getting Started
===============

.. image:: ../_static/icons/logo.svg
   :width: 80px
   :align: right
   :alt: WRT Logo

Welcome to WRT (WebAssembly Runtime)! This guide will help you install and set up WRT on various platforms, from development environments to embedded systems.

.. contents:: On this page
   :local:
   :depth: 2

Prerequisites
=============

WRT requires the following to build and run:

* **Rust**: Version 1.86.0 or newer (stable channel)
* **just**: Command runner for build automation
* **Git**: For source code management
* **Platform-specific tools**: Varies by target platform

Quick Start
===========

For most development scenarios, follow these steps:

1. **Install Rust** (if not already installed):

   .. code-block:: bash

      curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
      source ~/.cargo/env

2. **Install just** (build tool):

   .. code-block:: bash

      cargo install just

3. **Clone the repository**:

   .. code-block:: bash

      git clone https://github.com/pulseengine/wrt.git
      cd wrt

4. **Build and test**:

   .. code-block:: bash

      just build
      just ci-test

Supported Platforms
===================

WRT supports a wide range of platforms, from development machines to embedded systems:

.. grid:: 2

   .. grid-item-card:: Desktop Development
      :link: ../platform_guides/linux
      :link-type: doc

      * Linux (x86_64, ARM64)
      * macOS (Intel, Apple Silicon)
      * Complete toolchain and debugging support

   .. grid-item-card:: Real-Time Systems
      :link: ../platform_guides/qnx
      :link-type: doc

      * QNX Neutrino
      * Safety-critical automotive/medical
      * POSIX compliance with RT extensions

   .. grid-item-card:: Embedded RTOS
      :link: ../platform_guides/zephyr
      :link-type: doc

      * Zephyr RTOS
      * IoT and edge computing
      * Minimal resource footprint

   .. grid-item-card:: Bare Metal
      :link: ../platform_guides/bare_metal
      :link-type: doc

      * No operating system
      * Custom hardware platforms
      * Maximum control and performance

Basic Usage
===========

Once installed, you can use WRT in several ways:

Command Line Interface
----------------------

Run WebAssembly modules directly:

.. code-block:: bash

   # Run a simple module
   just test-wrtd-example

   # Run with custom parameters
   just test-wrtd-example "--fuel 1000 --stats"

Library Integration
-------------------

Add WRT to your Rust project:

.. code-block:: toml

   [dependencies]
   wrt = { path = "wrt" }  # Adjust path or use published version

Basic runtime usage:

.. code-block:: rust

   use wrt::prelude::*;

   // Load and execute WebAssembly
   let module = Module::from_bytes(&wasm_bytes)?;
   let mut instance = ModuleInstance::new(module, imports)?;
   let result = instance.invoke("function_name", &args)?;

Component Model
---------------

Work with WebAssembly components:

.. code-block:: rust

   use wrt_component::prelude::*;

   // Load component with WIT interface
   let component = Component::from_bytes(&component_bytes)?;
   let instance = ComponentInstance::new(component, imports)?;

Next Steps
==========

.. grid:: 3

   .. grid-item-card:: 📖 Examples
      :link: ../examples/index
      :link-type: doc

      Learn through hands-on examples from Hello World to advanced component usage.

   .. grid-item-card:: 🏗️ Architecture
      :link: ../architecture/index
      :link-type: doc

      Understand WRT's design, safety features, and performance characteristics.

   .. grid-item-card:: 🔧 Development
      :link: ../developer/index
      :link-type: doc

      Contributing guidelines, testing, and advanced development topics.

Need Help?
==========

* **Documentation**: Browse the complete documentation for detailed guides
* **Examples**: Check the ``example/`` directory for working code samples
* **Issues**: Report bugs or request features in the project repository
* **Platform Support**: Refer to platform-specific installation guides for detailed setup instructions