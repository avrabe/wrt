=======================
CLI (WRTD) Architecture
=======================

The WRTD command-line interface provides a user-friendly way to execute WebAssembly modules and components.

.. spec:: CLI Architecture
   :id: SPEC_005
   :links: REQ_003, REQ_015, REQ_RESOURCE_004
   
   .. uml:: ../../_static/cli_architecture.puml
      :alt: CLI Architecture
      :width: 100%
   
   The CLI architecture includes:
   
   1. Command-line argument parsing
   2. Module loading and instantiation
   3. Execution control with fuel limits
   4. Statistics reporting
   5. Logging configuration
   6. Component interface analysis capabilities
   7. WASI interface implementation

.. impl:: CLI Implementation
   :id: IMPL_008
   :status: implemented
   :links: SPEC_005, REQ_003, REQ_015, REQ_RESOURCE_004, IMPL_FUEL_001
   
   The WRTD CLI provides:
   
   1. WebAssembly file loading
   2. Optional function calling
   3. Fuel-bounded execution
   4. Execution statistics reporting
   5. Logging configuration and output (see :doc:`logging`)
   6. Component interface parsing and introspection
   7. Support for both WebAssembly modules and components
   
   Command-line options include:
   - ``--call <function>`` - Function to call
   - ``--fuel <amount>`` - Set fuel limit for bounded execution
   - ``--stats`` - Show execution statistics
   - ``--analyze-component-interfaces`` - Analyze component interfaces without execution 