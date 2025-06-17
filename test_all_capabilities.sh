#!/bin/bash

# test_all_capabilities.sh - Test all capability builds across all WRT crates
# This script systematically tests each crate with all ASIL safety levels

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# List of all WRT crates to test
CRATES=(
    "wrt-error"
    "wrt-sync"
    "wrt-math"
    "wrt-foundation"
    "wrt-platform"
    "wrt-logging"
    "wrt-host"
    "wrt-format"
    "wrt-instructions"
    "wrt-intercept"
    "wrt-decoder"
    "wrt-debug"
    "wrt-runtime"
    "wrt-component"
    "wrt-wasi"
    "wrtd"
)

# ASIL levels with their std requirements - using arrays for compatibility
ASIL_NAMES=("std+qm" "std+asil-a" "std+asil-b" "asil-c" "asil-d")
ASIL_FEATURES=("--features std,qm" "--features std,asil-a" "--features std,asil-b" "--features asil-c" "--features asil-d")

# Results storage - will use key:value pairs in a simple array
declare -a RESULTS
PASSED=0
FAILED=0
TOTAL=0

# Create log directory
LOG_DIR="build_test_logs"
mkdir -p "$LOG_DIR"
SUMMARY_FILE="$LOG_DIR/summary_$(date +%Y%m%d_%H%M%S).txt"

# Function to test a single crate with a single ASIL level
test_crate_asil() {
    local crate=$1
    local asil_level=$2
    local features=${ASIL_CONFIGS[$asil_level]}
    local log_file="$LOG_DIR/${crate}_${asil_level}.log"
    
    echo -n "  Testing $asil_level... "
    
    if cargo build -p "$crate" --no-default-features $features > "$log_file" 2>&1; then
        echo -e "${GREEN}PASSED${NC}"
        RESULTS["$crate:$asil_level"]="PASSED"
        ((PASSED++))
        return 0
    else
        echo -e "${RED}FAILED${NC}"
        RESULTS["$crate:$asil_level"]="FAILED"
        ((FAILED++))
        # Extract error summary
        echo "    Error summary:" >> "$SUMMARY_FILE"
        grep -E "^error" "$log_file" | head -5 | sed 's/^/      /' >> "$SUMMARY_FILE"
        return 1
    fi
}

# Function to test all ASIL levels for a crate
test_crate() {
    local crate=$1
    echo -e "\n${YELLOW}Testing $crate${NC}"
    echo "Testing $crate" >> "$SUMMARY_FILE"
    
    for asil_level in "std+qm" "std+asil-a" "std+asil-b" "asil-c" "asil-d"; do
        test_crate_asil "$crate" "$asil_level"
        ((TOTAL++))
    done
}

# Main test execution
echo -e "${YELLOW}=== WRT Capability Build Test Suite ===${NC}"
echo "Testing ${#CRATES[@]} crates with 5 ASIL levels each"
echo "Logs will be saved to: $LOG_DIR"
echo "Summary will be saved to: $SUMMARY_FILE"
echo ""

# Write summary header
{
    echo "WRT Capability Build Test Results"
    echo "================================="
    echo "Date: $(date)"
    echo ""
} > "$SUMMARY_FILE"

# Test each crate
for crate in "${CRATES[@]}"; do
    test_crate "$crate"
done

# Generate summary report
echo -e "\n${YELLOW}=== Test Summary ===${NC}"
echo -e "Total tests: $TOTAL"
echo -e "Passed: ${GREEN}$PASSED${NC}"
echo -e "Failed: ${RED}$FAILED${NC}"
echo -e "Success rate: $(( PASSED * 100 / TOTAL ))%"

# Add to summary file
{
    echo ""
    echo "Overall Summary"
    echo "==============="
    echo "Total tests: $TOTAL"
    echo "Passed: $PASSED"
    echo "Failed: $FAILED"
    echo "Success rate: $(( PASSED * 100 / TOTAL ))%"
    echo ""
} >> "$SUMMARY_FILE"

# Generate per-crate summary
echo -e "\n${YELLOW}=== Per-Crate Summary ===${NC}"
{
    echo "Per-Crate Summary"
    echo "================="
} >> "$SUMMARY_FILE"

for crate in "${CRATES[@]}"; do
    local crate_passed=0
    local crate_total=5
    
    for asil_level in "std+qm" "std+asil-a" "std+asil-b" "asil-c" "asil-d"; do
        if [[ ${RESULTS["$crate:$asil_level"]} == "PASSED" ]]; then
            ((crate_passed++))
        fi
    done
    
    if [[ $crate_passed -eq $crate_total ]]; then
        echo -e "$crate: ${GREEN}$crate_passed/$crate_total${NC} ✅"
        echo "$crate: $crate_passed/$crate_total ✅" >> "$SUMMARY_FILE"
    elif [[ $crate_passed -eq 0 ]]; then
        echo -e "$crate: ${RED}$crate_passed/$crate_total${NC} ❌"
        echo "$crate: $crate_passed/$crate_total ❌" >> "$SUMMARY_FILE"
    else
        echo -e "$crate: ${YELLOW}$crate_passed/$crate_total${NC} ⚠️"
        echo "$crate: $crate_passed/$crate_total ⚠️" >> "$SUMMARY_FILE"
    fi
done

# Generate per-ASIL summary
echo -e "\n${YELLOW}=== Per-ASIL Level Summary ===${NC}"
{
    echo ""
    echo "Per-ASIL Level Summary"
    echo "====================="
} >> "$SUMMARY_FILE"

for asil_level in "std+qm" "std+asil-a" "std+asil-b" "asil-c" "asil-d"; do
    local asil_passed=0
    local asil_total=${#CRATES[@]}
    
    for crate in "${CRATES[@]}"; do
        if [[ ${RESULTS["$crate:$asil_level"]} == "PASSED" ]]; then
            ((asil_passed++))
        fi
    done
    
    echo -e "$asil_level: $asil_passed/$asil_total passed"
    echo "$asil_level: $asil_passed/$asil_total passed" >> "$SUMMARY_FILE"
done

# List failed builds
if [[ $FAILED -gt 0 ]]; then
    echo -e "\n${YELLOW}=== Failed Builds ===${NC}"
    {
        echo ""
        echo "Failed Builds"
        echo "============="
    } >> "$SUMMARY_FILE"
    
    for key in "${!RESULTS[@]}"; do
        if [[ ${RESULTS[$key]} == "FAILED" ]]; then
            echo -e "${RED}$key${NC}"
            echo "$key" >> "$SUMMARY_FILE"
        fi
    done
fi

echo -e "\n${YELLOW}Full test results saved to: $SUMMARY_FILE${NC}"
echo -e "${YELLOW}Individual build logs saved to: $LOG_DIR/${NC}"

# Exit with error if any tests failed
if [[ $FAILED -gt 0 ]]; then
    exit 1
fi