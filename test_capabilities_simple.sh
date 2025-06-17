#!/bin/bash

# test_capabilities_simple.sh - Simple test of capability builds
# Compatible version that works with older bash

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

# Counters
PASSED=0
FAILED=0

echo -e "${YELLOW}=== WRT Capability Build Test ===${NC}"
echo ""

# Test a single configuration
test_build() {
    local crate=$1
    local features=$2
    local desc=$3
    
    printf "%-20s %-15s " "$crate" "$desc"
    
    if cargo build -p "$crate" --no-default-features $features >/dev/null 2>&1; then
        echo -e "${GREEN}PASSED${NC}"
        PASSED=$((PASSED + 1))
    else
        echo -e "${RED}FAILED${NC}"
        FAILED=$((FAILED + 1))
    fi
}

# Test key crates
echo "Testing base crates..."
test_build "wrt-error" "--features std,qm" "std+qm"
test_build "wrt-error" "--features asil-d" "asil-d"

echo ""
echo "Testing foundation..."
test_build "wrt-foundation" "--features std,qm" "std+qm"
test_build "wrt-foundation" "--features std,asil-b" "std+asil-b"
test_build "wrt-foundation" "--features asil-c" "asil-c"
test_build "wrt-foundation" "--features asil-d" "asil-d"

echo ""
echo "Testing runtime..."
test_build "wrt-runtime" "--features std,qm" "std+qm"
test_build "wrt-runtime" "--features std,asil-b" "std+asil-b"
test_build "wrt-runtime" "--features asil-d" "asil-d"

echo ""
echo "Testing host..."
test_build "wrt-host" "--features std,qm" "std+qm"
test_build "wrt-host" "--features asil-d" "asil-d"

echo ""
echo "Testing decoder..."
test_build "wrt-decoder" "--features std,qm" "std+qm"
test_build "wrt-decoder" "--features asil-d" "asil-d"

echo ""
echo "Testing wrtd..."
test_build "wrtd" "--features std,qm" "std+qm"
test_build "wrtd" "--features asil-d" "asil-d"

# Summary
TOTAL=$((PASSED + FAILED))
echo ""
echo -e "${YELLOW}=== Summary ===${NC}"
echo "Total tests: $TOTAL"
echo -e "Passed: ${GREEN}$PASSED${NC}"
echo -e "Failed: ${RED}$FAILED${NC}"

if [ $FAILED -eq 0 ]; then
    echo -e "\n${GREEN}All tests passed!${NC}"
    exit 0
else
    echo -e "\n${RED}Some tests failed.${NC}"
    exit 1
fi