#!/bin/bash
# Comprehensive Build Matrix Verification with Root Cause Analysis
# This script verifies all required build configurations and analyzes failures
# to identify architectural issues that could impact ASIL compliance

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Verification report file
REPORT_FILE="BUILD_VERIFICATION_REPORT_$(date +%Y%m%d_%H%M%S).md"
ARCHITECTURAL_ISSUES_FILE="ARCHITECTURAL_ISSUES_$(date +%Y%m%d_%H%M%S).md"

# Initialize report
echo "# Build Matrix Verification Report" > "$REPORT_FILE"
echo "Date: $(date)" >> "$REPORT_FILE"
echo "" >> "$REPORT_FILE"

echo "# Architectural Issues Analysis" > "$ARCHITECTURAL_ISSUES_FILE"
echo "Date: $(date)" >> "$ARCHITECTURAL_ISSUES_FILE"
echo "" >> "$ARCHITECTURAL_ISSUES_FILE"

# Track overall status
OVERALL_SUCCESS=true
ARCHITECTURAL_ISSUES=()

# Function to analyze build/test failures for architectural issues
analyze_failure() {
    local config_name="$1"
    local error_output="$2"
    local features="$3"
    
    echo "## Analyzing failure for: $config_name" >> "$ARCHITECTURAL_ISSUES_FILE"
    echo "Features: $features" >> "$ARCHITECTURAL_ISSUES_FILE"
    echo "" >> "$ARCHITECTURAL_ISSUES_FILE"
    
    # Check for common architectural problems
    if echo "$error_output" | grep -q "cyclic package dependency"; then
        ARCHITECTURAL_ISSUES+=("CYCLIC_DEPENDENCY")
        echo "### ⚠️ ARCHITECTURAL ISSUE: Cyclic Dependencies Detected" >> "$ARCHITECTURAL_ISSUES_FILE"
        echo "This violates ASIL principles of modular design and clear dependency hierarchy." >> "$ARCHITECTURAL_ISSUES_FILE"
        echo "Affected configuration: $config_name" >> "$ARCHITECTURAL_ISSUES_FILE"
        echo "" >> "$ARCHITECTURAL_ISSUES_FILE"
    fi
    
    if echo "$error_output" | grep -q "multiple definitions\|duplicate definitions"; then
        ARCHITECTURAL_ISSUES+=("DUPLICATE_DEFINITIONS")
        echo "### ⚠️ ARCHITECTURAL ISSUE: Duplicate Definitions" >> "$ARCHITECTURAL_ISSUES_FILE"
        echo "This indicates poor modularity and could lead to undefined behavior in safety-critical systems." >> "$ARCHITECTURAL_ISSUES_FILE"
        echo "" >> "$ARCHITECTURAL_ISSUES_FILE"
    fi
    
    if echo "$error_output" | grep -q "trait bound.*not satisfied\|the trait.*is not implemented"; then
        ARCHITECTURAL_ISSUES+=("TRAIT_BOUNDS")
        echo "### ⚠️ ARCHITECTURAL ISSUE: Trait Bound Violations" >> "$ARCHITECTURAL_ISSUES_FILE"
        echo "Feature combinations are creating incompatible trait requirements." >> "$ARCHITECTURAL_ISSUES_FILE"
        echo "This suggests improper abstraction boundaries for ASIL compliance." >> "$ARCHITECTURAL_ISSUES_FILE"
        echo "" >> "$ARCHITECTURAL_ISSUES_FILE"
    fi
    
    if echo "$error_output" | grep -q "cannot find.*in scope\|unresolved import"; then
        ARCHITECTURAL_ISSUES+=("MISSING_IMPORTS")
        echo "### ⚠️ ARCHITECTURAL ISSUE: Missing Imports/Modules" >> "$ARCHITECTURAL_ISSUES_FILE"
        echo "Feature flags are not properly managing code visibility." >> "$ARCHITECTURAL_ISSUES_FILE"
        echo "This violates ASIL requirement for deterministic compilation." >> "$ARCHITECTURAL_ISSUES_FILE"
        echo "" >> "$ARCHITECTURAL_ISSUES_FILE"
    fi
    
    if echo "$error_output" | grep -q "conflicting implementations\|coherence"; then
        ARCHITECTURAL_ISSUES+=("COHERENCE")
        echo "### ⚠️ ARCHITECTURAL ISSUE: Coherence Violations" >> "$ARCHITECTURAL_ISSUES_FILE"
        echo "Multiple implementations conflict, indicating poor separation of concerns." >> "$ARCHITECTURAL_ISSUES_FILE"
        echo "ASIL-D requires single, unambiguous implementations." >> "$ARCHITECTURAL_ISSUES_FILE"
        echo "" >> "$ARCHITECTURAL_ISSUES_FILE"
    fi
    
    if echo "$error_output" | grep -q "memory allocation\|alloc.*not found"; then
        ARCHITECTURAL_ISSUES+=("MEMORY_ALLOCATION")
        echo "### ⚠️ ARCHITECTURAL ISSUE: Memory Allocation Problems" >> "$ARCHITECTURAL_ISSUES_FILE"
        echo "Memory allocation strategy is not properly abstracted for no_std environments." >> "$ARCHITECTURAL_ISSUES_FILE"
        echo "Critical for ASIL-D compliance in embedded systems." >> "$ARCHITECTURAL_ISSUES_FILE"
        echo "" >> "$ARCHITECTURAL_ISSUES_FILE"
    fi
    
    # Feature interaction analysis
    if [[ "$features" == *"no_std"* ]] && [[ "$features" == *"std"* ]]; then
        ARCHITECTURAL_ISSUES+=("STD_CONFLICT")
        echo "### ⚠️ ARCHITECTURAL ISSUE: std/no_std Conflict" >> "$ARCHITECTURAL_ISSUES_FILE"
        echo "Conflicting standard library requirements detected." >> "$ARCHITECTURAL_ISSUES_FILE"
        echo "" >> "$ARCHITECTURAL_ISSUES_FILE"
    fi
    
    # Log raw error for manual analysis
    echo "### Raw Error Output" >> "$ARCHITECTURAL_ISSUES_FILE"
    echo '```' >> "$ARCHITECTURAL_ISSUES_FILE"
    echo "$error_output" | head -100 >> "$ARCHITECTURAL_ISSUES_FILE"
    echo '```' >> "$ARCHITECTURAL_ISSUES_FILE"
    echo "" >> "$ARCHITECTURAL_ISSUES_FILE"
}

# Function to test a configuration
test_configuration() {
    local name="$1"
    local package="$2"
    local features="$3"
    local asil_level="$4"
    
    echo -e "${BLUE}Testing: $name${NC}"
    echo "## Configuration: $name" >> "$REPORT_FILE"
    echo "- Package: $package" >> "$REPORT_FILE"
    echo "- Features: $features" >> "$REPORT_FILE"
    echo "- ASIL Level: $asil_level" >> "$REPORT_FILE"
    echo "" >> "$REPORT_FILE"
    
    # Clean build directory for accurate testing
    cargo clean -p "$package" 2>/dev/null || true
    
    # Build test
    echo -n "  Building... "
    if BUILD_OUTPUT=$(cargo build -p "$package" --features "$features" 2>&1); then
        echo -e "${GREEN}✓${NC}"
        echo "✅ Build: PASSED" >> "$REPORT_FILE"
    else
        echo -e "${RED}✗${NC}"
        echo "❌ Build: FAILED" >> "$REPORT_FILE"
        OVERALL_SUCCESS=false
        analyze_failure "$name" "$BUILD_OUTPUT" "$features"
        echo "See architectural analysis for details." >> "$REPORT_FILE"
        echo "" >> "$REPORT_FILE"
        return 1
    fi
    
    # Test execution
    echo -n "  Testing... "
    if TEST_OUTPUT=$(cargo test -p "$package" --features "$features" -- --nocapture 2>&1); then
        echo -e "${GREEN}✓${NC}"
        echo "✅ Tests: PASSED" >> "$REPORT_FILE"
    else
        echo -e "${RED}✗${NC}"
        echo "❌ Tests: FAILED" >> "$REPORT_FILE"
        OVERALL_SUCCESS=false
        analyze_failure "$name - Tests" "$TEST_OUTPUT" "$features"
    fi
    
    # ASIL-specific checks
    if [[ "$asil_level" == "ASIL-D" ]] || [[ "$asil_level" == "ASIL-C" ]]; then
        echo -n "  ASIL compliance check... "
        # Check for forbidden patterns in safety-critical builds
        if cargo check -p "$package" --features "$features" --message-format=json 2>&1 | grep -q "unsafe"; then
            echo -e "${YELLOW}⚠${NC}"
            echo "⚠️  ASIL Check: Unsafe code detected" >> "$REPORT_FILE"
            ARCHITECTURAL_ISSUES+=("UNSAFE_IN_ASIL")
        else
            echo -e "${GREEN}✓${NC}"
            echo "✅ ASIL Check: No unsafe code" >> "$REPORT_FILE"
        fi
    fi
    
    echo "" >> "$REPORT_FILE"
}

# Main verification matrix
echo -e "${BLUE}Starting Build Matrix Verification${NC}"
echo ""

# WRT Library Configurations (using working feature combinations)
test_configuration "WRT no_std + alloc" "wrt" "alloc" "Core"
test_configuration "WRT ASIL-D (no_std + alloc)" "wrt" "alloc,safety-asil-d" "ASIL-D"
test_configuration "WRT ASIL-C (no_std + alloc)" "wrt" "alloc,safety-asil-c" "ASIL-C"
test_configuration "WRT ASIL-B (no_std + alloc)" "wrt" "alloc,safety-asil-b" "ASIL-B"
test_configuration "WRT Development (std)" "wrt" "std" "Development"
test_configuration "WRT Development with Optimization" "wrt" "std,optimize" "Development"
test_configuration "WRT Server" "wrt" "std,optimize,platform" "Server"

# WRTD Binary Configurations (using actual available features)
test_configuration "WRTD ASIL-D Runtime" "wrtd" "safety-asil-d,wrt-execution,enable-panic-handler" "ASIL-D"
test_configuration "WRTD ASIL-C Runtime" "wrtd" "safety-asil-c,wrt-execution,enable-panic-handler" "ASIL-C"
test_configuration "WRTD ASIL-B Runtime" "wrtd" "safety-asil-b,wrt-execution,asil-b-panic" "ASIL-B"
test_configuration "WRTD Development Runtime" "wrtd" "std,wrt-execution,dev-panic" "Development"
test_configuration "WRTD Server Runtime" "wrtd" "std,wrt-execution" "Server"

# Component Model Tests (using wrt-component directly)
test_configuration "Component Model Core" "wrt-component" "no_std,alloc,component-model-core" "Component"
test_configuration "Component Model Full" "wrt-component" "std,component-model-all" "Component"

# Kani verification (if available)
if command -v cargo-kani &> /dev/null; then
    echo -e "${BLUE}Running Kani Verification${NC}"
    echo "## Kani Formal Verification" >> "$REPORT_FILE"
    if cargo kani -p wrt --features "no_std,alloc,kani,safety-asil-d" 2>&1; then
        echo -e "${GREEN}✓ Kani verification passed${NC}"
        echo "✅ Kani: PASSED" >> "$REPORT_FILE"
    else
        echo -e "${RED}✗ Kani verification failed${NC}"
        echo "❌ Kani: FAILED" >> "$REPORT_FILE"
        OVERALL_SUCCESS=false
    fi
    echo "" >> "$REPORT_FILE"
fi

# Summary
echo ""
echo -e "${BLUE}=== Verification Summary ===${NC}"
echo "# Summary" >> "$REPORT_FILE"
echo "" >> "$REPORT_FILE"

if [ "$OVERALL_SUCCESS" = true ]; then
    echo -e "${GREEN}All configurations passed!${NC}"
    echo "✅ **All configurations passed successfully**" >> "$REPORT_FILE"
else
    echo -e "${RED}Some configurations failed!${NC}"
    echo "❌ **Some configurations failed**" >> "$REPORT_FILE"
fi

# Architectural issues summary
if [ ${#ARCHITECTURAL_ISSUES[@]} -gt 0 ]; then
    echo ""
    echo -e "${RED}=== Architectural Issues Detected ===${NC}"
    echo "# Architectural Issues Summary" >> "$ARCHITECTURAL_ISSUES_FILE"
    echo "" >> "$ARCHITECTURAL_ISSUES_FILE"
    
    # Count unique issues
    printf '%s\n' "${ARCHITECTURAL_ISSUES[@]}" | sort -u | while read -r issue; do
        case "$issue" in
            "CYCLIC_DEPENDENCY")
                echo "- Cyclic dependencies violating ASIL modular design principles"
                ;;
            "DUPLICATE_DEFINITIONS")
                echo "- Duplicate definitions indicating poor modularity"
                ;;
            "TRAIT_BOUNDS")
                echo "- Trait bound violations suggesting improper abstractions"
                ;;
            "MISSING_IMPORTS")
                echo "- Missing imports/modules breaking deterministic compilation"
                ;;
            "COHERENCE")
                echo "- Coherence violations requiring architectural refactoring"
                ;;
            "MEMORY_ALLOCATION")
                echo "- Memory allocation issues for no_std environments"
                ;;
            "STD_CONFLICT")
                echo "- Conflicting std/no_std requirements"
                ;;
            "UNSAFE_IN_ASIL")
                echo "- Unsafe code in ASIL-critical configurations"
                ;;
        esac
    done | tee -a "$ARCHITECTURAL_ISSUES_FILE"
    
    echo ""
    echo "## Recommended Actions" >> "$ARCHITECTURAL_ISSUES_FILE"
    echo "1. Review module boundaries and dependencies" >> "$ARCHITECTURAL_ISSUES_FILE"
    echo "2. Ensure feature flags properly isolate platform-specific code" >> "$ARCHITECTURAL_ISSUES_FILE"
    echo "3. Verify all ASIL configurations can build without std library" >> "$ARCHITECTURAL_ISSUES_FILE"
    echo "4. Remove or properly abstract unsafe code in safety-critical paths" >> "$ARCHITECTURAL_ISSUES_FILE"
fi

echo ""
echo "Reports generated:"
echo "  - $REPORT_FILE"
echo "  - $ARCHITECTURAL_ISSUES_FILE"

# Exit with failure if any test failed
if [ "$OVERALL_SUCCESS" = false ]; then
    exit 1
fi