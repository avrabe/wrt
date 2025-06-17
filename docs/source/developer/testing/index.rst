=====================
Testing Documentation  
=====================

Comprehensive testing strategies and requirements for WRT development.

.. toctree::
   :maxdepth: 2

   unit_tests
   integration_tests
   wasm_test_suite
   wast_quick_reference
   coverage_reports
   formal_verification_guide
   mcdc_coverage

Testing Strategy
================

WRT employs a multi-layered testing approach:

1. **Unit Tests**: Test individual components in isolation
2. **Integration Tests**: Test component interactions
3. **WASM Test Suite**: Validate WebAssembly specification compliance
4. **Formal Verification**: Mathematical proofs using KANI (29 properties)
5. **Property Tests**: Verify system properties using formal methods
6. **Performance Tests**: Benchmark critical paths

Test Categories
===============

Unit Tests
----------

Run unit tests for all workspace crates:

.. code-block:: bash

   # All unit tests
   cargo test --workspace

   # Specific crate
   cargo test -p wrt-runtime

   # Specific test
   cargo test -p wrt-runtime test_name --nocapture

Integration Tests
-----------------

Integration tests validate cross-component behavior:

.. code-block:: bash

   # All integration tests  
   cargo test --test "*"

   # Specific integration test
   cargo test --test memory_safety_test

WASM Test Suite
---------------

Official WebAssembly specification tests:

.. code-block:: bash

   # Run spec tests
   cargo test -p wrt -- wasm_testsuite

   # Run specific spec test
   cargo test -p wrt spec_test_name

Coverage Requirements
====================

Minimum Coverage Targets
-------------------------

* **Overall**: 80% line coverage
* **Safety-critical**: 95% line coverage  
* **Core runtime**: 90% line coverage
* **Platform adapters**: 70% line coverage

Generate Coverage Reports
-------------------------

.. code-block:: bash

   # Generate coverage with xtask
   cargo xtask coverage

   # Generate coverage directly
   cargo llvm-cov --html --output-dir coverage

   # Open coverage report
   open coverage/index.html

Advanced Testing
================

Formal Verification
-------------------

KANI formal verification for mathematical proof of safety properties:

.. code-block:: bash

   # Run all formal verification (29 properties)
   cargo kani -p wrt-integration-tests --features kani

   # Run with specific ASIL profile
   ./scripts/kani-verify.sh --profile asil-c

   # Run specific proof harness
   cargo kani --harness kani_verify_memory_budget_never_exceeded

   # Check verification readiness
   ./scripts/check-kani-status.sh

   # Simulate CI workflow locally
   ./scripts/simulate-ci.sh

For complete formal verification documentation, see :doc:`../../safety/formal_verification`.

Memory Safety Testing
---------------------

Miri for undefined behavior detection:

.. code-block:: bash

   # Run under Miri
   cargo +nightly miri test

Property-Based Testing
----------------------

QuickCheck-style property tests for invariants:

.. code-block:: bash

   # Run property tests
   cargo test --features "proptest"

Test Requirements
=================

New Feature Testing
-------------------

All new features must include:

1. **Unit tests** for public APIs
2. **Error case testing** for failure modes
3. **Documentation tests** in code examples
4. **Integration tests** for cross-component features
5. **Performance benchmarks** for performance-critical code

Safety-Critical Testing
-----------------------

Safety-critical code requires:

1. **100% branch coverage**
2. **Formal verification proofs** where applicable
3. **Fault injection testing**
4. **Stress testing** under resource constraints

Test Organization
=================

Directory Structure
-------------------

.. code-block::

   tests/
   ├── integration_tests.rs      # Cross-component tests
   ├── memory_safety_tests.rs    # Memory safety validation  
   ├── wasm_testsuite.rs         # Spec compliance tests
   └── performance_tests.rs      # Benchmarks and stress tests

   crate/tests/
   ├── unit_tests.rs             # Crate-specific unit tests
   ├── property_tests.rs         # Property-based tests
   └── fixtures/                 # Test data and WASM files

Test Naming Conventions
-----------------------

* **Unit tests**: ``test_function_name_condition``
* **Integration tests**: ``test_integration_scenario``
* **Property tests**: ``prop_property_name_holds``
* **Benchmarks**: ``bench_operation_name``

Running CI Tests
================

Local CI Simulation
--------------------

.. code-block:: bash

   # Run main CI checks
   just ci-main

   # Run full CI suite
   just ci-full

   # Run specific test category
   cargo xtask ci-advanced-tests

Continuous Integration
----------------------

The CI pipeline runs:

1. **Fast checks**: Format, lint, basic tests
2. **Comprehensive tests**: Full test suite with coverage
3. **Advanced verification**: Miri, Kani, property tests
4. **Performance regression**: Benchmark comparisons

Debugging Tests
===============

Test Debugging
--------------

.. code-block:: bash

   # Run with output
   cargo test test_name -- --nocapture

   # Run single-threaded
   cargo test test_name -- --test-threads=1

   # Run with debug logging
   RUST_LOG=debug cargo test test_name

Performance Testing
===================

Benchmarks
----------

.. code-block:: bash

   # Run all benchmarks
   cargo bench

   # Run specific benchmark
   cargo bench --bench memory_benchmarks

   # Compare with baseline
   cargo bench -- --save-baseline main

Profiling
---------

.. code-block:: bash

   # Profile with perf
   cargo build --release
   perf record target/release/wrtd module.wasm
   perf report

Best Practices
==============

Test Design
-----------

1. **Test behavior, not implementation**
2. **Use descriptive test names**
3. **Test edge cases and error conditions**
4. **Minimize test dependencies**
5. **Use property-based testing for complex invariants**

Test Maintenance
----------------

1. **Keep tests simple and focused**
2. **Update tests when refactoring**
3. **Remove redundant tests**
4. **Document complex test scenarios**
5. **Review test coverage regularly**