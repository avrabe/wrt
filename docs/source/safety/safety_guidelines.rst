WebAssembly Runtime Safety Guidelines
=====================================

Introduction
------------

This document provides guidelines for safely utilizing the bounded collections and memory safety features
in the WebAssembly Runtime (WRT). These guidelines are intended to ensure that applications meet safety
requirements and conform to the design principles established in the Functional Safety Plan.

Bounded Collections
-------------------

Usage Guidelines
~~~~~~~~~~~~~~~~

1. **Always specify capacity limits**:
   
   * When creating bounded collections, always provide explicit capacity limits.
   * Do not use defaults unless you have verified they are appropriate for your use case.

   .. code-block:: rust

      // Good practice: Explicit capacity
      let stack = BoundedStack::<u32>::with_capacity(256);
      
      // Avoid: Using defaults without consideration
      let stack = BoundedStack::<u32>::default();

2. **Handle capacity errors**:
   
   * Always check return values for push operations.
   * Implement appropriate error handling for capacity overflows.

   .. code-block:: rust

      // Good practice: Error handling
      if let Err(e) = stack.push(value) {
          if let BoundedError::CapacityExceeded { .. } = e {
              // Handle capacity overflow appropriately
              log::warn!("Stack capacity exceeded: {}", e);
              // Take recovery action
          }
      }

3. **Verification levels**:
   
   * Use ``VerificationLevel::Full`` for critical operations where safety is paramount.
   * Use ``VerificationLevel::Standard`` for normal operations.
   * Use ``VerificationLevel::Sampling`` for performance-critical paths that still need some validation.
   * Only use ``VerificationLevel::None`` when maximum performance is required and safety is ensured by other means.

4. **Regular validation**:
   
   * Periodically call ``validate()`` on bounded collections to ensure integrity.
   * Consider validating after complex operation sequences.

   .. code-block:: rust

      // Validate after a sequence of operations
      stack.push(value)?;
      process_data(&mut stack);
      stack.validate()?; // Validates the stack integrity

Safe Memory Operations
----------------------

Safety Guidelines
~~~~~~~~~~~~~~~~~

1. **Bounds checking**:
   
   * Always use SafeSlice for memory access to ensure bounds checking.
   * Verify that memory operations stay within allocated bounds.

   .. code-block:: rust

      // Good practice: Using SafeSlice for bounds-checked access
      let safe_slice = SafeSlice::new(memory_buffer);
      safe_slice.copy_from_slice(offset, &data)?;

2. **Checksumming**:
   
   * Enable checksumming for critical memory regions.
   * Validate checksums before and after significant operations.

   .. code-block:: rust

      // Validate checksum before critical operations
      safe_memory.validate_checksum()?;
      perform_critical_operation(&safe_memory);
      safe_memory.validate_checksum()?;

3. **Verification levels for memory**:
   
   * Consider memory safety requirements when selecting verification levels.
   * Use ``VerificationLevel::Full`` when processing untrusted data.
   * Use ``VerificationLevel::Standard`` for most operations.

4. **Memory adapters**:
   
   * Use SafeMemoryAdapter when interfacing with WebAssembly memory.
   * Configure adapters with appropriate verification levels based on context.

   .. code-block:: rust

      // Create adapter with appropriate verification level
      let adapter = SafeMemoryAdapter::with_verification_level(
          memory.clone(),
          VerificationLevel::Standard
      );

Engine Integration
------------------

Safety Best Practices
~~~~~~~~~~~~~~~~~~~~~

1. **Validation checkpoints**:
   
   * Add validation checkpoints at critical execution stages.
   * Validate state before and after significant control flow changes.

   .. code-block:: rust

      // Validate engine state at critical points
      engine.validate()?;
      execute_wasm_function(...)?;
      engine.validate()?;

2. **Error handling strategy**:
   
   * Implement graceful error handling for safety violations.
   * Consider safe fallback strategies for critical applications.

   .. code-block:: rust

      match result {
          Ok(value) => process_value(value),
          Err(Error::SafetyViolation(e)) => {
              log::error!("Safety violation detected: {}", e);
              // Implement fallback or recovery
              recovery_action();
          }
          Err(e) => handle_other_error(e),
      }

3. **Operation tracking**:
   
   * Enable operation tracking for critical applications.
   * Monitor operation statistics to detect anomalies.

   .. code-block:: rust

      // Check operation stats for anomalies
      let stats = engine.execution_stats();
      if stats.memory_operations > MEMORY_OP_THRESHOLD {
          log::warn!("Excessive memory operations detected");
      }

Fuzzing and Testing
-------------------

Validation Guidelines
~~~~~~~~~~~~~~~~~~~~~

1. **Use provided fuzzers**:
   
   * Run the fuzzing infrastructure regularly to identify issues.
   * Use specific fuzzers for different collection types.

   .. code-block:: bash

      # Run fuzzers for different components
      cargo fuzz run fuzz_safe_slice
      cargo fuzz run fuzz_bounded_vec
      cargo fuzz run fuzz_bounded_stack
      cargo fuzz run fuzz_memory_adapter

2. **Validation tests**:
   
   * Implement validation tests for your specific use cases.
   * Test with different verification levels to understand tradeoffs.

3. **Benchmarking**:
   
   * Run performance benchmarks to measure the impact of safety features.
   * Use results to select appropriate verification levels.

   .. code-block:: bash

      # Run benchmarks to measure performance impact
      cargo bench --bench safe_memory_benchmarks

Performance Considerations
--------------------------

1. **Verification level selection**:
   
   * Balance safety and performance requirements.
   * Use the verification level selection guide to choose appropriate settings.

2. **Caching and preprocessing**:
   
   * When using higher verification levels, consider preprocessing or caching results.
   * Document performance impacts in your design.

Error Handling and Panic Conditions
-----------------------------------

1. **Error propagation**:
   
   * Prefer using `Result` types for error handling instead of panic.
   * Implement appropriate error recovery mechanisms.

2. **Panic conditions**:
   
   * Document all panic conditions according to the :doc:`../development/panic_documentation`.
   * Always provide a safety impact assessment for panic points.

3. **Containment**:
   
   * Design components to contain failures within their boundaries.
   * Consider using catch_unwind in safety-critical boundaries.

Certification and Compliance
----------------------------

1. **Documentation**:
   
   * Document safety features and verification strategies.
   * Maintain evidence of safety verification for certification.

2. **Traceability**:
   
   * Ensure requirements traceability to safety features.
   * Document safety case evidence for compliance.

3. **Verification**:
   
   * Implement regular verification processes.
   * Consider formal verification for critical components.

.. _comprehensive-rust-safety-checklist:

Comprehensive Rust Safety Checklist
-----------------------------------

Use this checklist when generating or reviewing Rust code for safety-critical projects. Each rule must pass its automated or manual check before the change can be merged. Many of these rules are enforced automatically by the CI system (see :doc:`../development/developer_tooling` and :doc:`../safety_mechanisms`). Specific constraints are detailed in :doc:`constraints`.

**0. Meta**

*   **Rule**: File banner includes project, module, SW-REQ-ID, copyright, license and SPDX identifiers. SW requirements as in the ``docs/source`` directory.
    *   **Check**: CI header-regex linter (``xtask ci-checks headers``) or manual review. See :ref:`dev-file-checks`.
*   **Rule**: Source file is UTF-8 with POSIX (``\\n``) newlines.
    *   **Check**: ``.gitattributes`` + CI encoding/newline checker.

**1. Language Subset**

*   **Rule**: Stable channel only – no ``#![feature]`` or nightly toolchains.
    *   **Check**: CI verifies ``rustup show active-toolchain`` is ``stable`` or ferrocene-xx.
*   **Rule**: No inline assembly (``asm!``, ``global_asm!``) in safety code.
    *   **Check**: Clippy lint ``clippy::inline_asm_x86_att_syntax``.
*   **Rule**: No proc-macros or build-time code-gen in safety crates.
    *   **Check**: ``cargo deny`` ban list + ``cargo tree diff``. See :ref:`dev-dependency-checks`.
*   **Rule**: Raw pointers & pointer casts only inside reviewed HAL crate.
    *   **Check**: ``#![deny(pointer_cast)]`` + crate ownership check.
*   **Rule**: No reflection (``std::any::TypeId``, ``core::mem::transmute``).
    *   **Check**: Clippy lints ``transmute_ptr_to_ref``, ``type_id``. See :ref:`dev-linting`.
*   **Rule**: No dynamic dispatch (``Box<dyn Trait>``) in hard-RT paths.
    *   **Check**: Clippy lint ``clippy::dyn_trait`` + architectural review.
*   **Rule**: No floating-point in fixed-point control loops.
    *   **Check**: Clippy lint ``clippy::float_arithmetic`` + unit tests. See :ref:`dev-linting`.

**2. Unsafe Usage**

*   **Rule**: ``#![forbid(unsafe_code)]`` in every crate except those tagged ``hal`` or ``ffi``.
    *   **Check**: CI grep for attribute + crate manifest tag. See :ref:`dev-file-checks` and :ref:`dev-linting`. Also see constraint :ref:`CNST_UNSAFE_FORBID`.
*   **Rule**: In ``hal``/``ffi`` crates: each ``unsafe`` block ≤ 10 LOC and wrapped by a safe API.
    *   **Check**: ``cargo geiger --sarif`` + LOC-count script. See :ref:`dev-geiger`.
*   **Rule**: Every ``unsafe`` wrapper has a ``/// # Safety`` doc explaining invariants.
    *   **Check**: Regex linter for ``Safety`` section before ``unsafe``. See constraint :ref:`CNST_UNSAFE_REVIEW`.
*   **Rule**: Invariant covered by unit or property test.
    *   **Check**: CI executes ``cargo test`` + coverage threshold.
*   **Rule**: External symbols declared with ``extern "C"`` documented in ``README-SAFETY.md``.
    *   **Check**: Documentation audit script (``docs/documentation_audit.sh``).
*   **Rule**: ``cargo geiger -D warnings`` passes; unsafe count does not grow in non-HAL crates.
    *   **Check**: CI step (``just ci-geiger``). See :ref:`dev-geiger`.
*   **Rule**: Pointer arithmetic uses checked ops (``checked_add``, ``checked_sub``).
    *   **Check**: Clippy lint ``ptr_offset_with_cast``.

**3. Error Handling & Panics**

*   **Rule**: ``panic = "abort"`` set for release and test profiles.
    *   **Check**: Grep ``Cargo.toml``. See :doc:`../development/developer_tooling`. Also see constraint :ref:`CNST_PANIC_ABORT`.
*   **Rule**: No ``unwrap``, ``expect``, ``panic!``, ``todo!``, ``unimplemented!`` in safety code.
    *   **Check**: Clippy lint ``clippy::unwrap_used``, ``clippy::panic``. See :ref:`dev-linting`.
*   **Rule**: All fallible calls return ``Result<T,E>`` with domain-specific error.
    *   **Check**: Code review + Clippy lint ``clippy::result_unit_err``.
*   **Rule**: Errors propagated with ``?``; ignored ``Err`` prohibited.
    *   **Check**: Clippy lints ``let_underscore_drop``, ``must_use``.
*   **Rule**: Custom panic hook logs location & triggers watchdog reset. See :doc:`../development/panic_documentation`.
    *   **Check**: Integration test verifies hook behaviour. See constraint :ref:`CNST_PANIC_HANDLE`.

**4. Control-Flow Soundness**

*   **Rule**: All ``match`` over enums are exhaustive – no ``_`` wildcards.
    *   **Check**: Compiler + Clippy lint ``match_wildcard_for_single_variants``.
*   **Rule**: No ``loop { … break … }`` as pseudo-while.
    *   **Check**: Clippy lint ``never_loop``.
*   **Rule**: Recursion bounded and depth justified.
    *   **Check**: Code review + unit test. See constraint :ref:`CNST_RECURSION_BOUND`.
*   **Rule**: Cyclomatic complexity per function ≤ 10.
    *   **Check**: ``cargo llvm-cov --show-functions`` + script.
*   **Rule**: No ``core::hint::unreachable_unchecked()``.
    *   **Check**: Grep or Clippy deny.
*   **Rule**: ``#![deny(clippy::panic, clippy::unreachable)]`` enabled.
    *   **Check**: CI compile step.

**5. Memory & Concurrency Safety**

*   **Rule**: Heap-free in critical path (alloc banned or ``heapless``/``arrayvec`` only). See constraint :ref:`CNST_NO_ALLOC`.
    *   **Check**: Compile with custom lint ``#![deny(alloc_instead_of_core)]`` or check dependencies.
*   **Rule**: No ``static mut``.
    *   **Check**: Compiler forbid + Clippy lint ``static_mut_reference``. See constraint :ref:`CNST_NO_STATIC_MUT`.
*   **Rule**: Shared data protected by ``Atomic*`` or RTOS ``Mutex`` with priority ceiling. See constraint :ref:`CNST_CONCURRENCY_SAFE`.
    *   **Check**: Code review + static analysis.
*   **Rule**: Manual ``unsafe impl Send/Sync`` reviewed & tested.
    *   **Check**: Pattern match + mandatory peer review.
*   **Rule**: ``cargo miri test`` passes (no UB or data races).
    *   **Check**: CI step (part of ``just ci-full``). See :doc:`../safety_mechanisms`.
*   **Rule**: No ``Arc::get_mut_unchecked``.
    *   **Check**: Clippy lint ``arc_mutate``.

**6. Determinism & Timing**

*   **Rule**: ``#[inline(always)]`` used only when justified.
    *   **Check**: Code review.
*   **Rule**: No ``std::thread::sleep`` or blocking sleeps in safety code. See constraint :ref:`CNST_NO_BLOCKING`.
    *   **Check**: Grep.
*   **Rule**: No non-crypto RNG (``rand``) in deterministic algorithms. See constraint :ref:`CNST_DETERMINISM`.
    *   **Check**: Grep + ``cargo deny`` ban list.
*   **Rule**: Build flags ``-C target-cpu=... -C target-feature=+strict-align`` set. See constraint :ref:`CNST_BUILD_FLAGS`.
    *   **Check**: Inspect CI build logs.

**7. Build Reproducibility**

*   **Rule**: ``rust-toolchain.toml`` pins exact toolchain version. See constraint :ref:`CNST_TOOLCHAIN_PIN`.
    *   **Check**: Presence + version regex. See :doc:`../development/developer_tooling`.
*   **Rule**: ``cargo fetch --locked`` succeeds offline.
    *   **Check**: CI offline build test.
*   **Rule**: Binary embeds SBOM via ``cargo auditable``.
    *   **Check**: CI verifies ``.note.package.metadata``.

**8. Static Analysis Gates (CI)**

*   **Rule**: ``cargo clippy --all-targets -D warnings -W clippy::pedantic`` passes.
    *   **Check**: CI clippy step (``just ci-clippy``). See :ref:`dev-linting`.
*   **Rule**: ``cargo deny check`` passes.
    *   **Check**: CI step (``just ci-deny``). See :ref:`dev-dependency-checks`.
*   **Rule**: All ``#[kani::proof]`` functions succeed.
    *   **Check**: CI runs ``cargo kani --ci`` (part of ``just ci-full``). See :doc:`../safety_mechanisms`.
*   **Rule**: ``cargo llvm-cov --mcdc`` ≥ 90 % on safety crates.
    *   **Check**: CI coverage threshold. See constraint :ref:`CNST_TEST_COVERAGE`.

**9. Documentation**

*   **Rule**: Public items have rustdoc with Purpose, Inputs, Outputs, Safety.
    *   **Check**: ``#![deny(missing_docs)]`` + CI build. See :doc:`../development/developer_tooling`.
*   **Rule**: Runtime invariants asserted with ``debug_assert!``.
    *   **Check**: Code review + Clippy lint ``debug_assert_with_mut_call``.

Conclusion
----------

Following these guidelines will help ensure the safe use of bounded collections and memory safety features
in the WebAssembly Runtime. By appropriately handling capacity limits, implementing proper error handling,
and selecting suitable verification levels, applications can achieve both safety and performance.

Regular validation, testing with the provided fuzzing infrastructure, and performance benchmarking
are essential practices for maintaining safety throughout the development lifecycle. 