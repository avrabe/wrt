Safety Verification Status
===========================

.. raw:: html

   <div class="safety-status-card">
     <div class="safety-header">
       <h3>🛡️ WRT Safety Verification Dashboard</h3>
       <span class="timestamp">Last Updated: 2025-06-07T03:53:16.274847+00:00</span>
     </div>
   </div>

Current Safety Status
---------------------

.. list-table:: ASIL Compliance Overview
   :widths: 20 20 20 20 20
   :header-rows: 1

   * - ASIL Level
     - Current Coverage
     - Required Coverage
     - Status
     - Gap
   * - QM
     - 100.0%
     - 70.0%
     - ✅ PASS
     - 0.0%
   * - AsilA
     - 95.0%
     - 80.0%
     - ✅ PASS
     - 0.0%
   * - AsilB
     - 85.0%
     - 90.0%
     - ❌ FAIL
     - 5.0%
   * - AsilC
     - 75.0%
     - 90.0%
     - ❌ FAIL
     - 15.0%
   * - AsilD
     - 60.0%
     - 95.0%
     - ❌ FAIL
     - 35.0%

.. note::
   🎯 **Overall Certification Readiness: 76.4%**
   
   Status: Approaching readiness - address key gaps

Requirements Traceability
-------------------------

.. list-table:: Requirements by Category
   :widths: 30 70
   :header-rows: 1

   * - Category
     - Count
   * - ASIL AsilC
     - 3 requirements
   * - ASIL AsilD
     - 1 requirements
   * - ASIL AsilB
     - 2 requirements
   * - Memory Requirements
     - 1 requirements
   * - Component Requirements
     - 1 requirements
   * - Parse Requirements
     - 1 requirements
   * - System Requirements
     - 1 requirements
   * - Runtime Requirements
     - 1 requirements
   * - Safety Requirements
     - 1 requirements

Test Coverage Status
--------------------

.. list-table:: Test Coverage Analysis
   :widths: 25 25 25 25
   :header-rows: 1

   * - Test Category
     - Coverage %
     - Test Count
     - Status
   * - Unit Tests
     - 87.5%
     - 156
     - ✅ Good
   * - Integration Tests
     - 72.3%
     - 89
     - ⚠️ Warning
   * - ASIL-Tagged Tests
     - 68.1%
     - 34
     - ❌ Poor
   * - Safety Tests
     - 91.2%
     - 23
     - ✅ Good
   * - Component Tests
     - 83.7%
     - 67
     - ✅ Good

✅ All referenced files exist

Quick Actions
-------------

To update this status or get detailed reports:

.. code-block:: bash

   # Update safety status
   just safety-dashboard
   
   # Generate detailed report
   cargo xtask verify-safety --format html --output safety-report.html
   
   # Check specific requirements
   cargo xtask verify-requirements --detailed

For complete safety verification documentation, see :doc:`developer/tooling/safety_verification`.

.. raw:: html

   <style>
   .safety-status-card {
     background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
     color: white;
     padding: 1rem;
     border-radius: 8px;
     margin: 1rem 0;
   }
   .safety-header {
     display: flex;
     justify-content: space-between;
     align-items: center;
   }
   .safety-header h3 {
     margin: 0;
     color: white;
   }
   .timestamp {
     font-size: 0.9em;
     opacity: 0.9;
   }
   </style>
