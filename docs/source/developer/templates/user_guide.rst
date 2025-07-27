=======================
User Guide Template
=======================

.. note::
   This template provides a standard structure for user-facing documentation.

Instructions
============

Copy this template and replace all placeholders marked with ``[...]``.

Template
========

.. code-block:: rst

   ====================================
   [Feature/Component Name] User Guide
   ====================================

   .. warning::
      **Development Status**: [Status message if feature is incomplete]

   [Brief introduction - what this guide covers and who it's for]

   .. contents:: On this page
      :local:
      :depth: 2

   What is [Feature]?
   ==================

   [Clear explanation in user terms, avoiding implementation details]

   **Key Benefits:**

   - [Benefit 1]
   - [Benefit 2]
   - [Benefit 3]

   **Use Cases:**

   - [Use case 1]
   - [Use case 2]

   Prerequisites
   =============

   Before you begin, ensure you have:

   - [ ] [Requirement 1]
   - [ ] [Requirement 2]
   - [ ] Basic understanding of [concept]

   Getting Started
   ===============

   Step 1: [First Action]
   ----------------------

   [Clear instructions with context]

   .. code-block:: bash

      # Example command
      command --with-options

   **Expected output:**

   .. code-block:: text

      [Show what users should see]

   Step 2: [Second Action]
   -----------------------

   [Continue with logical flow]

   Basic Usage
   ===========

   [Simple Example Name]
   ---------------------

   Here's how to [accomplish basic task]:

   .. code-block:: rust

      // [Explain what this does]
      use [module];

      fn main() {
          // [Step by step comments]
          let result = [operation]();
          println!("Result: {:?}", result);
      }

   **What's happening:**

   1. [Explain line 1]
   2. [Explain line 2]
   3. [Explain the output]

   Common Patterns
   ===============

   [Pattern 1]: [Name]
   -------------------

   **When to use:** [Scenario]

   .. code-block:: rust

      // [Pattern implementation]

   **Benefits:** [Why this pattern helps]

   [Pattern 2]: [Name]
   -------------------

   [Similar structure]

   Configuration
   =============

   Basic Configuration
   -------------------

   [Feature] can be configured through:

   .. code-block:: toml

      # In Cargo.toml
      [dependencies]
      module = { version = "0.1", features = ["[feature]"] }

   Configuration Options
   ---------------------

   .. list-table::
      :widths: 30 20 50
      :header-rows: 1

      * - Option
        - Default
        - Description
      * - ``[option_name]``
        - ``[default]``
        - [What it controls]

   Advanced Features
   =================

   [Advanced Feature 1]
   --------------------

   For more complex scenarios, you can:

   .. code-block:: rust

      // [Advanced example]

   .. warning::
      This requires [prerequisite or caution]

   Troubleshooting
   ===============

   Common Issues
   -------------

   **Problem:** [Error message or symptom]

   **Solution:** 
   
   1. [First step to try]
   2. [Second step]
   3. [How to verify it's fixed]

   ---

   **Problem:** [Another common issue]

   **Solution:** [Resolution steps]

   Debugging Tips
   --------------

   Enable debug output:

   .. code-block:: bash

      RUST_LOG=debug cargo run

   Performance Tips
   ================

   Optimization 1: [Name]
   ----------------------

   [Explain optimization and when to use it]

   .. code-block:: rust

      // Before
      [slower_code]

      // After
      [optimized_code]

   **Performance impact:** [Quantify if possible]

   Best Practices
   ==============

   DO:
   ---

   - ✅ [Best practice 1]
   - ✅ [Best practice 2]
   - ✅ [Best practice 3]

   DON'T:
   ------

   - ❌ [Anti-pattern 1]
   - ❌ [Anti-pattern 2]
   - ❌ [Anti-pattern 3]

   Real-World Example
   ==================

   Here's a complete example showing [feature] in a realistic scenario:

   .. code-block:: rust

      use [modules];

      /// [Describe what this example demonstrates]
      fn real_world_example() -> Result<()> {
          // [Complete, runnable example]
          
          Ok(())
      }

   Migration Guide
   ===============

   Migrating from [Old Version/Method]
   ------------------------------------

   If you're upgrading from [old version]:

   1. **Change 1:** [What changed and why]
   
      .. code-block:: rust
   
         // Old way
         [old_code]
         
         // New way
         [new_code]

   2. **Change 2:** [Next change]

   FAQ
   ===

   **Q: [Common question]?**

   A: [Clear, concise answer]

   ---

   **Q: [Another question]?**

   A: [Answer with example if helpful]

   Next Steps
   ==========

   Now that you understand [feature], you might want to:

   - :doc:`[related_guide]` - [What they'll learn]
   - :doc:`/examples/[example]` - [What it demonstrates]
   - :doc:`/api/[module]` - [When to reference API docs]

   Getting Help
   ============

   If you need assistance:

   1. Check the :doc:`/troubleshooting` guide
   2. Search existing `GitHub issues`_
   3. Ask in the community forums
   4. Report bugs with minimal reproduction

   .. _GitHub issues: https://github.com/[repo]/issues