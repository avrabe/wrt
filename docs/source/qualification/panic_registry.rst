.. _panic-registry:

Panic Registry
==============

This document contains all documented panic conditions in the WRT codebase.
Each panic is tracked as a qualification requirement using sphinx-needs.

.. contents:: Table of Contents
   :local:
   :depth: 2

Summary
-------

* Total panic points: 10
* Status:
  * Todo: 10
  * In Progress: 0
  * Resolved: 0

The original CSV version of this registry is maintained at:
docs/source/development/panic_registry.csv

.. csv-table:: Panic Registry CSV
   :file: panic_registry.csv
   :header-rows: 1
   :widths: auto

Panic Details
-------------

.. qual:: f32_nearest
   :id: WRTQ_001
   :item_status: Todo
   :implementation: 
   :tags: panic, unknown
   :safety_impact: LOW - Limited impact, only affects specific F32 operations
   :last_updated: 2025-04-25

   **File:** wrt/src/execution.rs
   **Line:** 389
   **Function:** f32_nearest

   This function will panic if the provided value is not an F32 value.

.. qual:: f64_nearest
   :id: WRTQ_002
   :item_status: Todo
   :implementation: 
   :tags: panic, unknown
   :safety_impact: LOW - Limited impact, only affects specific F64 operations
   :last_updated: 2025-04-25

   **File:** wrt/src/execution.rs
   **Line:** 425
   **Function:** f64_nearest

   This function will panic if the provided value is not an F64 value.

.. qual:: new
   :id: WRTQ_003
   :item_status: Todo
   :implementation: 
   :tags: panic, unknown
   :safety_impact: LOW - This function does not actually panic
   :last_updated: 2025-04-25

   **File:** wrt-sync/src/mutex.rs
   **Line:** 53
   **Function:** new

   This function does not panic.

.. qual:: new
   :id: WRTQ_004
   :item_status: Todo
   :implementation: 
   :tags: panic, unknown
   :safety_impact: MEDIUM - Memory corruption could cause system instability
   :last_updated: 2025-04-25

   **File:** wrt-types/src/safe_memory.rs
   **Line:** 50
   **Function:** new

   This function will panic if the initial integrity verification fails. This can happen if memory corruption is detected during initialization.

.. qual:: push
   :id: WRTQ_005
   :item_status: Todo
   :implementation: Return Result instead of panic
   :tags: panic, unknown
   :safety_impact: LOW - This function does not actually panic
   :last_updated: 2025-04-25

   **File:** wrt-types/src/bounded.rs
   **Line:** 196
   **Function:** push

   This function does not panic.

.. qual:: encode
   :id: WRTQ_006
   :item_status: Todo
   :implementation: Add checks for empty vector 
   :tags: panic, unknown
   :safety_impact: MEDIUM - Could cause unexpected termination during module loading
   :last_updated: 2025-04-25

   **File:** wrt-decoder/src/module.rs
   **Line:** 214
   **Function:** encode

   This function will panic if it attempts to access the last element of an empty custom_sections vector, which can happen if the implementation tries to process a custom section before any custom sections have been added to the module.

.. qual:: buffer
   :id: WRTQ_007
   :item_status: Todo
   :implementation: Improve error handling
   :tags: panic, unknown
   :safety_impact: MEDIUM - Memory access issues could cause system instability
   :last_updated: 2025-04-25

   **File:** wrt-runtime/src/memory.rs
   **Line:** 229
   **Function:** buffer

   In `no_std`