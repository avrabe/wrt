#!/bin/bash
# Fix double "safe_" prefixes created by migration script

set -euo pipefail

WORKSPACE_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

echo "🔧 Fixing double safe_ prefixes..."

# Fix safe_safe_managed_alloc! -> safe_managed_alloc!
find "$WORKSPACE_ROOT" -name "*.rs" -type f -not -path "*/target/*" -exec sed -i '' 's/safe_safe_managed_alloc!/safe_managed_alloc!/g' {} \;

# Fix import issues
find "$WORKSPACE_ROOT" -name "*.rs" -type f -not -path "*/target/*" -exec sed -i '' 's/use wrt_foundation::safe_managed_alloc, CrateId}/use wrt_foundation::{safe_managed_alloc, CrateId}/g' {} \;

# Count remaining issues
echo "✅ Fixed double prefixes"

remaining=$(find "$WORKSPACE_ROOT" -name "*.rs" -type f -not -path "*/target/*" -exec grep -l "safe_safe_managed_alloc!" {} \; 2>/dev/null | wc -l)
echo "📊 Remaining files with safe_safe_managed_alloc!: $remaining"

# Test compilation
cd "$WORKSPACE_ROOT"
if cargo check --workspace >/dev/null 2>&1; then
    echo "✅ Workspace compiles successfully"
else
    echo "⚠️  Some compilation issues remain"
fi