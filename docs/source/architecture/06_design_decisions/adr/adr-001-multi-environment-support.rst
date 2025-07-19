=====================================
ADR-001: Multi-Environment Support
=====================================

:Date: 2024-12-15
:Status: ✅ Accepted
:Deciders: Architecture Team, Safety Engineer
:Technical Story: Support deployment from embedded systems to development machines

Context and Problem Statement
=============================

PulseEngine needs to run in diverse environments ranging from resource-constrained embedded 
systems to full development machines. Different environments have varying capabilities:

- **Development machines**: Full std library, unlimited memory, debugging tools
- **Embedded Linux/QNX**: Limited std library, constrained memory, real-time requirements  
- **RTOS systems**: No std library, static allocation only, deterministic behavior required
- **Bare metal**: Minimal runtime, custom hardware, maximum control needed

How do we support this diversity while maintaining a single codebase?

Decision Drivers
================

* **Portability**: Single codebase for easier maintenance
* **Performance**: Each environment should get optimal performance for its constraints
* **Safety**: Safety-critical deployments require deterministic behavior
* **Developer Experience**: Development and testing should be convenient
* **Binary Size**: Embedded systems require minimal binary footprint

Considered Options
==================

* **Option 1**: Multiple separate crates for each environment
* **Option 2**: Single crate with feature flags for environment selection
* **Option 3**: Runtime environment detection and adaptation
* **Option 4**: Conditional compilation with shared core

Decision Outcome
================

Chosen option: "Option 2: Single crate with feature flags", because it provides the best 
balance of maintainability, performance, and safety verification while keeping complexity manageable.

**Implementation**: Four feature-gated configurations:
- `std`: Full standard library support (development, testing)
- `no_std + alloc`: Heap allocation without std (embedded Linux, QNX)  
- `no_std`: Only static allocation with bounded collections (safety-critical RTOS)
- `bare_metal`: Minimal runtime for custom hardware

Positive Consequences
---------------------

* **Single Codebase**: Easier maintenance and testing
* **Compile-time Optimization**: Each environment gets optimal binary
* **Safety Verification**: Bounded collections enable formal verification
* **Developer Flexibility**: Can choose appropriate environment for use case
* **Clear Boundaries**: Environment constraints explicit in code

Negative Consequences
--------------------

* **Build Complexity**: Multiple configurations to test and maintain
* **Feature Combinatorics**: Risk of untested feature combinations
* **Documentation Overhead**: Must document behavior in each environment
* **API Differences**: Some APIs unavailable in constrained environments

Pros and Cons of the Options
=============================

Option 1: Multiple Separate Crates
-----------------------------------

Separate crates like `pulseengine-std`, `pulseengine-embedded`, etc.

* Good, because each crate optimized for specific environment
* Good, because clear separation of concerns
* Good, because no feature flag complexity
* Bad, because code duplication across crates
* Bad, because difficult to maintain consistency
* Bad, because testing complexity across multiple crates

Option 2: Single Crate with Feature Flags (Selected)
----------------------------------------------------

One crate with features like `std`, `alloc`, `bare-metal`.

* Good, because single codebase to maintain
* Good, because compile-time optimization for each environment
* Good, because shared core logic
* Good, because easier testing of common functionality
* Bad, because feature flag complexity
* Bad, because potential for untested combinations

Option 3: Runtime Environment Detection
---------------------------------------

Detect environment capabilities at runtime and adapt behavior.

* Good, because single binary works everywhere
* Good, because automatic adaptation
* Bad, because runtime overhead for environment detection
* Bad, because non-deterministic behavior
* Bad, because cannot optimize for specific environments
* Bad, because conflicts with safety requirements

Option 4: Conditional Compilation with Shared Core
--------------------------------------------------

Core functionality in shared module, environment-specific code separate.

* Good, because optimized per environment
* Good, because shared core reduces duplication
* Bad, because complex build system required
* Bad, because unclear ownership of functionality
* Bad, because difficult dependency management

Implementation Details
======================

**Feature Flag Strategy**:

.. code-block:: toml

   [features]
   default = ["std"]
   
   # Environment configurations (mutually exclusive)
   std = ["dep:std", "alloc"]
   no_std_alloc = ["alloc"]
   no_std = []
   bare_metal = ["no_std"]
   
   # Optional capabilities
   alloc = ["dep:linked_list_allocator"]
   safety_critical = ["no_std", "bounded_collections"]

**Memory Management Strategy**:

.. code-block:: rust

   #[cfg(feature = "std")]
   type RuntimeVec<T> = std::vec::Vec<T>;
   
   #[cfg(all(feature = "alloc", not(feature = "std")))]
   type RuntimeVec<T> = alloc::vec::Vec<T>;
   
   #[cfg(feature = "no_std")]
   type RuntimeVec<T> = BoundedVec<T, 256>;

**Testing Strategy**:
- CI tests all feature combinations
- Environment-specific integration tests
- Shared unit tests for core functionality

Links
=====

* Implements requirement REQ_PORTABILITY_001
* Related to [ADR-002](adr-002-safety-memory-management.rst) (Safety Memory Management)
* Refined by [ADR-004](adr-004-platform-abstraction.rst) (Platform Abstraction)