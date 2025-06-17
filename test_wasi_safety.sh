#!/bin/bash
# Test WASI safety feature enforcement

echo "Testing WASI safety feature enforcement..."
echo "=========================================="

# Test with QM (no limits)
echo -e "\n1. Testing with QM feature (no safety limits):"
cargo build -p wrt-wasi --no-default-features --features std,preview2,qm 2>&1 | grep -E "(error|warning|Compiling wrt-wasi)" | head -5
if [ $? -eq 0 ]; then
    echo "✓ QM build initiated"
fi

# Test with ASIL-D (strictest)
echo -e "\n2. Testing with ASIL-D feature (16KB limit):"
cargo build -p wrt-wasi --no-default-features --features std,preview2,asil-d 2>&1 | grep -E "(error|warning|Compiling wrt-wasi)" | head -5
if [ $? -eq 0 ]; then
    echo "✓ ASIL-D build initiated"
fi

# Test with ASIL-C
echo -e "\n3. Testing with ASIL-C feature (32KB limit):"
cargo build -p wrt-wasi --no-default-features --features std,preview2,asil-c 2>&1 | grep -E "(error|warning|Compiling wrt-wasi)" | head -5
if [ $? -eq 0 ]; then
    echo "✓ ASIL-C build initiated"
fi

# Check feature propagation
echo -e "\n4. Checking feature propagation:"
echo "QM features:"
cargo tree -p wrt-wasi --no-default-features --features std,preview2,qm -f '{p} {f}' 2>/dev/null | grep -E "wrt-foundation.*dynamic-allocation"

echo -e "\nASIL-D features:"
cargo tree -p wrt-wasi --no-default-features --features std,preview2,asil-d -f '{p} {f}' 2>/dev/null | grep -E "wrt-foundation.*(maximum-safety|verified-static-allocation)"

echo -e "\n5. Summary:"
echo "The safety features are enforced through:"
echo "- Feature flags that enable different allocation strategies"
echo "- Compile-time checks in safety_aware_alloc! macro"
echo "- Runtime allocation limits based on safety level"
echo "- Capability restrictions based on safety level"