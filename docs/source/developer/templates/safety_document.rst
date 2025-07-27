============================
Safety Document Template
============================

.. note::
   This template provides a standard structure for safety-critical documentation
   following ISO 26262 and IEC 61508 guidelines.

Instructions
============

Copy this template and replace all placeholders marked with ``[...]``.

Template
========

.. code-block:: rst

   ====================================
   [Component Name] Safety Documentation
   ====================================

   :Document ID: [SAFE-COMP-XXX]
   :Version: [1.0]
   :Date: [YYYY-MM-DD]
   :Author: [Name]
   :Safety Level: [ASIL-D|ASIL-C|SIL 3|etc]
   :Status: [Draft|Review|Approved]

   .. warning::
      **Safety-Critical Component**: This documentation is part of the safety case.
      Changes require safety assessment and approval.

   .. contents:: Table of Contents
      :local:
      :depth: 3

   Introduction
   ============

   Purpose
   -------

   This document provides safety documentation for [component name], which is
   classified as a safety-critical component at [ASIL level/SIL level].

   Scope
   -----

   This documentation covers:

   - Safety requirements and their implementation
   - Safety mechanisms and their verification
   - Fault detection and handling
   - Safety-related constraints and assumptions

   References
   ----------

   .. list-table::
      :widths: 20 60 20
      :header-rows: 1

      * - Document ID
        - Title
        - Version
      * - [REF-001]
        - ISO 26262-6:2018
        - 2018
      * - [REF-002]
        - [Project Safety Plan]
        - [Version]

   Safety Requirements
   ===================

   Functional Safety Requirements
   ------------------------------

   **[FSR-001]: [Requirement Name]**

   - **Description:** [What the system shall do]
   - **ASIL:** [A|B|C|D]
   - **Rationale:** [Why this is needed for safety]
   - **Verification:** [How this is verified]

   .. code-block:: text

      Requirement trace:
      System Requirement SR-XXX → FSR-001 → Implementation in [module]

   Technical Safety Requirements
   -----------------------------

   **[TSR-001]: [Technical Requirement Name]**

   - **Parent FSR:** FSR-001
   - **Description:** [Technical implementation requirement]
   - **Implementation:** [Where/how implemented]
   - **Verification Method:** [Test|Analysis|Inspection]

   Safety Analysis
   ===============

   Hazard Analysis
   ---------------

   .. list-table:: Identified Hazards
      :widths: 15 40 15 15 15
      :header-rows: 1

      * - Hazard ID
        - Description
        - Severity
        - Exposure
        - ASIL
      * - [HAZ-001]
        - [Hazard description]
        - [S0-S3]
        - [E0-E4]
        - [QM|A|B|C|D]

   Failure Mode Analysis
   ---------------------

   **Component:** [Component Name]

   .. list-table:: FMEA
      :widths: 20 30 20 15 15
      :header-rows: 1

      * - Failure Mode
        - Effect
        - Detection
        - Severity
        - Mitigation
      * - [Failure 1]
        - [Local/System effect]
        - [How detected]
        - [1-10]
        - [Mitigation strategy]

   Safety Mechanisms
   =================

   Implemented Safety Mechanisms
   -----------------------------

   **[SM-001]: [Mechanism Name]**

   - **Purpose:** [What hazard it mitigates]
   - **Type:** [Error detection|Error correction|Fail-safe]
   - **Coverage:** [Diagnostic coverage percentage]
   - **Latency:** [Detection time requirement]

   **Implementation:**

   .. code-block:: rust

      /// Safety mechanism: [Name]
      /// Diagnostic coverage: [XX]%
      pub fn safety_check() -> Result<()> {
          // Implementation
      }

   **Verification:**

   - Unit test: [test_name]
   - Fault injection: [test_scenario]
   - Formal proof: [if applicable]

   Fault Handling
   --------------

   **Fault Detection Strategy:**

   1. [Detection method 1]
   2. [Detection method 2]

   **Fault Response:**

   .. code-block:: rust

      match detect_fault() {
          Fault::Category1 => safe_state_transition(),
          Fault::Category2 => degraded_operation(),
          Fault::Critical => emergency_shutdown(),
      }

   Design Constraints
   ==================

   Memory Constraints
   ------------------

   - **Stack usage:** Maximum [X] bytes
   - **Heap usage:** [None|Bounded to X bytes]
   - **Static allocation:** [X] bytes

   Timing Constraints
   ------------------

   - **WCET:** [Worst Case Execution Time]
   - **Response time:** Maximum [X]ms
   - **Watchdog timeout:** [X]ms

   Environmental Constraints
   -------------------------

   - **Operating temperature:** [Range]
   - **Memory integrity:** [ECC|Parity|CRC]
   - **Clock source:** [Requirements]

   Assumptions and Dependencies
   ============================

   Safety Assumptions
   ------------------

   .. list-table::
      :widths: 15 60 25
      :header-rows: 1

      * - ID
        - Assumption
        - Validation Responsibility
      * - [ASM-001]
        - [Assumption description]
        - [Integrator|Runtime|Hardware]

   Dependencies on Other Components
   --------------------------------

   - [Component 1]: [Safety requirement on this component]
   - [Component 2]: [Safety requirement on this component]

   Verification and Validation
   ===========================

   Verification Strategy
   ---------------------

   .. list-table:: Verification Matrix
      :widths: 25 25 25 25
      :header-rows: 1

      * - Requirement
        - Method
        - Coverage
        - Evidence
      * - FSR-001
        - Testing
        - MC/DC 100%
        - [Test report ID]
      * - TSR-001
        - Formal verification
        - Complete
        - [Proof ID]

   Test Coverage Requirements
   --------------------------

   - **Statement coverage:** 100%
   - **Branch coverage:** 100%
   - **MC/DC coverage:** 100% for ASIL-D
   - **Fault injection:** [Coverage target]

   Validation Activities
   ---------------------

   1. **Hardware-in-loop testing**
   2. **Field testing** (if applicable)
   3. **Fault injection campaigns**

   Safety Manual Integration
   =========================

   Usage Constraints
   -----------------

   Users of this component **shall**:

   1. [Constraint 1]
   2. [Constraint 2]

   Users of this component **shall not**:

   1. [Prohibition 1]
   2. [Prohibition 2]

   Integration Requirements
   ------------------------

   When integrating this component:

   - [ ] Verify assumption [ASM-001]
   - [ ] Configure [parameter] within safe range
   - [ ] Enable safety mechanism [SM-001]
   - [ ] Validate timing constraints

   Change Management
   =================

   Safety Impact Analysis
   ----------------------

   Any changes to this component require:

   1. Impact analysis on safety requirements
   2. Re-verification of affected requirements
   3. Safety assessment approval
   4. Documentation update

   Version History
   ---------------

   .. list-table::
      :widths: 20 20 60
      :header-rows: 1

      * - Version
        - Date
        - Safety-Relevant Changes
      * - 1.0
        - [Date]
        - Initial safety release

   Appendices
   ==========

   Appendix A: Acronyms
   --------------------

   - **ASIL**: Automotive Safety Integrity Level
   - **FMEA**: Failure Mode and Effects Analysis
   - **FSR**: Functional Safety Requirement
   - **TSR**: Technical Safety Requirement

   Appendix B: Safety Checklist
   ----------------------------

   Development Phase:
   
   - [ ] Safety requirements defined
   - [ ] Safety analysis completed
   - [ ] Safety mechanisms implemented
   - [ ] Code review with safety focus
   
   Verification Phase:
   
   - [ ] Safety tests executed
   - [ ] Coverage targets met
   - [ ] Fault injection completed
   - [ ] Formal verification (if required)
   
   Release Phase:
   
   - [ ] Safety manual updated
   - [ ] Assumptions documented
   - [ ] Safety assessment completed
   - [ ] Traceability confirmed