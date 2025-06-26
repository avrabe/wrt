# Deprecated Memory Patterns Migration Guide

## Overview

This guide documents all deprecated memory patterns in the WRT project and their modern replacements for ASIL-D compliance.

## Deprecated Patterns and Replacements

### 1. ❌ Direct NoStdProvider Construction
```rust
// ❌ DEPRECATED - Bypasses capability system
let provider = NoStdProvider::new();
let provider = NoStdProvider::with_verification_level(level);
let provider = NoStdProvider::default(); // Only when used directly

// ✅ CORRECT - Use safe_managed_alloc! macro
let provider = safe_managed_alloc!(size, CrateId::Component)?;
```

### 2. ❌ BudgetProvider Pattern
```rust
// ❌ DEPRECATED - Old budget system
let provider = BudgetProvider::new(CrateId::Component)?;

// ✅ CORRECT - Unified allocation
let provider = safe_managed_alloc!(4096, CrateId::Component)?;
```

### 3. ❌ CapabilityMemoryFactory
```rust
// ❌ DEPRECATED - Complex API
#[deprecated(note = "Use MemoryFactory for simpler memory provider creation")]
let factory = CapabilityMemoryFactory::new(context);
let provider = factory.create_provider(crate_id)?;

// ✅ CORRECT - Use MemoryFactory
use wrt_foundation::capabilities::MemoryFactory;
let provider = MemoryFactory::create::<N>(crate_id)?;
```

### 4. ❌ SafeProviderFactory
```rust
// ❌ DEPRECATED - Removed entirely
// SafeProviderFactory has been removed

// ✅ CORRECT - Use MemoryFactory
use wrt_foundation::capabilities::MemoryFactory;
let provider = MemoryFactory::create::<N>(crate_id)?;
```

### 5. ❌ WrtProviderFactory
```rust
// ❌ DEPRECATED - Legacy factory removed
// WrtProviderFactory has been removed

// ✅ CORRECT - Use CapabilityWrtFactory (NOT deprecated)
use wrt_foundation::wrt_memory_system::CapabilityWrtFactory;
let provider = CapabilityWrtFactory::create_provider::<N>(crate_id)?;
```

## Current Approved Patterns

### ✅ Pattern 1: safe_managed_alloc! Macro (Recommended)
```rust
use wrt_foundation::{safe_managed_alloc, budget_aware_provider::CrateId};

// Primary recommended pattern
let provider = safe_managed_alloc!(4096, CrateId::Component)?;
let vec = BoundedVec::new(provider)?;
```

### ✅ Pattern 2: MemoryFactory (For Advanced Use)
```rust
use wrt_foundation::capabilities::MemoryFactory;

// Direct factory usage
let provider = MemoryFactory::create::<4096>(CrateId::Component)?;

// With custom context
let provider = MemoryFactory::create_with_context::<4096>(&context, CrateId::Component)?;
```

### ✅ Pattern 3: CapabilityWrtFactory (For Capability-Guarded Providers)
```rust
use wrt_foundation::wrt_memory_system::CapabilityWrtFactory;

// Creates a CapabilityGuardedProvider
let guarded_provider = CapabilityWrtFactory::create_provider::<4096>(CrateId::Component)?;

// Or use the macro
let guarded_provider = wrt_provider!(4096, CrateId::Component)?;
```

## Migration Checklist

### For Each Component:

- [ ] Search for `NoStdProvider::new()` and replace with `safe_managed_alloc!`
- [ ] Search for `NoStdProvider::default()` direct usage and replace
- [ ] Search for `BudgetProvider` and replace with `safe_managed_alloc!`
- [ ] Search for `CapabilityMemoryFactory` and replace with `MemoryFactory`
- [ ] Search for `SafeProviderFactory` (should be none - it's removed)
- [ ] Search for `WrtProviderFactory` (should be none - it's removed)
- [ ] Verify all allocations use one of the approved patterns

## Type Definitions Still Valid

These type aliases are still valid and not deprecated:
```rust
// These are type aliases, not constructors - OK to use
type MyProvider = NoStdProvider<4096>;
type ComponentVec<T> = BoundedVec<T, 1024, NoStdProvider<4096>>;
```

## Special Cases

### NoStdProvider::default() in MemoryFactory
The `MemoryFactory::create()` method internally uses `NoStdProvider::default()`. This is acceptable because:
1. It's within the capability verification system
2. The allocation is verified before provider creation
3. This avoids circular dependency

### wrt-platform Exception
The `wrt-platform` crate has its own `NoStdProvider` implementation to avoid cyclic dependency with `wrt-foundation`. This is documented and acceptable for ASIL-B compliance.

## Verification Commands

### Find Deprecated Patterns
```bash
# Direct construction patterns
rg "NoStdProvider::new\(\)" --type rust
rg "NoStdProvider::default\(\)" --type rust | grep -v "MemoryFactory"
rg "BudgetProvider::new" --type rust

# Deprecated factories
rg "CapabilityMemoryFactory::new" --type rust
rg "SafeProviderFactory" --type rust
rg "WrtProviderFactory::new" --type rust
```

### Verify Compliance
```bash
# Run build verification
cargo wrt verify --asil d

# Check for deprecation warnings
cargo check 2>&1 | grep deprecated
```

## Summary

The migration to capability-based allocation is complete with these guidelines:

1. **Primary Pattern**: Use `safe_managed_alloc!` macro for most cases
2. **Advanced Pattern**: Use `MemoryFactory` for custom contexts
3. **Capability Guards**: Use `CapabilityWrtFactory` when needed
4. **No Direct Construction**: Never use `NoStdProvider::new()` or `::default()` directly
5. **No Legacy Factories**: All old factory patterns are deprecated or removed

All components following these patterns will meet ASIL-D compliance requirements for memory safety.