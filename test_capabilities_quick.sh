#!/bin/bash

# test_capabilities_quick.sh - Quick test of key capability builds
# This script tests a subset of crates and ASIL levels for faster feedback

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Subset of key crates to test
CRATES=(
    "wrt-error"      # Base crate
    "wrt-foundation" # Core foundation
    "wrt-runtime"    # Runtime with known issues
    "wrt-wasi"       # Top-level functionality
)

# Subset of ASIL levels - using simple arrays for compatibility
ASIL_LEVELS=("std+qm" "std+asil-b" "asil-d")
ASIL_FEATURES=("--features std,qm" "--features std,asil-b" "--features asil-d")

# Results storage
PASSED=0
FAILED=0
TOTAL=0

echo -e "${YELLOW}=== WRT Capability Quick Test ===${NC}"
echo "Testing ${#CRATES[@]} key crates with 3 ASIL levels"
echo ""

# Test function
test_crate_asil() {
    local crate=$1
    local asil_idx=$2
    local asil_level=${ASIL_LEVELS[$asil_idx]}
    local features=${ASIL_FEATURES[$asil_idx]}
    
    echo -n "  Testing $asil_level... "
    
    if cargo build -p "$crate" --no-default-features $features >/dev/null 2>&1; then
        echo -e "${GREEN}PASSED${NC}"
        ((PASSED++))
    else
        echo -e "${RED}FAILED${NC}"
        ((FAILED++))
    fi
    ((TOTAL++))
}

# Test each crate
for crate in "${CRATES[@]}"; do
    echo -e "\n${YELLOW}Testing $crate${NC}"
    for i in 0 1 2; do
        test_crate_asil "$crate" "$i"
    done
done

# Summary
echo -e "\n${YELLOW}=== Quick Test Summary ===${NC}"
echo -e "Total tests: $TOTAL"
echo -e "Passed: ${GREEN}$PASSED${NC}"
echo -e "Failed: ${RED}$FAILED${NC}"

if [[ $FAILED -eq 0 ]]; then
    echo -e "\n${GREEN}All quick tests passed!${NC}"
else
    echo -e "\n${RED}Some tests failed. Run ./test_all_capabilities.sh for detailed results.${NC}"
    exit 1
fi