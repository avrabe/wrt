================================
Safety Documentation Migration
================================

This guide documents the consolidation of safety documentation into the unified Safety Manual.

Migration Status
================

Files to Deprecate
------------------

The following files are superseded by the Safety Manual and should be removed after verification:

**Root-level safety files** (to be removed):

- ``/docs/source/safety_requirements.rst`` → Migrated to ``safety_manual/requirements.rst``
- ``/docs/source/safety_mechanisms.rst`` → Migrated to ``safety_manual/mechanisms.rst``  
- ``/docs/source/safety_implementations.rst`` → Migrated to ``safety_manual/implementations.rst``
- ``/docs/source/safety_test_cases.rst`` → Migrated to ``safety_manual/verification.rst``

**Safety directory files** (to be removed or consolidated):

- ``/docs/source/safety/mechanisms.rst`` → Consolidated into ``safety_manual/mechanisms.rst``
- ``/docs/source/safety/implementations.rst`` → Consolidated into ``safety_manual/implementations.rst``
- ``/docs/source/safety/test_cases.rst`` → Consolidated into ``safety_manual/verification.rst``

Files to Keep
-------------

The following files serve different purposes and should be retained:

**Architecture files** (keep):

- ``/docs/source/architecture/safety.rst`` - Architectural view of safety
- Other architecture documentation

**Requirements directory** (keep structure, update content):

- ``/docs/source/requirements/index.rst`` - Overall requirements index
- ``/docs/source/requirements/functional.rst`` - Functional requirements
- ``/docs/source/requirements/safety.rst`` - Update to reference Safety Manual

**Qualification directory** (keep):

- All files in ``/docs/source/qualification/`` - Separate qualification evidence

Migration Checklist
===================

Phase 1: Content Migration ✅
-----------------------------

- [x] Create Safety Manual structure
- [x] Migrate safety assumptions (new comprehensive document)
- [x] Consolidate safety requirements 
- [x] Merge safety mechanisms (reconcile duplicate IDs)
- [ ] Consolidate implementations
- [ ] Merge test cases into verification
- [ ] Create unified traceability matrix

Phase 2: Cross-Reference Update
-------------------------------

- [ ] Update all internal links to point to Safety Manual
- [ ] Update index.rst files to remove deprecated entries
- [ ] Update README files that reference old structure
- [ ] Fix documentation cross-references

Phase 3: Cleanup
----------------

- [ ] Archive deprecated files (don't delete immediately)
- [ ] Update CI/CD to exclude deprecated files
- [ ] Update documentation build configuration
- [ ] Verify no broken links remain

ID Reconciliation
=================

The following ID conflicts need resolution:

**Safety Mechanism IDs**:

- Old: ``SAFETY_MEM_003`` (Resource limitation)
- New: ``SAFETY_MEM_001`` (Bounds checking)
- Resolution: Renumber in consolidated document

**Implementation IDs**:

- Various ``IMPL_*`` IDs scattered across files
- Resolution: Create unified numbering scheme

Benefits of Consolidation
=========================

1. **Single Source of Truth** - No more duplicate/conflicting information
2. **ISO 26262 Compliance** - Follows SEooC structure requirements
3. **Easier Maintenance** - One location to update
4. **Better Traceability** - Unified requirement/mechanism mapping
5. **Clearer Navigation** - Logical organization by safety topic

Rollback Plan
=============

If issues arise:

1. Git history preserves all original files
2. Keep archived copies for 6 months
3. Document any integration-specific needs
4. Maintain redirect links if needed

Next Steps
==========

1. Complete remaining content migration
2. Review and approve consolidated content
3. Update all cross-references
4. Test documentation build
5. Archive deprecated files
6. Communicate changes to team