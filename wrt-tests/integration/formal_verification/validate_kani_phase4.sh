#!/bin/bash
# KANI Phase 4 Validation Script
# This script validates the structure and completeness of KANI Phase 4 implementation

echo "=== KANI Phase 4 Validation ==="
echo

# Check module files exist
echo "1. Checking module files..."
modules=(
    "concurrency_proofs.rs"
    "resource_lifecycle_proofs.rs"
    "integration_proofs.rs"
)

for module in "${modules[@]}"; do
    if [ -f "$module" ]; then
        echo "✅ $module exists"
        # Count KANI proofs
        kani_proofs=$(grep -c "#\[kani::proof\]" "$module" || echo 0)
        # Count verify functions
        verify_funcs=$(grep -c "pub fn verify_" "$module" || echo 0)
        # Count test functions
        test_funcs=$(grep -c "fn test_" "$module" || echo 0)
        echo "   - KANI proofs: $kani_proofs"
        echo "   - Verify functions: $verify_funcs"
        echo "   - Test functions: $test_funcs"
    else
        echo "❌ $module missing"
    fi
done

echo
echo "2. Checking property counts..."

# Extract property counts
concurrency_count=$(grep -A1 "pub fn property_count()" concurrency_proofs.rs | tail -1 | grep -o '[0-9]\+' || echo 0)
resource_count=$(grep -A1 "pub fn property_count()" resource_lifecycle_proofs.rs | tail -1 | grep -o '[0-9]\+' || echo 0)
integration_count=$(grep -A1 "pub fn property_count()" integration_proofs.rs | tail -1 | grep -o '[0-9]\+' || echo 0)

echo "✅ Concurrency proofs: $concurrency_count properties"
echo "✅ Resource lifecycle proofs: $resource_count properties"
echo "✅ Integration proofs: $integration_count properties"

total_phase4=$((concurrency_count + resource_count + integration_count))
echo "✅ Total KANI Phase 4 properties: $total_phase4"

echo
echo "3. Checking module exports in mod.rs..."
if grep -q "pub mod concurrency_proofs" mod.rs && \
   grep -q "pub mod resource_lifecycle_proofs" mod.rs && \
   grep -q "pub mod integration_proofs" mod.rs; then
    echo "✅ All Phase 4 modules properly exported"
else
    echo "❌ Missing module exports in mod.rs"
fi

echo
echo "4. Checking TestRegistry integration..."
for module in "${modules[@]}"; do
    if grep -q "pub fn register_tests" "$module"; then
        echo "✅ $module has TestRegistry integration"
    else
        echo "❌ $module missing TestRegistry integration"
    fi
done

echo
echo "5. Summary Statistics:"
echo "===================="
# Count all KANI proofs across all modules
total_kani_proofs=$(grep -c "#\[kani::proof\]" *.rs | awk -F: '{sum += $2} END {print sum}')
# Count all verify functions
total_verify_funcs=$(grep -c "pub fn verify_" *.rs | awk -F: '{sum += $2} END {print sum}')
# Count all property counts
total_properties=$(grep -A1 "pub fn property_count()" *.rs | grep -E "^\s*[0-9]+" | awk '{sum += $1} END {print sum}')

echo "Total KANI proofs: $total_kani_proofs"
echo "Total verify functions: $total_verify_funcs" 
echo "Total formal properties: $total_properties"

echo
echo "=== KANI Phase 4 Validation Complete ==="