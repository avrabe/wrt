=========================
Core Runtime Architecture
=========================

.. image:: ../../_static/icons/execution_flow.svg
   :width: 48px
   :align: left
   :alt: Execution Flow Icon

The core runtime is responsible for executing WebAssembly instructions and managing the WebAssembly execution environment.

.. spec:: Core Runtime Execution Flow
   :id: SPEC_006
   :links: REQ_PLATFORM_001, REQ_HELPER_ABI_001, REQ_007, REQ_RESOURCE_004
   
   .. code-block:: text
      
      User -> WRTD CLI -> Engine -> Module
      Engine -> Memory
      
      When fuel is exhausted:
      Engine -> WRTD CLI (paused)
      WRTD CLI -> Engine (add fuel)
      Engine -> WRTD CLI (resume)
      
      Final result:
      Engine -> WRTD CLI -> User
   
   The execution flow demonstrates the bounded execution model, showing how the WRTD CLI interacts with the Engine and how the fuel-based execution can be paused when fuel is exhausted. The diagram also illustrates the difference between executing a standard WebAssembly module and a Component Model component, highlighting the resource management and canonical conversion aspects of the Component Model.

.. spec:: Core Runtime Architecture
   :id: SPEC_002
   :links: REQ_PLATFORM_001, REQ_HELPER_ABI_001, REQ_005, REQ_006, REQ_007, REQ_MEM_SAFETY_001, REQ_ERROR_003
   
   .. image:: ../../_static/icons/stackless_engine.svg
      :width: 48px
      :align: right
      :alt: Stackless Engine Icon
   
   .. code-block:: text
      
      Core Runtime
      ├── Engine
      │   ├── Stackless Engine
      │   ├── Execution State
      │   ├── Fuel Counter
      │   └── Statistics
      ├── Module
      │   ├── Binary Format
      │   ├── Types
      │   ├── Functions
      │   └── Validation
      └── Execution
          ├── Stack
          ├── Frame
          ├── Control Flow
          └── Instructions
   
   The core runtime provides a stackless interpreter implementation designed specifically for:
   
   - Bounded execution through fuel metering
   - Resumability after execution pauses
   - No-std compatibility for bare-metal environments
   - State serialization for migration between systems

.. impl:: Engine Implementation
   :id: IMPL_001
   :status: implemented
   :links: SPEC_002, REQ_PLATFORM_001, REQ_HELPER_ABI_001, REQ_007, REQ_RESOURCE_004, IMPL_FUEL_001
   
   The ``Engine`` struct is the central execution component that:
   
   1. Manages the WebAssembly state
   2. Tracks fuel consumption
   3. Provides execution control
   4. Contains statistics gathering capabilities
   
   Key methods include:
   - ``set_fuel(amount)`` - Sets the fuel limit for bounded execution
   - ``execute(instance_idx, func_idx, args)`` - Executes a WebAssembly function
   - ``remaining_fuel()`` - Returns the remaining fuel after execution
   - ``stats()`` - Returns execution statistics

.. impl:: Module Implementation
   :id: IMPL_002
   :status: implemented
   :links: SPEC_002, REQ_018, REQ_WASM_001
   
   The ``Module`` struct encapsulates a WebAssembly module and provides:
   
   1. Binary parsing and validation
   2. Type checking
   3. Function table management
   4. Memory management
   
   Key methods include:
   - ``load_from_binary(bytes)`` - Loads a WebAssembly binary
   - ``validate()`` - Validates the module structure and types
   - ``instantiate(engine)`` - Creates a new module instance

.. impl:: Stack Implementation
   :id: IMPL_004
   :status: implemented
   :links: SPEC_002, REQ_005, REQ_RESOURCE_002
   
   The ``Stack`` struct implements a value stack for the stackless interpreter model:
   
   1. Stores the WebAssembly value stack
   2. Tracks control flow with labels
   3. Enables pausing and resuming execution at any point
   
   This implementation enables bounded execution and state migration. 