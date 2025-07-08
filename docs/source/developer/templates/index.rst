=======================
Documentation Templates
=======================

These templates ensure consistent, high-quality documentation across the PulseEngine project.

.. contents:: Available Templates
   :local:
   :depth: 1

Overview
========

Each template provides:

- Standard structure for specific document types
- Required sections with guidance
- Examples of proper formatting
- Compliance with style guide

Using Templates
===============

1. Choose the appropriate template for your document type
2. Copy the template content
3. Replace all ``[...]`` placeholders with actual content
4. Follow the embedded instructions
5. Remove instruction sections before finalizing

Available Templates
===================

API Documentation
-----------------

.. toctree::
   :maxdepth: 1

   api_reference

Use for documenting:

- Public APIs and modules
- Crate-level documentation
- Trait and type references

User Guides
-----------

.. toctree::
   :maxdepth: 1

   user_guide

Use for:

- Feature tutorials
- How-to guides
- Getting started documentation

Technical Design
----------------

.. toctree::
   :maxdepth: 1

   design_document

Use for:

- Architecture proposals
- Feature designs
- Technical specifications

Safety Documentation
--------------------

.. toctree::
   :maxdepth: 1

   safety_document

**Required for:**

- Safety-critical components
- ASIL-rated features
- Components requiring certification

Template Selection Guide
========================

.. list-table:: When to Use Each Template
   :widths: 30 70
   :header-rows: 1

   * - Template
     - Use When
   * - API Reference
     - Documenting public interfaces, modules, or crates
   * - User Guide
     - Creating tutorials or how-to documentation
   * - Design Document
     - Proposing new features or architecture changes
   * - Safety Document
     - Documenting safety-critical components

Best Practices
==============

1. **Start with a template** - Don't create documents from scratch
2. **Keep placeholders updated** - Replace all [...] markers
3. **Follow the style guide** - See :doc:`../style_guide`
4. **Review before publishing** - Use the review checklist
5. **Version control** - Track all document changes

Creating New Templates
======================

If you need a new template type:

1. Identify common structure across similar documents
2. Create template following existing patterns
3. Add to this index
4. Submit PR with rationale

Template Maintenance
====================

Templates are living documents. When updating:

- Consider impact on existing documents
- Update style guide if needed
- Communicate changes to team
- Version significant changes

See Also
========

- :doc:`../style_guide` - Writing standards
- :doc:`../contributing/documentation` - Documentation process
- :doc:`/safety_manual/index` - Safety documentation requirements