====================
WRT Safety Tiers
====================

.. image:: ../_static/icons/safety_features.svg
   :width: 64px
   :align: center
   :alt: Safety Tiers Icon

WRT implements a sophisticated multi-tier safety architecture supporting the full spectrum from non-safety-critical development (QM) to highest safety-critical embedded systems (ASIL-D).

.. contents:: On this page
   :local:
   :depth: 2

Safety Philosophy
-----------------

WRT's safety architecture is based on three core principles:

1. **Graduated Safety**: Different components require different safety levels
2. **Memory Determinism**: Higher safety levels require more deterministic memory behavior  
3. **Compile-Time Enforcement**: Safety violations should be caught at compile time when possible

Safety Tier Structure
---------------------

Tier 1: QM (Quality Management) - Non-Safety
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

**Purpose**: Development tools, testing, non-critical components

**Configuration**:

.. code-block:: toml

   [dependencies]
   wrt = { version = "*", features = ["std"] }

**Characteristics**:

- Dynamic memory allocation permitted (``Vec::new()``, ``HashMap::new()``)
- Standard library available
- No memory bounds enforcement
- Suitable for: CLI tools, test harnesses, development utilities

**Example Usage**:

.. code-block:: rust

   // QM-level component - unrestricted allocation
   pub fn create_development_tool() -> Result<Tool> {
       let mut data = Vec::new();  // ✅ Allowed for QM
       let mut cache = HashMap::new();  // ✅ Allowed for QM
       
       // No safety context required
       Ok(Tool { data, cache })
   }

Tier 2: ASIL-A/B (Safety-Critical with Standard Library)
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

**Purpose**: Production systems with safety requirements but not resource-constrained

**Configuration**:

.. code-block:: toml

   [dependencies]
   wrt = { version = "*", features = ["std", "safety-asil-b"] }

**Characteristics**:

- Bounded collections with runtime capacity enforcement
- Standard library available but monitored
- Memory budget warnings and monitoring
- Suitable for: Desktop applications, server systems, development environments

**Example Usage**:

.. code-block:: rust

   use wrt_foundation::{BoundedVec, SafetyContext, SafetyLevel};
   
   pub fn create_asil_b_component() -> Result<Component> {
       let safety_ctx = SafetyContext::new(SafetyLevel::ASIL_B)?;
       
       // Bounded collections with runtime enforcement
       let mut data = BoundedVec::<Data, 1000>::new()?;
       let mut cache = BoundedMap::<Key, Value, 100>::new()?;
       
       Ok(Component::new(data, cache, safety_ctx))
   }

Tier 3: ASIL-C (Safety-Critical with Allocation Budgets)
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

**Purpose**: Safety-critical embedded systems with deterministic memory requirements

**Configuration**:

.. code-block:: toml

   [dependencies]
   wrt = { version = "*", features = ["safety-asil-c"], default-features = false }

**Characteristics**:

- No dynamic allocation after initialization
- Compile-time memory budgets enforced
- All collections have compile-time capacity limits
- Formal verification of memory properties
- Suitable for: Automotive ECUs, industrial controllers, embedded safety systems

**Example Usage**:

.. code-block:: rust

   use wrt_foundation::{BoundedVec, NoStdProvider, SafetyLevel, safe_managed_alloc};
   
   const MAX_ITEMS: usize = 256;
   type ComponentProvider = NoStdProvider<{MAX_ITEMS * 64}>;
   
   pub fn create_asil_c_component() -> Result<Component> {
       // Static memory allocation with budget enforcement
       let memory_guard = safe_managed_alloc!(16384, CrateId::Component)?;
       let provider = ComponentProvider::new(memory_guard);
       
       // Compile-time bounded collections
       let data = BoundedVec::<Data, MAX_ITEMS, ComponentProvider>::new(provider)?;
       
       Ok(Component::new_with_safety(data, SafetyLevel::ASIL_C))
   }

Tier 4: ASIL-D (Highest Safety-Critical)
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

**Purpose**: Highest safety-critical systems requiring mathematical verification

**Configuration**:

.. code-block:: toml

   [dependencies]
   wrt = { version = "*", features = ["safety-asil-d"], default-features = false }

**Characteristics**:

- Zero dynamic allocation (completely static)
- All memory allocated at compile time
- Formal verification proofs required
- Mathematical guarantees of safety properties
- Suitable for: Safety-critical automotive functions, aerospace systems, medical devices

**Example Usage**:

.. code-block:: rust

   use wrt_foundation::{BoundedVec, StaticProvider, SafetyLevel, static_alloc};
   
   // Completely static allocation
   static_alloc! {
       static COMPONENT_MEMORY: [u8; 8192] = [0; 8192];
   }
   
   const MAX_ITEMS: usize = 128;
   type StaticComponentProvider = StaticProvider<8192>;
   
   pub fn create_asil_d_component() -> Result<Component> {
       // Static provider with compile-time memory
       let provider = StaticComponentProvider::from_static(&COMPONENT_MEMORY);
       
       // Compile-time bounded collections with formal verification
       let data = BoundedVec::<Data, MAX_ITEMS, StaticComponentProvider>::new(provider)?;
       
       // ASIL-D requires formal verification proof
       #[cfg(kani)]
       kani::proof(|| {
           assert!(data.capacity() == MAX_ITEMS);
           assert!(data.memory_usage() <= 8192);
       });
       
       Ok(Component::new_asil_d(data))
   }

Feature Configuration Matrix
---------------------------

.. list-table:: Safety Tier Feature Matrix
   :header-rows: 1
   :widths: 20 15 15 15 15 20

   * - Feature
     - QM
     - ASIL-A/B
     - ASIL-C
     - ASIL-D
     - Notes
   * - Dynamic Allocation
     - ✅ Full
     - ⚠️ Bounded
     - ❌ None
     - ❌ None
     - Progressively restricted
   * - Standard Library
     - ✅ Full
     - ✅ Full
     - ❌ None
     - ❌ None
     - no_std for safety levels
   * - Runtime Checks
     - ⚠️ Basic
     - ✅ Enhanced
     - ✅ Complete
     - ✅ Complete
     - More checks at higher levels
   * - Formal Verification
     - ❌ None
     - ❌ None
     - ⚠️ Optional
     - ✅ Required
     - KANI proofs required for ASIL-D
   * - Memory Budgets
     - ❌ None
     - ⚠️ Soft
     - ✅ Hard
     - ✅ Static
     - Enforcement level increases
   * - Compile-Time Bounds
     - ❌ None
     - ⚠️ Optional
     - ✅ Required
     - ✅ Complete
     - Static verification increases

Memory Model by Tier
--------------------

QM Memory Model
~~~~~~~~~~~~~~

.. code-block:: rust

   // Unrestricted memory allocation
   let mut dynamic_data = Vec::new();
   let mut hash_map = HashMap::new();
   
   // No memory tracking
   dynamic_data.push(expensive_computation());
   hash_map.insert(key, large_value);

ASIL-A/B Memory Model
~~~~~~~~~~~~~~~~~~~~

.. code-block:: rust

   // Bounded collections with runtime enforcement
   let mut bounded_data = BoundedVec::<Item, 1000>::new()?;
   let mut bounded_map = BoundedMap::<Key, Value, 100>::new()?;
   
   // Runtime capacity checking
   if bounded_data.len() < bounded_data.capacity() {
       bounded_data.push(item)?;
   }

ASIL-C Memory Model
~~~~~~~~~~~~~~~~~~

.. code-block:: rust

   // Static allocation with budget enforcement
   let memory_guard = safe_managed_alloc!(16384, CrateId::Component)?;
   let provider = NoStdProvider::<16384>::new(memory_guard);
   
   // Compile-time bounded collections
   let data = BoundedVec::<Item, 256, _>::new(provider)?;
   let map = BoundedMap::<Key, Value, 64, _>::new(provider.clone())?;

ASIL-D Memory Model
~~~~~~~~~~~~~~~~~~

.. code-block:: rust

   // Completely static memory
   static MEMORY: [u8; 8192] = [0; 8192];
   
   // Static provider
   let provider = StaticProvider::<8192>::from_static(&MEMORY);
   
   // Formal verification required
   #[cfg(kani)]
   fn verify_memory_safety() {
       kani::proof(|| {
           let data = BoundedVec::<Item, 128, _>::new(provider).unwrap();
           assert!(data.capacity() == 128);
           assert!(data.memory_usage() <= 8192);
       });
   }

Safety Context Management
------------------------

Safety Context Creation
~~~~~~~~~~~~~~~~~~~~~~

.. code-block:: rust

   use wrt_foundation::{SafetyContext, SafetyLevel};
   
   // Create safety context appropriate for tier
   let qm_context = SafetyContext::new(SafetyLevel::QM)?;
   let asil_b_context = SafetyContext::new(SafetyLevel::ASIL_B)?;
   let asil_c_context = SafetyContext::new(SafetyLevel::ASIL_C)?;
   let asil_d_context = SafetyContext::new(SafetyLevel::ASIL_D)?;

Safety Level Enforcement
~~~~~~~~~~~~~~~~~~~~~~~

.. code-block:: rust

   pub fn perform_operation(context: &SafetyContext) -> Result<()> {
       // Check safety level requirements
       match context.safety_level() {
           SafetyLevel::QM => {
               // No additional constraints
               perform_unrestricted_operation()
           }
           SafetyLevel::ASIL_A | SafetyLevel::ASIL_B => {
               // Bounded resource usage
               perform_bounded_operation()
           }
           SafetyLevel::ASIL_C => {
               // Static allocation only
               perform_static_operation()
           }
           SafetyLevel::ASIL_D => {
               // Formal verification required
               perform_verified_operation()
           }
       }
   }

Cross-Tier Interaction
---------------------

Safety Level Compatibility
~~~~~~~~~~~~~~~~~~~~~~~~~

.. list-table:: Tier Interaction Matrix
   :header-rows: 1
   :widths: 15 15 15 15 15 25

   * - Caller
     - QM
     - ASIL-A/B
     - ASIL-C
     - ASIL-D
     - Notes
   * - QM
     - ✅
     - ❌
     - ❌
     - ❌
     - QM cannot call safety functions
   * - ASIL-A/B
     - ✅
     - ✅
     - ❌
     - ❌
     - Can call lower safety levels
   * - ASIL-C
     - ✅
     - ✅
     - ✅
     - ❌
     - Can call equal or lower levels
   * - ASIL-D
     - ✅
     - ✅
     - ✅
     - ✅
     - Can call any level

Safety Boundary Enforcement
~~~~~~~~~~~~~~~~~~~~~~~~~~

.. code-block:: rust

   pub fn safe_interface_call(
       from_level: SafetyLevel,
       to_level: SafetyLevel,
       operation: fn() -> Result<()>
   ) -> Result<()> {
       // Enforce safety level compatibility
       if !from_level.can_call(to_level) {
           return Err(Error::SafetyLevelViolation {
               from: from_level,
               to: to_level,
           });
       }
       
       // Perform operation with appropriate safety context
       operation()
   }

Migration Guidelines
-------------------

QM to ASIL-A/B Migration
~~~~~~~~~~~~~~~~~~~~~~~

1. **Replace Dynamic Collections**:
   
   .. code-block:: rust
   
      // Before
      let mut data = Vec::new();
      
      // After  
      let mut data = BoundedVec::<Item, 1000>::new()?;

2. **Add Safety Context**:
   
   .. code-block:: rust
   
      // Add safety context to component
      let safety_ctx = SafetyContext::new(SafetyLevel::ASIL_B)?;

ASIL-A/B to ASIL-C Migration
~~~~~~~~~~~~~~~~~~~~~~~~~~~~

1. **Replace Standard Collections**:
   
   .. code-block:: rust
   
      // Before
      let mut data = BoundedVec::<Item, 1000>::new()?;
      
      // After
      let guard = safe_managed_alloc!(16384, CrateId::Component)?;
      let provider = NoStdProvider::<16384>::new(guard);
      let mut data = BoundedVec::<Item, 256, _>::new(provider)?;

2. **Remove Standard Library Dependencies**:
   
   .. code-block:: toml
   
      # Remove std feature
      wrt = { version = "*", features = ["safety-asil-c"], default-features = false }

ASIL-C to ASIL-D Migration
~~~~~~~~~~~~~~~~~~~~~~~~~~

1. **Convert to Static Allocation**:
   
   .. code-block:: rust
   
      // Before
      let guard = safe_managed_alloc!(16384, CrateId::Component)?;
      
      // After
      static MEMORY: [u8; 16384] = [0; 16384];
      let provider = StaticProvider::<16384>::from_static(&MEMORY);

2. **Add Formal Verification**:
   
   .. code-block:: rust
   
      #[cfg(kani)]
      fn verify_component_safety() {
          kani::proof(|| {
              // Formal verification of safety properties
          });
      }

Best Practices
--------------

Tier Selection Guidelines
~~~~~~~~~~~~~~~~~~~~~~~~

1. **Start with Appropriate Tier**: Choose the lowest tier that meets safety requirements
2. **Progressive Migration**: Move to higher tiers as safety requirements increase
3. **Component Isolation**: Use different tiers for different components as appropriate
4. **Interface Design**: Design clear interfaces between different safety tiers

Testing Strategies
~~~~~~~~~~~~~~~~~

1. **QM**: Standard unit and integration testing
2. **ASIL-A/B**: Add property-based testing and resource exhaustion testing
3. **ASIL-C**: Include formal specification testing and budget validation
4. **ASIL-D**: Require formal verification proofs and mathematical validation

Documentation Requirements
~~~~~~~~~~~~~~~~~~~~~~~~~~

1. **Safety Rationale**: Document why each tier was chosen
2. **Interface Contracts**: Specify safety requirements for cross-tier calls
3. **Verification Evidence**: Provide appropriate evidence for each tier
4. **Migration Plans**: Document upgrade paths between tiers

See Also
--------

- :doc:`safety_guidelines` - General safety development guidelines
- :doc:`formal_verification` - Formal verification with KANI
- :doc:`../architecture/memory_safety_comparison` - Memory safety approaches
- :doc:`../qualification/safety_analysis` - Safety analysis documentation