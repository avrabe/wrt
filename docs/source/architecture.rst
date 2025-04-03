Software Architecture
=====================

This chapter describes the software architecture of the WRT system. The architecture is designed to meet the requirements specified in the :doc:`requirements` section.

System Overview
---------------

WRT is a WebAssembly runtime implementation with a focus on bounded execution, bare-metal support, and component model capabilities. The architecture is organized into several key subsystems:

.. spec:: System Component Diagram
   :id: SPEC_001
   :links: REQ_001, REQ_003, REQ_014, REQ_018
   
   .. uml:: _static/system_components.puml
      :alt: WRT System Components
      :width: 100%

   The WRT system is comprised of the following main components:
   
   1. Core Runtime - The foundational WebAssembly execution engine
      - Traditional Engine - Stack-based execution engine
      - Stackless Engine - State machine-based execution engine for bounded execution
   2. Component Model - The implementation of WebAssembly Component Model Preview 2
   3. WASI Interfaces - Platform-specific implementations of WASI APIs
   4. CLI (WRTD) - Command-line interface for running WebAssembly modules


Core Runtime Architecture
-------------------------

The core runtime is responsible for executing WebAssembly instructions and managing the WebAssembly execution environment.

.. spec:: Core Runtime Execution Flow
   :id: SPEC_006
   :links: REQ_001, REQ_003, REQ_007
   
   .. uml:: _static/component_flow.puml
      :alt: WRT Execution Flow
      :width: 100%
   
   The execution flow demonstrates the bounded execution model, showing how the WRTD CLI interacts with the Engine and how the fuel-based execution can be paused when fuel is exhausted.

.. spec:: Core Runtime Architecture
   :id: SPEC_002
   :links: REQ_001, REQ_003, REQ_005, REQ_006, REQ_007
   
   The core runtime provides two main interpreter implementations:
   
   1. Traditional Execution Engine - Suitable for most environments
   2. Stackless Execution Engine - Designed specifically for:
      - Bounded execution through fuel metering
      - Resumability after execution pauses
      - No-std compatibility for bare-metal environments
      - State serialization for migration between systems

.. impl:: Engine Implementation
   :id: IMPL_001
   :status: implemented
   :links: SPEC_002, REQ_001, REQ_003, REQ_007
   
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
   :status: partial
   :links: SPEC_002, REQ_018
   
   The ``Module`` struct encapsulates a WebAssembly module and provides:
   
   1. Binary parsing and validation
   2. Type checking
   3. Function table management
   4. Memory management
   
   Key methods include:
   - ``load_from_binary(bytes)`` - Loads a WebAssembly binary
   - ``validate()`` - Validates the module structure and types
   - ``instantiate(engine)`` - Creates a new module instance

.. impl:: Memory Implementation
   :id: IMPL_003
   :status: implemented
   :links: SPEC_002, REQ_018
   
   The ``Memory`` struct manages WebAssembly linear memory:
   
   1. Handles memory allocations and resizing
   2. Enforces memory access boundaries
   3. Provides safe read/write operations
   
   Key methods include:
   - ``grow(pages)`` - Grows memory by the specified number of pages
   - ``size()`` - Returns the current memory size in pages
   - ``read/write(addr, data)`` - Safely reads/writes memory with bounds checking

.. impl:: Stack Implementation
   :id: IMPL_004
   :status: partial
   :links: SPEC_002, REQ_005
   
   The ``Stack`` struct implements a stackless interpreter model:
   
   1. Stores the WebAssembly value stack
   2. Tracks control flow with labels
   3. Enables pausing and resuming execution at any point
   
   This implementation enables bounded execution and future state migration.

Component Model Architecture
----------------------------

The Component Model subsystem implements the WebAssembly Component Model Preview 2 specification.

.. spec:: Component Model Architecture
   :id: SPEC_003
   :links: REQ_014, REQ_019, REQ_020
   
   The Component Model implementation provides:
   
   1. Component instantiation and linking
   2. Interface type conversion
   3. Resource type management
   4. Host function binding
   5. Binary format parsing and validation
   6. Component instance management

.. impl:: Component Implementation
   :id: IMPL_005
   :status: partial
   :links: SPEC_003, REQ_014, REQ_019
   
   The ``Component`` struct represents a WebAssembly component:
   
   1. Parses component binary format
   2. Manages component instances
   3. Handles interface binding
   4. Orchestrates resource lifetime
   
   Key methods include:
   - ``load_from_binary(bytes)`` - Loads a component binary
   - ``instantiate(engine, imports)`` - Creates a new component instance
   - ``link(other_component)`` - Links two components together

.. impl:: Interface Type Handling
   :id: IMPL_006
   :status: partial
   :links: SPEC_003, REQ_014, REQ_019
   
   Interface types are managed through:
   
   1. Type adapters for each interface type
   2. Conversion between host and component types
   3. Validation of type compatibility
   
   The implementation handles interface types including records, variants, enums, flags, and resources.

.. impl:: Resource Type Handling
   :id: IMPL_010
   :status: partial
   :links: SPEC_003, REQ_014, REQ_019
   
   Resource types are implemented through:
   
   1. Reference counting for resource instances
   2. Resource tables for tracking live resources
   3. Host callbacks for resource lifecycle events
   4. Resource dropping semantics
   
   Key components:
   - ``Resource`` struct - Represents a component model resource
   - ``ResourceType`` - Type information for resources
   - Resource lifetime management functions

Logging Subsystem
-----------------

The logging subsystem implements the WASI logging API and provides platform-specific backends.

.. spec:: Logging Architecture
   :id: SPEC_004
   :links: REQ_015, REQ_016
   
   The logging architecture consists of:
   
   1. WASI logging component interface
   2. Host logging handler registration
   3. Platform-specific backends (planned)
   4. Log level filtering and routing
   
   **Logging Flow Sequence**
   
   The following sequence diagram illustrates how logging flows from a WebAssembly module through the runtime:
   
   .. uml:: _static/logging_flow.puml
      :alt: WRT Logging Flow
      :width: 100%
   
   When a WebAssembly module calls a logging function, the following steps occur:
   
   1. WebAssembly module calls the WASI logging interface function (`log`, `logTrace`, etc.)
   2. The WASI logging interface implementation in WRT converts the call to an engine operation
   3. The StacklessEngine creates a LogOperation object with level, message, and optional component ID
   4. The operation is passed to the CallbackRegistry via handle_log()
   5. The CallbackRegistry invokes the registered log handler (if any)
   6. The host logging system (terminal, file, syslog, etc.) processes the log message

.. impl:: Logging Implementation
   :id: IMPL_007
   :status: implemented
   :links: SPEC_004, REQ_015
   
   The logging implementation provides:
   
   1. Standard log levels (Trace, Debug, Info, Warn, Error, Critical)
   2. Registration of custom log handlers
   3. Default stderr fallback
   4. Component-specific context tracking
   
   Key components:
   
   - ``LogLevel`` - Enum with standard log levels (Trace, Debug, Info, Warn, Error, Critical)
   - ``LogOperation`` - Struct containing level, message, and optional component ID
   - ``CallbackRegistry`` - Central registry for handling WebAssembly component operations
   
   Key methods include:
   - ``register_log_handler(handler)`` - Registers a custom log handler
   - ``handle_log(operation)`` - Internal method to process log messages
   - ``LogOperation::with_component(level, message, component_id)`` - Creates a log operation with component context
   
   Note: Platform-specific backends (syslog for Linux, os_log for macOS) are currently planned but not yet implemented.

CLI (WRTD) Architecture
-----------------------

The WRTD command-line interface provides a user-friendly way to execute WebAssembly modules and components.

.. spec:: CLI Architecture
   :id: SPEC_005
   :links: REQ_003, REQ_015
   
   The CLI architecture includes:
   
   1. Command-line argument parsing
   2. Module loading and instantiation
   3. Execution control with fuel limits
   4. Statistics reporting
   5. Logging configuration
   6. Component interface analysis capabilities

.. impl:: CLI Implementation
   :id: IMPL_008
   :status: implemented
   :links: SPEC_005, REQ_003, REQ_015
   
   The WRTD CLI provides:
   
   1. WebAssembly file loading
   2. Optional function calling
   3. Fuel-bounded execution
   4. Execution statistics reporting
   5. Logging configuration and output
   6. Component interface parsing and introspection
   
   Command-line options include:
   - ``--call <function>`` - Function to call
   - ``--fuel <amount>`` - Set fuel limit for bounded execution
   - ``--stats`` - Show execution statistics
   - ``--analyze-component-interfaces`` - Analyze component interfaces without execution

Testing Tools
------------

WRT includes specialized tools for testing and validation of the WebAssembly implementation.

.. impl:: WAST Test Runner
   :id: IMPL_009
   :status: implemented
   :links: REQ_022
   
   The WAST test runner tool is a specialized binary for executing WebAssembly specification tests:
   
   1. Parses and executes WAST test files
   2. Validates interpreter behavior against the WebAssembly specification
   3. Tracks test results for conformance reporting
   4. Supports blacklisting of tests that are known to fail
   
   This tool is crucial for ensuring that the WRT implementation correctly follows the WebAssembly specification.

Development Status
------------------

The current implementation status of the WRT architecture is as follows:

.. needtable::
   :columns: id;title;status;links
   :filter: type == 'impl'

Architecture-Requirement Mapping
--------------------------------

The following diagram shows how the architectural components map to requirements:

.. needflow::
   :filter: id in ['SPEC_001', 'SPEC_002', 'SPEC_003', 'SPEC_004', 'SPEC_005', 'SPEC_006', 'IMPL_001', 'IMPL_002', 'IMPL_003', 'IMPL_004', 'IMPL_005', 'IMPL_006', 'IMPL_007', 'IMPL_008', 'IMPL_009', 'IMPL_010', 'REQ_001', 'REQ_003', 'REQ_005', 'REQ_006', 'REQ_007', 'REQ_014', 'REQ_015', 'REQ_016', 'REQ_018', 'REQ_019', 'REQ_020', 'REQ_022']