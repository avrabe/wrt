=========================
Development Documentation
=========================

.. image:: ../_static/icons/safety_features.svg
   :width: 64px
   :align: right
   :alt: Development Icon

This section contains documentation for developers working on the WebAssembly Runtime.

.. contents:: On this page
   :local:
   :depth: 2

.. toctree::
   :maxdepth: 2
   :caption: Development Documentation:

   developer_tooling
   panic_documentation
   panic_registry_proposal

Overview
--------

This section provides documentation for developers working on the WebAssembly Runtime. It includes information about code organization, development practices, and guidelines for contributors.

Development Guidelines
----------------------

- Follow the Rust API Guidelines for consistent code structure
- Add comprehensive tests for all new functionality
- Document safety impacts of all panic conditions
- Use the safety mechanisms documented in the safety manual
- Adhere to the :ref:`comprehensive Rust safety checklist <comprehensive-rust-safety-checklist>` for all safety-critical code.

Contributing
------------

To contribute to the WebAssembly Runtime, please follow these steps:

1. Create a fork of the repository
2. Create a feature branch
3. Make your changes
4. Add tests for your changes
5. Update documentation as needed
6. Submit a pull request

See the [CONTRIBUTING.md](https://github.com/example/wrt/blob/main/CONTRIBUTING.md) file for more details. 