#!/bin/bash
# KANI Verification Script for WRT
# This script runs formal verification using KANI and generates reports

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
WORKSPACE_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
REPORT_DIR="${WORKSPACE_ROOT}/target/kani-reports"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
REPORT_FILE="${REPORT_DIR}/verification_report_${TIMESTAMP}.md"

# Parse command line arguments
PROFILE="asil-c"
PACKAGE=""
HARNESS=""
VERBOSE=false

while [[ $# -gt 0 ]]; do
    case $1 in
        --profile)
            PROFILE="$2"
            shift 2
            ;;
        --package)
            PACKAGE="$2"
            shift 2
            ;;
        --harness)
            HARNESS="$2"
            shift 2
            ;;
        --verbose)
            VERBOSE=true
            shift
            ;;
        --help)
            echo "Usage: $0 [options]"
            echo "Options:"
            echo "  --profile <profile>  ASIL profile to use (asil-a, asil-b, asil-c, asil-d)"
            echo "  --package <package>  Specific package to verify"
            echo "  --harness <harness>  Specific harness to run"
            echo "  --verbose           Enable verbose output"
            echo "  --help              Show this help message"
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            exit 1
            ;;
    esac
done

# Create report directory
mkdir -p "${REPORT_DIR}"

# Print header
echo -e "${BLUE}=== WRT KANI Formal Verification ===${NC}"
echo -e "Profile: ${PROFILE}"
echo -e "Timestamp: ${TIMESTAMP}"
echo

# Initialize report
cat > "${REPORT_FILE}" << EOF
# WRT KANI Formal Verification Report

**Date**: $(date)  
**Profile**: ${PROFILE}  
**System**: $(uname -a)  

## Summary

EOF

# Function to run KANI on a package
run_kani_package() {
    local pkg=$1
    echo -e "${YELLOW}Verifying package: ${pkg}${NC}"
    
    # Check if package has KANI configuration
    if ! grep -q "name = \"${pkg}\"" "${WORKSPACE_ROOT}/Cargo.toml"; then
        echo -e "${RED}Package ${pkg} not configured for KANI${NC}"
        return 1
    fi
    
    # Run KANI
    local kani_args="--tests"
    if [ -n "${HARNESS}" ]; then
        kani_args="${kani_args} --harness ${HARNESS}"
    fi
    
    if [ "${VERBOSE}" = true ]; then
        kani_args="${kani_args} --verbose"
    fi
    
    # Add profile-specific arguments
    case ${PROFILE} in
        asil-d)
            kani_args="${kani_args} --enable-unstable --solver cadical"
            ;;
        asil-c)
            kani_args="${kani_args} --solver cadical"
            ;;
        asil-b)
            kani_args="${kani_args} --solver cadical"
            ;;
        *)
            kani_args="${kani_args} --solver minisat"
            ;;
    esac
    
    echo "Running: cargo kani -p ${pkg} ${kani_args}"
    
    # Capture output
    local output_file="${REPORT_DIR}/${pkg}_${TIMESTAMP}.log"
    if cargo kani -p "${pkg}" ${kani_args} 2>&1 | tee "${output_file}"; then
        echo -e "${GREEN}✓ ${pkg} verification passed${NC}"
        echo "### ${pkg}" >> "${REPORT_FILE}"
        echo "**Status**: ✅ PASSED" >> "${REPORT_FILE}"
        echo "" >> "${REPORT_FILE}"
        
        # Extract statistics
        local total_checks=$(grep -c "VERIFICATION:.*CHECK" "${output_file}" || echo "0")
        local passed_checks=$(grep -c "VERIFICATION:.*SUCCESS" "${output_file}" || echo "0")
        echo "**Checks**: ${passed_checks}/${total_checks} passed" >> "${REPORT_FILE}"
        echo "" >> "${REPORT_FILE}"
    else
        echo -e "${RED}✗ ${pkg} verification failed${NC}"
        echo "### ${pkg}" >> "${REPORT_FILE}"
        echo "**Status**: ❌ FAILED" >> "${REPORT_FILE}"
        echo "" >> "${REPORT_FILE}"
        
        # Extract failure information
        echo "**Failures**:" >> "${REPORT_FILE}"
        grep "VERIFICATION:.*FAILURE" "${output_file}" | sed 's/^/- /' >> "${REPORT_FILE}" || echo "- See log for details" >> "${REPORT_FILE}"
        echo "" >> "${REPORT_FILE}"
    fi
}

# Function to run all configured packages
run_all_packages() {
    # Extract packages with KANI configuration
    local packages=$(grep -A1 '\[\[workspace.metadata.kani.package\]\]' "${WORKSPACE_ROOT}/Cargo.toml" | \
                     grep 'name = ' | \
                     sed 's/.*name = "\(.*\)"/\1/' | \
                     sort -u)
    
    local total=0
    local passed=0
    
    for pkg in ${packages}; do
        ((total++))
        if run_kani_package "${pkg}"; then
            ((passed++))
        fi
        echo
    done
    
    # Update summary
    echo "## Overall Results" >> "${REPORT_FILE}"
    echo "" >> "${REPORT_FILE}"
    echo "**Total Packages**: ${total}" >> "${REPORT_FILE}"
    echo "**Passed**: ${passed}" >> "${REPORT_FILE}"
    echo "**Failed**: $((total - passed))" >> "${REPORT_FILE}"
    echo "**Success Rate**: $(( passed * 100 / total ))%" >> "${REPORT_FILE}"
}

# Main execution
if [ -n "${PACKAGE}" ]; then
    # Run specific package
    run_kani_package "${PACKAGE}"
else
    # Run all packages
    run_all_packages
fi

# Generate coverage report for ASIL-D
if [ "${PROFILE}" = "asil-d" ] && command -v kani-cov &> /dev/null; then
    echo -e "${BLUE}Generating coverage report...${NC}"
    kani-cov "${REPORT_DIR}"/*.log > "${REPORT_DIR}/coverage_${TIMESTAMP}.txt" || true
fi

# Print report location
echo
echo -e "${GREEN}Verification complete!${NC}"
echo -e "Report saved to: ${REPORT_FILE}"
echo
echo "To view the report:"
echo "  cat ${REPORT_FILE}"

# Exit with appropriate code
if grep -q "❌ FAILED" "${REPORT_FILE}"; then
    exit 1
else
    exit 0
fi