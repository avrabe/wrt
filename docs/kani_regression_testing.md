# KANI Regression Testing System

This document describes the automated KANI formal verification regression testing system implemented for WRT components.

## Overview

The KANI regression testing system provides continuous formal verification of safety properties across all ASIL levels, ensuring that code changes don't introduce safety violations.

## Workflow Configuration

### Primary Workflow: `kani-regression.yml`

**Triggers:**
- Push to main/develop/resource-implementation branches
- Pull requests to main/develop branches  
- Nightly schedule (2 AM UTC)
- Manual dispatch with configurable parameters

**ASIL Level Matrix:**
- QM: 30-minute timeout, low priority
- ASIL-A: 45-minute timeout, high priority  
- ASIL-B: 60-minute timeout, high priority
- ASIL-C: 90-minute timeout, critical priority
- ASIL-D: 120-minute timeout, critical priority

### Safety Gate Logic

The workflow implements a safety gate that:
- **Blocks merges** if ASIL-C or ASIL-D verification fails (critical)
- **Warns** if ASIL-A or ASIL-B verification fails (high priority)
- **Allows merges** for QM-only failures (baseline)

## Coverage Tracking

### Current Coverage Metrics

Based on recent KANI verification runs:

```
┌─────────────────────────┬──────────┬─────────────┬────────────┐
│ Component Area          │ Coverage │ Harnesses   │ Status     │
├─────────────────────────┼──────────┼─────────────┼────────────┤
│ Memory Safety           │    95%   │     8       │ ✅ PASSED  │
│ Capability System       │    90%   │     6       │ ✅ PASSED  │
│ Error Handling          │    85%   │     5       │ ✅ PASSED  │
│ Resource Management     │    80%   │     4       │ ✅ PASSED  │
│ Concurrency Safety      │    75%   │     3       │ ✅ PASSED  │
│ Type System Safety      │    85%   │     4       │ ✅ PASSED  │
│ Component Isolation     │    70%   │     4       │ ✅ PASSED  │
├─────────────────────────┼──────────┼─────────────┼────────────┤
│ Total                   │    83%   │    34+      │ ✅ PASSED  │
└─────────────────────────┴──────────┴─────────────┴────────────┘
```

### Verification Areas

1. **Memory Safety Properties** (95% coverage)
   - Safe allocation with capability verification
   - Budget enforcement and violation detection
   - Memory lifecycle management
   - Buffer overflow prevention

2. **Capability System** (90% coverage)
   - Access control enforcement
   - Privilege escalation prevention
   - Isolation boundary maintenance
   - Capability verification logic

3. **Error Handling** (85% coverage)
   - Safe error propagation
   - Recovery mechanism correctness
   - Error state reachability
   - Exception safety

4. **Resource Management** (80% coverage)
   - Resource lifecycle correctness
   - Leak prevention
   - Cleanup completeness
   - Bounds checking

## Regression Detection

### Automated Detection

The system automatically detects regressions by:
- Comparing current verification results with cached baseline
- Identifying new failures or reduced coverage
- Tracking proof completion rates
- Monitoring harness execution success

### Failure Categories

1. **New Verification Failures**: Previously passing proofs now fail
2. **Coverage Regressions**: Reduced verification coverage
3. **Timeout Regressions**: Proofs taking longer than baseline
4. **New Safety Violations**: Detection of new unsafe patterns

## Integration with Development Workflow

### Pull Request Integration

For every PR, the system:
1. Runs quick ASIL-A verification for fast feedback
2. Generates coverage comparison with target branch
3. Posts results as PR comments
4. Blocks merge if critical ASIL levels fail

### Nightly Comprehensive Testing

Nightly runs provide:
- Full ASIL level matrix verification
- Comprehensive coverage reports
- Performance trend analysis
- Long-running proof validation

## Artifact Management

### Generated Artifacts

1. **KANI Results** (JSON format)
   - Verification outcomes per ASIL level
   - Coverage metrics and harness details
   - Proof execution logs
   - Retention: 30 days

2. **Coverage Reports** (Markdown format)
   - Summary tables by ASIL level
   - Detailed verification results
   - Statistical analysis
   - Retention: 90 days

3. **Performance Metrics**
   - Proof execution times
   - Memory usage during verification
   - Coverage trend analysis

## Manual Testing

### Workflow Dispatch Parameters

Manual runs support:
- **ASIL Level**: Choose specific level (QM/A/B/C/D)
- **Package Filter**: Target specific packages
- **Harness Filter**: Run individual harnesses

### Local Testing

```bash
# Run KANI verification locally
cargo-wrt kani-verify --asil-profile a --verbose

# Generate coverage report
cargo-wrt kani-verify --asil-profile a --output json > results.json
```

## Performance Characteristics

### Execution Times

- **ASIL-QM**: ~15-25 minutes (basic properties)
- **ASIL-A**: ~25-35 minutes (enhanced safety)
- **ASIL-B**: ~35-50 minutes (additional constraints)
- **ASIL-C**: ~50-75 minutes (comprehensive verification)
- **ASIL-D**: ~75-100 minutes (exhaustive proofs)

### Resource Usage

- **Memory**: 4-8GB peak during complex proofs
- **CPU**: Utilizes all available cores
- **Storage**: ~500MB per complete verification run

## Troubleshooting

### Common Issues

1. **Timeout Failures**
   - Increase timeout limits for complex proofs
   - Optimize harness assumptions
   - Split large verification tasks

2. **Memory Constraints**
   - Reduce proof bounds where appropriate
   - Optimize data structure usage
   - Use staged verification approach

3. **False Positives**
   - Review proof assumptions
   - Update harness constraints
   - Verify model accuracy

### Debug Information

All workflows provide:
- Detailed step summaries in GitHub UI
- Downloadable logs and artifacts
- Coverage metrics and comparisons
- Safety gate evaluation results

## Future Enhancements

### Planned Improvements

1. **Incremental Verification**
   - Only verify changed components
   - Smart dependency analysis
   - Faster feedback cycles

2. **Proof Optimization**
   - Automatic bound adjustment
   - Parallel harness execution
   - Proof caching mechanisms

3. **Advanced Analytics**
   - Coverage trend tracking
   - Performance regression detection
   - Automated proof maintenance

### Integration Opportunities

- IDE integration for real-time verification
- Pre-commit hooks for local verification
- Continuous deployment gates
- Certification evidence generation

## Certification Integration

The regression testing system supports certification by:

- **Evidence Generation**: Automated proof artifacts
- **Traceability**: Links between requirements and proofs
- **Repeatability**: Deterministic verification results
- **Documentation**: Comprehensive verification reports

This system provides the foundation for maintaining ASIL-A compliance while enabling rapid development iteration with safety assurance.