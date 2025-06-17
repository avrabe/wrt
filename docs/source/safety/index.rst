====================
Safety Documentation
====================

.. image:: ../_static/icons/safety_features.svg
   :width: 64px
   :align: center
   :alt: Safety Features Icon

This document is the Safety Manual (SM) of the qualification material developed for safety-critical applications and systems. It provides the use constraints associated with the WebAssembly Runtime (WRT) qualification scope, in accordance with established safety standards.

.. contents:: On this page
   :local:
   :depth: 2

Safety Documentation Overview
-----------------------------

This safety documentation is organized into the following major components:

1. **Safety Guidelines**: General guidelines for using the runtime safely
2. **Safety Classification**: Unified cross-standard safety integrity levels
3. **Safety Constraints**: Specific constraints that must be followed
4. **Formal Verification**: Mathematical proof of safety properties using KANI
5. **Verification Strategies**: Approaches for verifying safety properties
6. **Safety Mechanisms**: Specific mechanisms implemented to ensure safety
7. **Safety Implementations**: How safety requirements are implemented
8. **Safety Test Cases**: Test cases that verify safety properties
9. **Performance Tuning**: Guidelines for balancing safety and performance
10. **Traceability Matrix**: Mapping from safety standards to implementations

Live Safety Verification Status
---------------------------------

.. include:: ../_generated_safety_summary.rst

Safety Requirements
-------------------

For details on specific safety requirements, see the :doc:`../requirements/safety` page.

Qualification Scope
-------------------

The WebAssembly Runtime has been developed with safety considerations appropriate for:

* Embedded systems with safety requirements
* Isolated execution environments for untrusted code
* Systems requiring deterministic resource usage
* Applications with memory safety requirements

Target Applications
~~~~~~~~~~~~~~~~~~~

The WebAssembly Runtime is designed for, but not limited to, the following applications:

* Embedded systems with mixed-criticality software
* IoT devices requiring isolation between components
* Edge computing platforms executing untrusted code
* Systems requiring predictable resource usage
* Applications requiring memory isolation between components

Safety Certification Approach
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

The safety certification approach for the WebAssembly Runtime includes:

* **Formal verification** of safety properties using KANI (29 mathematical proofs)
* **Static verification** of critical components through advanced static analysis
* **Dynamic verification** through comprehensive test suites and property testing
* **Code review** by safety experts with ISO 26262 expertise
* **Hazard and risk analysis** following automotive safety standards
* **ASIL-level verification** with four levels of verification rigor (ASIL-A through ASIL-D)

Safety-Critical Features
------------------------

Memory Safety
~~~~~~~~~~~~~

The WebAssembly Runtime implements several memory safety features to prevent out-of-bounds memory access that could corrupt system memory. All memory accesses must be validated against defined boundaries. Use SafeSlice for all memory operations to ensure bounds checking.

Resource Limitations
~~~~~~~~~~~~~~~~~~~~

Always define explicit resource limits for:

* Memory usage (pages)
* Stack depth
* Call depth
* Execution time/instruction count

Bounded Collections
~~~~~~~~~~~~~~~~~~~

When using bounded collections, always provide explicit capacity limits and handle capacity errors appropriately.

.. code-block:: rust

   // Good practice: Explicit capacity
   let stack = BoundedStack::<u32>::with_capacity(256);
   
   // Handle capacity errors
   if let Err(e) = stack.push(value) {
       if let BoundedError::CapacityExceeded { .. } = e {
           // Handle capacity overflow appropriately
           log::warn!("Stack capacity exceeded: {}", e);
           // Take recovery action
       }
   }

For more details on specific safety mechanisms, see :doc:`mechanisms`.

User Interactions
-----------------

2.1 Support Requests and Bug Reports
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

Users must report any observed failures, unexpected behaviors, or safety-related concerns through the official issue tracking system. Each report should include:

* Description of the observed behavior
* Expected behavior
* Steps to reproduce
* Environment details (hardware, OS, compiler version)
* Impact assessment on safety

2.2 Obtaining Documentation
~~~~~~~~~~~~~~~~~~~~~~~~~~~

The complete documentation, including this Safety Manual, is available through:

* Source repository documentation directory
* Generated HTML/PDF documentation
* API documentation generated from source code comments

2.3 Consulting Known Problems
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

Users should regularly check the known problems section in the official repository to stay informed about identified issues and available workarounds.

Installation Procedures
-----------------------

3.1 Installing Prerequisites
~~~~~~~~~~~~~~~~~~~~~~~~~~~~

The following prerequisites must be correctly installed before using the WebAssembly Runtime:

* Rust toolchain (minimum version 1.86.0)
* Required build dependencies as specified in the README
* For development: just command runner and python for documentation

3.2 Installing the Runtime
~~~~~~~~~~~~~~~~~~~~~~~~~~

Follow the installation instructions in the README.md file:

.. code-block:: bash

   # Install Rust (if not already installed)
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

   # Install just command runner (for development)
   cargo install just

   # Setup project dependencies
   just setup

3.3 Installation Validation
~~~~~~~~~~~~~~~~~~~~~~~~~~~

After installation, execute the validation tests to verify the installation:

.. code-block:: bash

   # Run validation tests
   just test-validation

A successful test run confirms the installation is valid.

Usage
-----

4.1 Cleaning the Build Space
~~~~~~~~~~~~~~~~~~~~~~~~~~~~

Before building for safety-critical applications, ensure a clean build environment:

.. code-block:: bash

   # Clean build artifacts
   just clean

   # Build from clean state
   just build

4.2 Warnings and Errors
~~~~~~~~~~~~~~~~~~~~~~~

All compiler warnings must be treated as errors and addressed before deployment in safety-critical applications. Use:

.. code-block:: bash

   # Build with warnings treated as errors
   RUSTFLAGS="-D warnings" just build

4.3 Building WebAssembly Modules
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

When building WebAssembly modules for use with this runtime, follow these safety guidelines:

* Use memory-safe languages where possible
* Enable all compiler safety checks
* Validate WebAssembly modules before execution
* Set appropriate resource limits

4.4 Creating Host Functions
~~~~~~~~~~~~~~~~~~~~~~~~~~~

When implementing host functions:

* Validate all inputs from WebAssembly modules
* Handle all error cases explicitly
* Implement resource limitation and monitoring
* Use the SafeMemoryAdapter for memory access

Verification Levels
-------------------

The runtime supports different verification levels for balancing safety and performance. Select the appropriate verification level based on safety criticality:

* ``VerificationLevel::Off`` - No verification checks (performance-sensitive paths)
* ``VerificationLevel::Basic`` - Basic verification checks (minimal overhead)
* ``VerificationLevel::Standard`` - Standard verification level (recommended default)
* ``VerificationLevel::Full`` - Full verification including checksums (safety-critical)
* ``VerificationLevel::Sampling`` - Probabilistic verification based on operation importance
* ``VerificationLevel::Redundant`` - Redundant checks in addition to full verification

Detailed Safety Documentation
-----------------------------

.. toctree::
   :maxdepth: 2

   safety_guidelines
   safety_classification
   constraints
   formal_verification
   verification_strategies
   mechanisms
   implementations
   test_cases
   traceability_matrix
   performance_tuning
   safety_tiers
   iso26262_compliance
   certification_validation