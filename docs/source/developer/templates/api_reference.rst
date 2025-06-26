=======================
API Reference Template
=======================

.. note::
   This template provides a standard structure for API documentation pages.

Instructions
============

Copy this template and replace all placeholders marked with ``[...]``.

Template
========

.. code-block:: rst

   ====================================
   [Module Name] API Reference
   ====================================

   .. warning::
      **API Stability**: [Stable|Experimental|Deprecated]
      
      [Additional status information if needed]

   [Brief description of the module's purpose - 2-3 sentences]

   .. contents:: On this page
      :local:
      :depth: 2

   Overview
   ========

   [Expanded description of the module, its role in the system, and primary use cases]

   Key Concepts
   ------------

   [Define important concepts users need to understand]

   - **[Concept 1]**: [Definition]
   - **[Concept 2]**: [Definition]

   Quick Example
   =============

   .. code-block:: rust

      use [module_path]::[MainType];

      // [Brief comment explaining the example]
      let instance = [MainType]::new();
      instance.[method]()?;

   Core Types
   ==========

   [TypeName]
   ----------

   .. code-block:: rust

      pub struct [TypeName] {
          // ...
      }

   [Description of the type and its purpose]

   **Key Methods:**

   - ``new()`` - [Description]
   - ``[method]()`` - [Description]

   **Example:**

   .. code-block:: rust

      let instance = [TypeName]::new();
      // [Usage example]

   Traits
   ======

   [TraitName]
   -----------

   .. code-block:: rust

      pub trait [TraitName] {
          fn [method](&self) -> Result<()>;
      }

   [Description of the trait and when to implement it]

   Functions
   =========

   [function_name]
   ---------------

   .. code-block:: rust

      pub fn [function_name](param: Type) -> Result<ReturnType>

   [Description of function purpose and behavior]

   **Parameters:**

   - ``param`` - [Description]

   **Returns:**

   - ``Ok(value)`` - [When successful]
   - ``Err(error)`` - [Common error cases]

   **Example:**

   .. code-block:: rust

      let result = [function_name](input)?;

   Error Handling
   ==============

   Common Errors
   -------------

   .. list-table::
      :widths: 30 70
      :header-rows: 1

      * - Error
        - Description
      * - ``[ErrorType]``
        - [When this occurs]
      * - ``[ErrorType]``
        - [When this occurs]

   Error Example
   -------------

   .. code-block:: rust

      match operation() {
          Ok(value) => println!("Success: {}", value),
          Err(e) => match e.kind() {
              ErrorKind::[Variant] => {
                  // Handle specific error
              }
              _ => return Err(e),
          }
      }

   Configuration
   =============

   [If applicable, describe configuration options]

   Feature Flags
   -------------

   .. list-table::
      :widths: 30 70
      :header-rows: 1

      * - Feature
        - Description
      * - ``[feature-name]``
        - [What it enables]

   Best Practices
   ==============

   1. **[Practice 1]**: [Description]
   2. **[Practice 2]**: [Description]

   Performance Considerations
   ==========================

   [Discuss performance characteristics, complexity, memory usage]

   Safety Considerations
   =====================

   [For safety-critical APIs, discuss safety requirements and constraints]

   Examples
   ========

   Complete Example
   ----------------

   .. code-block:: rust

      use [module]::prelude::*;

      fn main() -> Result<()> {
          // [Complete working example]
          Ok(())
      }

   Integration Example
   -------------------

   [Show how this API integrates with other modules]

   Migration Guide
   ===============

   [If API has changed, provide migration guidance]

   From v[X] to v[Y]
   -----------------

   .. code-block:: rust

      // Old way
      [old_code]

      // New way
      [new_code]

   See Also
   ========

   - :doc:`[related_module]` - [Description]
   - :doc:`/examples/[example]` - [Description]
   - `External Documentation`_ - [If applicable]

   .. _External Documentation: https://example.com