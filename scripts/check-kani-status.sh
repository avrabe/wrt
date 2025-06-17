#!/bin/bash
# Check KANI verification status across the workspace
# This script provides a quick overview of verification readiness

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

WORKSPACE_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

echo -e "${BLUE}=== WRT KANI Verification Status ===${NC}"
echo

# Check if KANI is installed
echo -e "${YELLOW}Checking KANI installation...${NC}"
if command -v kani &> /dev/null; then
    KANI_VERSION=$(kani --version | head -1)
    echo -e "${GREEN}✓ KANI installed: ${KANI_VERSION}${NC}"
else
    echo -e "${RED}✗ KANI not installed${NC}"
    echo "Install with: cargo install --locked kani-verifier && cargo kani setup"
    exit 1
fi

# Check Rust toolchain
echo -e "${YELLOW}Checking Rust toolchain...${NC}"
if rustup toolchain list | grep -q "nightly-2024-01-01"; then
    echo -e "${GREEN}✓ Required nightly toolchain available${NC}"
else
    echo -e "${RED}✗ Required nightly toolchain missing${NC}"
    echo "Install with: rustup toolchain install nightly-2024-01-01"
    echo "              rustup component add rust-src --toolchain nightly-2024-01-01"
fi

# Check workspace configuration
echo -e "${YELLOW}Checking workspace configuration...${NC}"
if grep -q "workspace.metadata.kani" "${WORKSPACE_ROOT}/Cargo.toml"; then
    CONFIGURED_PACKAGES=$(grep -c "name.*wrt-" "${WORKSPACE_ROOT}/Cargo.toml" | head -1)
    echo -e "${GREEN}✓ Workspace KANI configuration found${NC}"
    echo "  Configured packages: ${CONFIGURED_PACKAGES}"
else
    echo -e "${RED}✗ Workspace KANI configuration missing${NC}"
fi

# Check integration test configuration
echo -e "${YELLOW}Checking integration test configuration...${NC}"
INTEGRATION_DIR="${WORKSPACE_ROOT}/wrt-tests/integration"
if [ -f "${INTEGRATION_DIR}/Kani.toml" ]; then
    echo -e "${GREEN}✓ Integration Kani.toml found${NC}"
else
    echo -e "${RED}✗ Integration Kani.toml missing${NC}"
fi

# Count harnesses
echo -e "${YELLOW}Counting verification harnesses...${NC}"
TOTAL_HARNESSES=0

# Count harnesses in workspace Cargo.toml
if [ -f "${WORKSPACE_ROOT}/Cargo.toml" ]; then
    WORKSPACE_HARNESSES=$(grep -c '"kani_verify_' "${WORKSPACE_ROOT}/Cargo.toml" 2>/dev/null || echo 0)
    echo "  Workspace harnesses: ${WORKSPACE_HARNESSES}"
    TOTAL_HARNESSES=$((TOTAL_HARNESSES + WORKSPACE_HARNESSES))
fi

# Count harnesses in formal verification modules
FORMAL_VERIFICATION_DIR="${INTEGRATION_DIR}/formal_verification"
if [ -d "${FORMAL_VERIFICATION_DIR}" ]; then
    MODULE_HARNESSES=$(find "${FORMAL_VERIFICATION_DIR}" -name "*.rs" -exec grep -c "#\[kani::proof\]" {} + 2>/dev/null | awk '{sum += $1} END {print sum}' || echo 0)
    echo "  Module harnesses: ${MODULE_HARNESSES}"
    TOTAL_HARNESSES=$((TOTAL_HARNESSES + MODULE_HARNESSES))
fi

echo "  Total harnesses: ${TOTAL_HARNESSES}"

# Check individual verification modules
echo -e "${YELLOW}Checking verification modules...${NC}"
MODULES=(
    "memory_safety_proofs.rs"
    "safety_invariants_proofs.rs"
    "concurrency_proofs.rs"
    "resource_lifecycle_proofs.rs"
    "integration_proofs.rs"
)

MODULES_FOUND=0
for module in "${MODULES[@]}"; do
    if [ -f "${FORMAL_VERIFICATION_DIR}/${module}" ]; then
        PROOFS=$(grep -c "#\[kani::proof\]" "${FORMAL_VERIFICATION_DIR}/${module}" 2>/dev/null || echo 0)
        PROPERTIES=$(grep -A1 "pub fn property_count()" "${FORMAL_VERIFICATION_DIR}/${module}" 2>/dev/null | tail -1 | grep -o '[0-9]\+' | head -1 || echo 0)
        echo "  ✓ ${module}: ${PROOFS} proofs, ${PROPERTIES} properties"
        ((MODULES_FOUND++))
    else
        echo "  ✗ ${module}: missing"
    fi
done

# Check CI configuration
echo -e "${YELLOW}Checking CI configuration...${NC}"
CI_FILE="${WORKSPACE_ROOT}/.github/workflows/kani-verification.yml"
if [ -f "${CI_FILE}" ]; then
    JOBS=$(grep -c "^[[:space:]]*[^#]*:" "${CI_FILE}" | head -1)
    echo -e "${GREEN}✓ CI workflow configured${NC}"
    echo "  Workflow jobs: ${JOBS}"
else
    echo -e "${RED}✗ CI workflow missing${NC}"
fi

# Check scripts
echo -e "${YELLOW}Checking verification scripts...${NC}"
VERIFY_SCRIPT="${WORKSPACE_ROOT}/scripts/kani-verify.sh"
if [ -f "${VERIFY_SCRIPT}" ] && [ -x "${VERIFY_SCRIPT}" ]; then
    echo -e "${GREEN}✓ kani-verify.sh script ready${NC}"
else
    echo -e "${RED}✗ kani-verify.sh script missing or not executable${NC}"
fi

# Summary
echo
echo -e "${BLUE}=== Summary ===${NC}"
echo "Modules found: ${MODULES_FOUND}/5"
echo "Total harnesses: ${TOTAL_HARNESSES}"
echo

if [ ${MODULES_FOUND} -eq 5 ] && [ ${TOTAL_HARNESSES} -gt 20 ]; then
    echo -e "${GREEN}✅ WRT is ready for formal verification!${NC}"
    echo
    echo "Quick start:"
    echo "  ./scripts/kani-verify.sh --profile asil-c"
    echo
    echo "CI workflow:"
    echo "  Push to main branch to trigger comprehensive verification"
else
    echo -e "${YELLOW}⚠️  Setup incomplete${NC}"
    echo "Missing components prevent full verification."
fi

echo
echo "For detailed instructions, see:"
echo "  wrt-tests/integration/formal_verification/README.md"