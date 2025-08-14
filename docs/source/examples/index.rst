====================
Examples & Tutorials
====================

.. image:: ../_static/icons/execution_flow.svg
   :width: 64px
   :align: right
   :alt: Examples Icon

.. epigraph::

   "Show me the code!" 
   
   -- Linus Torvalds

Welcome to PulseEngine examples! These examples are organized by implementation status and complexity to help you find exactly what you need.

.. contents:: What's Inside
   :local:
   :depth: 2

.. warning::
   **Implementation Status**: Example status varies by category. Check individual sections for current availability.

Example Categories
==================

Our examples are organized into four main categories:

Fundamentals (Working Code)
---------------------------

Ready-to-use examples demonstrating PulseEngine's working components.

.. grid:: 1
   :gutter: 3

   .. grid-item-card:: 🏗️ Foundation Examples
      :link: fundamentals/index
      :link-type: doc

      **Status: ✅ Working** - Core building blocks, safe memory, collections, and primitives that work today.

Target API (Design Intent)
---------------------------

Examples showing PulseEngine's intended API design once development is complete.

.. grid:: 1
   :gutter: 3

   .. grid-item-card:: 🎯 Target API Examples
      :link: target_api/index
      :link-type: doc

      **Status: 🔄 Design** - Hello World, basic components, and intended high-level APIs.

Platform Integration
--------------------

Platform-specific examples and integration patterns.

.. grid:: 1
   :gutter: 3

   .. grid-item-card:: 🖥️ Integration Examples
      :link: integration/index
      :link-type: doc

      **Status: ⚠️ Mixed** - Platform features, memory management, host functions, and system integration.

Advanced Reference
------------------

Advanced patterns, debugging, and reference implementations.

.. grid:: 1
   :gutter: 3

   .. grid-item-card:: 🎓 Reference Examples
      :link: reference/index
      :link-type: doc

      **Status: 📋 Reference** - Advanced patterns, debugging techniques, and comprehensive guides.

Quick Start
===========

New to PulseEngine? Start with these recommended paths:

.. grid:: 2
   :gutter: 3

   .. grid-item-card:: I want working code now
      :link: fundamentals/index
      :link-type: doc

      Jump to **Fundamentals** for examples that compile and run today.

   .. grid-item-card:: I want to see the vision
      :link: target_api/index
      :link-type: doc

      Check **Target API** to see where PulseEngine is heading.

Status Legend
=============

- ✅ **Working** - Code compiles and runs, ready to use
- ⚠️ **Mixed** - Some examples work, others are placeholders  
- 🔄 **Design** - Shows intended API, implementation in progress
- 📋 **Reference** - Documentation and patterns, varying implementation status

.. admonition:: Implementation Note
   :class: note

   PulseEngine is under active development. Examples in **Fundamentals** represent working code you can use today, while **Target API** shows the intended interface once development is complete.

.. toctree::
   :hidden:
   :maxdepth: 2
   :caption: Examples

   fundamentals/index
   target_api/index
   integration/index
   reference/index