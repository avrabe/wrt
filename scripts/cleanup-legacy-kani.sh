#!/bin/bash
# Legacy KANI Cleanup Script
# Safely removes legacy KANI test files after successful migration verification

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
WORKSPACE_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
MIGRATION_REPORT="${WORKSPACE_ROOT}/scripts/.kani-migration-report.md"
BACKUP_DIR="${WORKSPACE_ROOT}/.legacy-kani-backup"
DRY_RUN=false

# Function to print colored output
print_status() {
    local color=$1
    local message=$2
    echo -e "${color}${message}${NC}"
}

print_header() {
    echo
    print_status "$BLUE" "=== $1 ==="
}

print_success() {
    print_status "$GREEN" "✅ $1"
}

print_warning() {
    print_status "$YELLOW" "⚠️  $1"
}

print_error() {
    print_status "$RED" "❌ $1"
}

# Function to show usage
show_usage() {
    echo "Usage: $0 [OPTIONS]"
    echo
    echo "Options:"
    echo "  --dry-run    Show what would be done without making changes"
    echo "  --force      Skip safety checks and proceed with cleanup"
    echo "  --help       Show this help message"
    echo
    echo "This script safely removes legacy KANI test files after migration verification."
    echo "It requires a successful migration verification report to proceed."
}

# Function to parse command line arguments
parse_args() {
    while [[ $# -gt 0 ]]; do
        case $1 in
            --dry-run)
                DRY_RUN=true
                shift
                ;;
            --force)
                FORCE_CLEANUP=true
                shift
                ;;
            --help)
                show_usage
                exit 0
                ;;
            *)
                print_error "Unknown option: $1"
                show_usage
                exit 1
                ;;
        esac
    done
}

# Function to verify migration is complete
verify_migration_complete() {
    print_header "Verifying Migration Completion"
    
    # Check if migration verification has been run
    if [[ ! -f "$MIGRATION_REPORT" ]]; then
        print_error "Migration report not found at $MIGRATION_REPORT"
        print_status "$YELLOW" "Please run ./scripts/verify-kani-migration.sh first"
        return 1
    fi
    
    # Check if migration was successful
    if ! grep -q "completed successfully" "$MIGRATION_REPORT" 2>/dev/null; then
        print_error "Migration verification did not complete successfully"
        print_status "$YELLOW" "Please ensure all legacy tests have modern equivalents before cleanup"
        return 1
    fi
    
    # Check if modern infrastructure is working
    if ! ./scripts/check-kani-status.sh >/dev/null 2>&1; then
        print_error "Modern KANI infrastructure is not ready"
        print_status "$YELLOW" "Please ensure modern formal verification is working before cleanup"
        return 1
    fi
    
    print_success "Migration verification passed - safe to proceed with cleanup"
    return 0
}

# Function to identify legacy files
identify_legacy_files() {
    print_header "Identifying Legacy KANI Files"
    
    local legacy_files=()
    
    # Find files with legacy KANI proof annotations
    while IFS= read -r -d '' file; do
        if grep -q "#\[kani::proof\]" "$file" 2>/dev/null; then
            # Skip files in the modern infrastructure
            if [[ "$file" != *"wrt-tests/integration/formal_verification"* ]]; then
                legacy_files+=("$file")
                print_status "$YELLOW" "Legacy file: $file"
            fi
        fi
    done < <(find "$WORKSPACE_ROOT" -name "*.rs" -type f -print0 2>/dev/null)
    
    # Look for dedicated legacy test files
    local known_legacy_files=(
        "wrt-sync/tests/kani_proofs.rs"
    )
    
    for file in "${known_legacy_files[@]}"; do
        local full_path="${WORKSPACE_ROOT}/${file}"
        if [[ -f "$full_path" ]]; then
            legacy_files+=("$full_path")
            print_status "$YELLOW" "Known legacy file: $full_path"
        fi
    done
    
    if [[ ${#legacy_files[@]} -eq 0 ]]; then
        print_success "No legacy KANI files found - cleanup already complete"
        return 0
    fi
    
    print_status "$BLUE" "Found ${#legacy_files[@]} legacy files to process"
    
    # Store legacy files for processing
    printf '%s\n' "${legacy_files[@]}" > "${WORKSPACE_ROOT}/.legacy-files-list"
    
    return 0
}

# Function to create backup
create_backup() {
    print_header "Creating Backup"
    
    if [[ ! -f "${WORKSPACE_ROOT}/.legacy-files-list" ]]; then
        print_warning "No legacy files to backup"
        return 0
    fi
    
    # Create backup directory
    mkdir -p "$BACKUP_DIR"
    
    # Copy legacy files to backup
    while IFS= read -r file; do
        if [[ -f "$file" ]]; then
            local rel_path="${file#$WORKSPACE_ROOT/}"
            local backup_path="${BACKUP_DIR}/${rel_path}"
            local backup_dir="$(dirname "$backup_path")"
            
            mkdir -p "$backup_dir"
            
            if [[ "$DRY_RUN" == "true" ]]; then
                print_status "$BLUE" "Would backup: $rel_path → $backup_path"
            else
                cp "$file" "$backup_path"
                print_success "Backed up: $rel_path"
            fi
        fi
    done < "${WORKSPACE_ROOT}/.legacy-files-list"
    
    if [[ "$DRY_RUN" != "true" ]]; then
        print_success "Backup created at: $BACKUP_DIR"
    fi
    
    return 0
}

# Function to remove legacy files
remove_legacy_files() {
    print_header "Removing Legacy Files"
    
    if [[ ! -f "${WORKSPACE_ROOT}/.legacy-files-list" ]]; then
        print_warning "No legacy files to remove"
        return 0
    fi
    
    local removed_count=0
    
    while IFS= read -r file; do
        if [[ -f "$file" ]]; then
            local rel_path="${file#$WORKSPACE_ROOT/}"
            
            if [[ "$DRY_RUN" == "true" ]]; then
                print_status "$BLUE" "Would remove: $rel_path"
            else
                rm "$file"
                print_success "Removed: $rel_path"
                ((removed_count++))
            fi
        fi
    done < "${WORKSPACE_ROOT}/.legacy-files-list"
    
    if [[ "$DRY_RUN" != "true" ]]; then
        print_success "Removed $removed_count legacy files"
    fi
    
    return 0
}

# Function to clean up legacy dependencies
clean_legacy_dependencies() {
    print_header "Cleaning Legacy Dependencies"
    
    # Look for legacy kani dependencies in individual crates
    while IFS= read -r -d '' cargo_file; do
        if grep -q "kani-verifier" "$cargo_file" 2>/dev/null; then
            # Skip the integration test crate which should keep the dependency
            if [[ "$cargo_file" != *"wrt-tests/integration/Cargo.toml" ]]; then
                local rel_path="${cargo_file#$WORKSPACE_ROOT/}"
                
                if [[ "$DRY_RUN" == "true" ]]; then
                    print_status "$BLUE" "Would check for legacy dependencies in: $rel_path"
                else
                    print_status "$YELLOW" "Manual review needed for dependencies in: $rel_path"
                fi
            fi
        fi
    done < <(find "$WORKSPACE_ROOT" -name "Cargo.toml" -type f -print0 2>/dev/null)
    
    return 0
}

# Function to update documentation
update_documentation() {
    print_header "Updating Documentation"
    
    # Check for references to legacy test files in documentation
    local doc_files=()
    while IFS= read -r -d '' file; do
        if grep -q "kani_proofs.rs\|legacy.*kani" "$file" 2>/dev/null; then
            doc_files+=("$file")
        fi
    done < <(find "${WORKSPACE_ROOT}/docs" -name "*.rst" -o -name "*.md" -type f -print0 2>/dev/null)
    
    if [[ ${#doc_files[@]} -gt 0 ]]; then
        print_status "$YELLOW" "Documentation files may need updates:"
        for file in "${doc_files[@]}"; do
            local rel_path="${file#$WORKSPACE_ROOT/}"
            print_status "$YELLOW" "  - $rel_path"
        done
    else
        print_success "No documentation updates needed"
    fi
    
    return 0
}

# Function to run post-cleanup verification
post_cleanup_verification() {
    print_header "Post-Cleanup Verification"
    
    if [[ "$DRY_RUN" == "true" ]]; then
        print_status "$BLUE" "Dry run - skipping verification"
        return 0
    fi
    
    # Verify modern infrastructure still works
    if ./scripts/check-kani-status.sh >/dev/null 2>&1; then
        print_success "Modern KANI infrastructure is still functional"
    else
        print_error "Modern KANI infrastructure has issues after cleanup"
        return 1
    fi
    
    # Try a quick compilation test
    cd "$WORKSPACE_ROOT"
    if cargo check --workspace >/dev/null 2>&1; then
        print_success "Workspace compiles successfully after cleanup"
    else
        print_warning "Workspace compilation issues detected - may need manual review"
    fi
    
    return 0
}

# Main execution function
main() {
    print_header "Legacy KANI Cleanup"
    
    if [[ "$DRY_RUN" == "true" ]]; then
        print_status "$BLUE" "DRY RUN MODE - No changes will be made"
    fi
    
    local exit_code=0
    
    # Step 1: Verify migration is complete
    if [[ "${FORCE_CLEANUP:-false}" != "true" ]]; then
        if ! verify_migration_complete; then
            print_error "Migration verification failed - aborting cleanup"
            exit 1
        fi
    fi
    
    # Step 2: Identify legacy files
    if ! identify_legacy_files; then
        print_error "Failed to identify legacy files"
        exit 1
    fi
    
    # Step 3: Create backup
    if ! create_backup; then
        print_error "Failed to create backup"
        exit 1
    fi
    
    # Step 4: Remove legacy files
    if ! remove_legacy_files; then
        print_error "Failed to remove legacy files"
        exit_code=1
    fi
    
    # Step 5: Clean legacy dependencies
    clean_legacy_dependencies
    
    # Step 6: Update documentation
    update_documentation
    
    # Step 7: Post-cleanup verification
    if ! post_cleanup_verification; then
        print_warning "Post-cleanup verification issues detected"
        exit_code=2
    fi
    
    # Final status
    if [[ $exit_code -eq 0 ]]; then
        if [[ "$DRY_RUN" == "true" ]]; then
            print_success "✅ Dry run completed - ready for actual cleanup"
            print_status "$BLUE" "Run without --dry-run to perform the cleanup"
        else
            print_success "✅ Legacy KANI cleanup completed successfully!"
            print_status "$GREEN" "Backup available at: $BACKUP_DIR"
        fi
    else
        print_error "❌ Legacy KANI cleanup encountered issues"
        if [[ "$DRY_RUN" != "true" ]]; then
            print_status "$YELLOW" "Backup available for recovery at: $BACKUP_DIR"
        fi
    fi
    
    # Cleanup temporary files
    rm -f "${WORKSPACE_ROOT}/.legacy-files-list"
    
    exit $exit_code
}

# Parse arguments and execute
parse_args "$@"
main