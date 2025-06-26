# WRT ASIL Compliance Matrix and Memory Migration Guide

## Overview

This document provides a comprehensive compliance matrix for ASIL levels QM through ASIL-D and tracks the migration status from deprecated memory patterns to the capability-based allocation system.

## ASIL Compliance Matrix

### Component Safety Level Requirements

| Component | Current Level | Target Level | Memory Pattern | Status |
|-----------|--------------|--------------|----------------|---------|
| **wrt-foundation** | ASIL-D | ASIL-D | Capability-based | ✅ Ready |
| **wrt-component** | ASIL-C | ASIL-D | Mixed | 🔄 Migration needed |
| **wrt-runtime** | ASIL-C | ASIL-D | Mixed | 🔄 Migration needed |
| **wrt-host** | ASIL-B | ASIL-C | Legacy | ⚠️ Major refactor |
| **wrt-wasi** | ASIL-A | ASIL-B | Legacy | ⚠️ Major refactor |
| **wrt-decoder** | QM | ASIL-A | Legacy | ⚠️ Major refactor |
| **wrt-format** | QM | ASIL-A | Legacy | ⚠️ Major refactor |
| **wrt-platform** | ASIL-B | ASIL-B | Independent | ⚠️ Architectural |

### Safety Level Requirements

#### QM (Quality Management)
- Basic memory safety
- No specific safety requirements
- Standard Rust safety guarantees

#### ASIL-A
- Fault detection capability
- Error classification and propagation
- Basic runtime monitoring
- Memory budget tracking

#### ASIL-B
- All ASIL-A requirements plus:
- Deterministic memory allocation
- Performance monitoring
- Enhanced error recovery

#### ASIL-C
- All ASIL-B requirements plus:
- Formal verification of safety properties
- Capability-based access control
- Runtime safety monitoring

#### ASIL-D
- All ASIL-C requirements plus:
- Complete formal verification
- Zero unsafe code in critical paths
- Comprehensive telemetry
- Fault isolation

## Deprecated Memory Pattern Migration

### Patterns to Replace

#### 1. Direct NoStdProvider Construction
```rust
// ❌ DEPRECATED - Bypasses capability system
let provider = NoStdProvider::new();
let provider = NoStdProvider::with_verification_level(level);
let provider = NoStdProvider::default();

// ✅ CORRECT - Uses capability-based allocation
let provider = safe_managed_alloc!(size, CrateId::Component)?;
```

#### 2. BudgetProvider Usage
```rust
// ❌ DEPRECATED - Old budget system
let provider = BudgetProvider::new(CrateId::Component)?;

// ✅ CORRECT - Unified allocation
let provider = safe_managed_alloc!(4096, CrateId::Component)?;
```

#### 3. Direct Factory Usage
```rust
// ❌ DEPRECATED - Low-level factory
let factory = WrtProviderFactory::new();
let provider = factory.create_provider(size)?;

// ✅ CORRECT - Capability factory
use wrt_foundation::capabilities::CapabilityWrtFactory;
let provider = CapabilityWrtFactory::create_provider(size, CrateId::Component)?;
```

## Migration Status by Component

### ✅ wrt-foundation (ASIL-D Ready)
- Fully migrated to capability-based allocation
- Safety monitor integrated
- Telemetry system implemented
- KANI verification: 83% coverage

### 🔄 wrt-component (Needs Migration)
**Files requiring updates:**
- `src/component.rs` - Uses NoStdProvider::default()
- `src/instance.rs` - Direct provider construction
- `src/memory.rs` - Legacy allocation patterns

**Migration steps:**
1. Replace all NoStdProvider::default() with safe_managed_alloc!
2. Update tests to use capability-based allocation
3. Add safety monitoring hooks
4. Implement ASIL-D verification harnesses

### 🔄 wrt-runtime (Needs Migration)
**Files requiring updates:**
- `src/runtime.rs` - Mixed allocation patterns
- `src/store.rs` - Direct provider usage
- `src/engine.rs` - Legacy memory management

**Migration steps:**
1. Audit all memory allocation paths
2. Replace with capability-based allocation
3. Add runtime safety monitoring
4. Implement formal verification

### ⚠️ wrt-host (Major Refactor)
**Current issues:**
- Heavy use of legacy patterns
- No capability awareness
- Missing safety monitoring

**Required changes:**
1. Complete memory subsystem rewrite
2. Implement capability checks
3. Add ASIL-C verification
4. Integrate telemetry

### ⚠️ wrt-wasi (Major Refactor)
**Current issues:**
- Direct memory manipulation
- No budget tracking
- Legacy error handling

**Required changes:**
1. Implement capability-aware WASI
2. Add memory budget enforcement
3. Implement fault detection
4. Add ASIL-B compliance

### ⚠️ wrt-decoder (Major Refactor)
**Current issues:**
- Fixed in some files, but widespread legacy usage
- No safety monitoring
- Missing verification

**Required changes:**
1. Complete migration to safe_managed_alloc!
2. Add input validation
3. Implement ASIL-A error handling
4. Add verification harnesses

### ⚠️ wrt-platform (Architectural Issue)
**Special case:**
- Has its own NoStdProvider to avoid cyclic dependency
- Cannot depend on wrt-foundation
- Needs architectural solution

**Potential solutions:**
1. Extract common memory types to separate crate
2. Use trait-based abstraction
3. Implement capability system at platform level

## Implementation Priority

### Phase 1: Critical Path (Week 1-2)
1. **wrt-component** - Core runtime component
2. **wrt-runtime** - Execution engine
3. Integration testing with safety monitor

### Phase 2: WASI Integration (Week 3-4)
1. **wrt-wasi** - WASI implementation
2. **wrt-host** - Host interface
3. End-to-end safety verification

### Phase 3: Input Processing (Week 5-6)
1. **wrt-decoder** - WebAssembly decoder
2. **wrt-format** - Format handling
3. Fuzz testing with safety checks

### Phase 4: Architecture (Week 7-8)
1. **wrt-platform** - Resolve cyclic dependency
2. Cross-component verification
3. Full ASIL-D compliance testing

## Verification Requirements

### Per-Component Verification

Each component must have:
1. **Unit tests** using capability-based allocation
2. **Integration tests** with safety monitoring
3. **KANI harnesses** for formal verification
4. **Fuzz tests** for robustness
5. **Performance benchmarks** with telemetry

### System-Level Verification

1. **End-to-end safety tests**
2. **Cross-component isolation verification**
3. **Fault injection testing**
4. **Performance regression tests**
5. **ASIL compliance validation**

## Automated Migration Tools

### Migration Script
```bash
#!/bin/bash
# migrate_memory_patterns.sh

# Find and report deprecated patterns
echo "=== Deprecated Pattern Report ==="
rg "NoStdProvider::new\(\)" --type rust
rg "NoStdProvider::default\(\)" --type rust
rg "BudgetProvider::new" --type rust
rg "WrtProviderFactory" --type rust

# Generate migration patches
echo "=== Generating Migration Patches ==="
for file in $(rg -l "NoStdProvider::default\(\)" --type rust); do
    echo "File: $file"
    # Create patch file
    sed 's/NoStdProvider::default()/safe_managed_alloc!(4096, CrateId::Component)?/g' "$file" > "$file.patch"
done
```

### Verification Command
```bash
# Run after migration
cargo wrt verify --asil d --all-features
```

## Success Criteria

### Component Level
- [ ] No deprecated memory patterns
- [ ] All allocations use safe_managed_alloc!
- [ ] Safety monitoring integrated
- [ ] KANI verification passing
- [ ] Zero unsafe code in critical paths

### System Level
- [ ] Full ASIL-D compliance
- [ ] 90%+ KANI coverage
- [ ] All components at target ASIL level
- [ ] Telemetry operational
- [ ] CI/CD verification passing

## Conclusion

The WRT project requires comprehensive migration from legacy memory patterns to achieve full ASIL-D compliance. The capability-based allocation system is ready in wrt-foundation, but significant work remains to migrate all components. Priority should be given to critical runtime components (wrt-component, wrt-runtime) followed by interface layers and input processing.