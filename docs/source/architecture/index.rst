=====================
Software Architecture
=====================

.. image:: ../_static/icons/wrt_architecture.svg
   :width: 64px
   :align: right
   :alt: WRT Architecture Icon

This chapter describes the software architecture of the WRT system. The architecture is designed to meet the requirements specified in the :doc:`../requirements` section and the safety requirements in the :doc:`../safety_requirements` section.

.. toctree::
   :maxdepth: 2
   :caption: Architecture Topics:

   core_runtime
   memory_model
   platform_layer
   component_model
   resource_management
   intercept_system
   safe_memory
   hardening
   logging
   safety
   cli
   testing

.. _system-overview:

System Overview
---------------

WRT is a WebAssembly runtime implementation with a focus on bounded execution, bare-metal support, and component model capabilities. The architecture is organized into several key subsystems, detailed in the pages linked above.

.. _system-component-diagram:

.. spec:: System Component Diagram
   :id: SPEC_001
   :links: REQ_PLATFORM_001, REQ_HELPER_ABI_001, REQ_014, REQ_018, REQ_MEM_SAFETY_001, REQ_RESOURCE_001
   
   .. note::
      This diagram will be updated to reflect the new platform layer and memory model.

   .. uml::
      
      @startuml
      package "WRT System (Current)" {
        [Core Runtime] as Core
        [Component Model] as Component
        [Memory Subsystem (Vec<u8>)] as Memory
        [Resource Management] as Resources
        [Safety Layer] as Safety
        [WASI Interfaces] as WASI
        [CLI (WRTD)] as CLI
        
        Core --> Memory
        Component --> Core
        Safety --> Core
        Safety --> Memory
        Resources --> Memory
        CLI --> Core
        CLI --> Component
        WASI --> Core
      }
      @enduml

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
   :filter: id in ['SPEC_001', 'SPEC_002', 'SPEC_003', 'SPEC_004', 'SPEC_005', 'SPEC_006', 'SPEC_007', 'SPEC_008', 'SPEC_009', 'SPEC_010', 'IMPL_001', 'IMPL_002', 'IMPL_003', 'IMPL_004', 'IMPL_005', 'IMPL_006', 'IMPL_007', 'IMPL_008', 'IMPL_009', 'IMPL_010', 'IMPL_011', 'IMPL_012', 'REQ_PLATFORM_001', 'REQ_HELPER_ABI_001', 'REQ_005', 'REQ_006', 'REQ_007', 'REQ_014', 'REQ_015', 'REQ_016', 'REQ_018', 'REQ_019', 'REQ_020', 'REQ_021', 'REQ_022', 'REQ_023', 'REQ_024', 'REQ_MEM_SAFETY_001', 'REQ_MEM_SAFETY_002', 'REQ_MEM_SAFETY_003', 'REQ_RESOURCE_001', 'REQ_RESOURCE_002', 'REQ_RESOURCE_003', 'REQ_RESOURCE_004', 'REQ_RESOURCE_005', 'REQ_ERROR_001', 'REQ_ERROR_002', 'REQ_ERROR_003', 'REQ_ERROR_004', 'REQ_ERROR_005', 'REQ_VERIFY_001', 'REQ_VERIFY_002', 'REQ_VERIFY_003', 'REQ_VERIFY_004', 'REQ_QA_001', 'REQ_QA_002', 'REQ_QA_003', 'REQ_SAFETY_001', 'REQ_SAFETY_002']
   :name: architecture_requirement_mapping

.. _safety-architecture-mapping:

Safety-Architecture Mapping
---------------------------

The following diagram shows the relationship between safety requirements and architectural components:

.. needflow::
   :filter: id in ['SPEC_002', 'SPEC_007', 'SPEC_008', 'SPEC_009', 'SPEC_010', 'IMPL_MEMORY_SAFETY_001', 'IMPL_RESOURCE_SAFETY_001', 'IMPL_ERROR_HANDLING_RECOVERY_001', 'IMPL_VERIFICATION_001', 'IMPL_SAFETY_TESTING_001', 'REQ_MEM_SAFETY_001', 'REQ_MEM_SAFETY_002', 'REQ_MEM_SAFETY_003', 'REQ_RESOURCE_001', 'REQ_RESOURCE_002', 'REQ_RESOURCE_003', 'REQ_RESOURCE_004', 'REQ_RESOURCE_005', 'REQ_ERROR_001', 'REQ_ERROR_002', 'REQ_ERROR_003', 'REQ_ERROR_004', 'REQ_ERROR_005', 'REQ_VERIFY_001', 'REQ_VERIFY_002', 'REQ_VERIFY_003', 'REQ_VERIFY_004', 'REQ_QA_001', 'REQ_QA_002', 'REQ_QA_003', 'REQ_SAFETY_001', 'REQ_SAFETY_002', 'IMPL_BOUNDS_001', 'IMPL_SAFE_SLICE_001', 'IMPL_ADAPTER_001', 'IMPL_WASM_MEM_001', 'IMPL_LIMITS_001', 'IMPL_FUEL_001', 'IMPL_ERROR_HANDLING_001', 'IMPL_RECOVERY_001', 'IMPL_SAFETY_TEST_001', 'IMPL_FUZZ_001']
   :name: safety_architecture_mapping 