#!/bin/bash
# Script to migrate managed_alloc! to safe_managed_alloc!

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

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

# Configuration
WORKSPACE_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BACKUP_DIR="${WORKSPACE_ROOT}/.managed-alloc-backup"
DRY_RUN=false

# Function to show usage
show_usage() {
    echo "Usage: $0 [OPTIONS]"
    echo
    echo "Options:"
    echo "  --dry-run    Show what would be done without making changes"
    echo "  --help       Show this help message"
    echo
    echo "This script migrates all managed_alloc! usage to safe_managed_alloc!."
}

# Function to parse command line arguments
parse_args() {
    while [[ $# -gt 0 ]]; do
        case $1 in
            --dry-run)
                DRY_RUN=true
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

# Function to find files needing migration
find_files_to_migrate() {
    print_header "Finding Files to Migrate"
    
    local files_to_migrate=()
    
    # Find all Rust files with managed_alloc! usage
    while IFS= read -r -d '' file; do
        if grep -q "managed_alloc!" "$file" 2>/dev/null; then
            # Skip if it's already the new macro
            if ! grep -q "safe_managed_alloc!" "$file" || grep -q "managed_alloc![^_]" "$file"; then
                files_to_migrate+=("$file")
                print_status "$YELLOW" "Found: $file"
            fi
        fi
    done < <(find "$WORKSPACE_ROOT" -name "*.rs" -type f -print0 2>/dev/null | grep -z -v target)
    
    if [[ ${#files_to_migrate[@]} -eq 0 ]]; then
        print_success "No files need migration - all already use safe_managed_alloc!"
        return 0
    fi
    
    # Store files for processing
    printf '%s\n' "${files_to_migrate[@]}" > "${WORKSPACE_ROOT}/.files-to-migrate"
    
    print_status "$BLUE" "Found ${#files_to_migrate[@]} files to migrate"
    return 0
}

# Function to create backup
create_backup() {
    print_header "Creating Backup"
    
    if [[ ! -f "${WORKSPACE_ROOT}/.files-to-migrate" ]]; then
        print_warning "No files to backup"
        return 0
    fi
    
    # Create backup directory
    mkdir -p "$BACKUP_DIR"
    
    # Copy files to backup
    while IFS= read -r file; do
        if [[ -f "$file" ]]; then
            local rel_path="${file#$WORKSPACE_ROOT/}"
            local backup_path="${BACKUP_DIR}/${rel_path}"
            local backup_dir="$(dirname "$backup_path")"
            
            mkdir -p "$backup_dir"
            
            if [[ "$DRY_RUN" == "true" ]]; then
                print_status "$BLUE" "Would backup: $rel_path"
            else
                cp "$file" "$backup_path"
                print_success "Backed up: $rel_path"
            fi
        fi
    done < "${WORKSPACE_ROOT}/.files-to-migrate"
    
    if [[ "$DRY_RUN" != "true" ]]; then
        print_success "Backup created at: $BACKUP_DIR"
    fi
    
    return 0
}

# Function to migrate files
migrate_files() {
    print_header "Migrating Files"
    
    if [[ ! -f "${WORKSPACE_ROOT}/.files-to-migrate" ]]; then
        print_warning "No files to migrate"
        return 0
    fi
    
    local migrated_count=0
    local failed_count=0
    
    while IFS= read -r file; do
        if [[ -f "$file" ]]; then
            local rel_path="${file#$WORKSPACE_ROOT/}"
            
            if [[ "$DRY_RUN" == "true" ]]; then
                local count=$(grep -c "managed_alloc!" "$file" 2>/dev/null || echo "0")
                print_status "$BLUE" "Would migrate $count occurrences in: $rel_path"
            else
                # Count occurrences before migration
                local before_count=$(grep -c "managed_alloc!" "$file" 2>/dev/null || echo "0")
                
                if [[ $before_count -gt 0 ]]; then
                    # Perform the migration using sed
                    if sed -i '' 's/managed_alloc!/safe_managed_alloc!/g' "$file"; then
                        local after_count=$(grep -c "managed_alloc!" "$file" 2>/dev/null || echo "0")
                        print_success "Migrated $before_count occurrences in: $rel_path"
                        ((migrated_count++))
                        
                        # Verify the migration
                        if [[ $after_count -gt 0 ]]; then
                            print_warning "  Note: $after_count occurrences still remain (may be in comments)"
                        fi
                    else
                        print_error "Failed to migrate: $rel_path"
                        ((failed_count++))
                    fi
                else
                    print_success "No migration needed for: $rel_path"
                fi
            fi
        fi
    done < "${WORKSPACE_ROOT}/.files-to-migrate"
    
    if [[ "$DRY_RUN" != "true" ]]; then
        print_success "Successfully migrated $migrated_count files"
        if [[ $failed_count -gt 0 ]]; then
            print_error "$failed_count files failed migration"
            return 1
        fi
    fi
    
    return 0
}

# Function to update imports
update_imports() {
    print_header "Updating Imports"
    
    if [[ ! -f "${WORKSPACE_ROOT}/.files-to-migrate" ]]; then
        print_warning "No files to update imports for"
        return 0
    fi
    
    while IFS= read -r file; do
        if [[ -f "$file" ]]; then
            local rel_path="${file#$WORKSPACE_ROOT/}"
            
            # Check if file needs import updates
            if grep -q "use.*managed_alloc" "$file" 2>/dev/null; then
                if [[ "$DRY_RUN" == "true" ]]; then
                    print_status "$BLUE" "Would update imports in: $rel_path"
                else
                    # Update imports
                    sed -i '' 's/use.*managed_alloc/use wrt_foundation::safe_managed_alloc/g' "$file"
                    print_success "Updated imports in: $rel_path"
                fi
            fi
        fi
    done < "${WORKSPACE_ROOT}/.files-to-migrate"
    
    return 0
}

# Function to verify migration
verify_migration() {
    print_header "Verifying Migration"
    
    if [[ "$DRY_RUN" == "true" ]]; then
        print_status "$BLUE" "Dry run - skipping verification"
        return 0
    fi
    
    # Count remaining managed_alloc! usages
    local remaining_files=$(find "$WORKSPACE_ROOT" -name "*.rs" -type f -exec grep -l "managed_alloc!" {} \; 2>/dev/null | grep -v target | wc -l)
    
    if [[ $remaining_files -eq 0 ]]; then
        print_success "All managed_alloc! usages successfully migrated"
    else
        print_warning "$remaining_files files still contain managed_alloc! (may be in comments or macros.rs)"
        
        # Show remaining files
        find "$WORKSPACE_ROOT" -name "*.rs" -type f -exec grep -l "managed_alloc!" {} \; 2>/dev/null | grep -v target | while read -r file; do
            local rel_path="${file#$WORKSPACE_ROOT/}"
            local count=$(grep -c "managed_alloc!" "$file" 2>/dev/null || echo "0")
            print_status "$YELLOW" "  $rel_path: $count occurrences"
        done
    fi
    
    return 0
}

# Function to test compilation
test_compilation() {
    print_header "Testing Compilation"
    
    if [[ "$DRY_RUN" == "true" ]]; then
        print_status "$BLUE" "Dry run - skipping compilation test"
        return 0
    fi
    
    cd "$WORKSPACE_ROOT"
    
    # Test basic compilation
    if cargo check --workspace >/dev/null 2>&1; then
        print_success "Workspace compiles successfully after migration"
    else
        print_error "Compilation issues detected after migration"
        print_status "$YELLOW" "You may need to run: cargo build --workspace"
        return 1
    fi
    
    return 0
}

# Main execution function
main() {
    print_header "Managed Alloc Migration"
    
    if [[ "$DRY_RUN" == "true" ]]; then
        print_status "$BLUE" "DRY RUN MODE - No changes will be made"
    fi
    
    local exit_code=0
    
    # Step 1: Find files to migrate
    if ! find_files_to_migrate; then
        print_error "Failed to find files for migration"
        exit 1
    fi
    
    # Step 2: Create backup
    if ! create_backup; then
        print_error "Failed to create backup"
        exit 1
    fi
    
    # Step 3: Migrate files
    if ! migrate_files; then
        print_error "Failed to migrate files"
        exit_code=1
    fi
    
    # Step 4: Update imports
    if ! update_imports; then
        print_warning "Issues updating imports"
        exit_code=2
    fi
    
    # Step 5: Verify migration
    if ! verify_migration; then
        print_warning "Migration verification issues"
        exit_code=2
    fi
    
    # Step 6: Test compilation
    if ! test_compilation; then
        print_warning "Compilation test issues"
        exit_code=2
    fi
    
    # Final status
    if [[ $exit_code -eq 0 ]]; then
        if [[ "$DRY_RUN" == "true" ]]; then
            print_success "✅ Dry run completed - ready for actual migration"
            print_status "$BLUE" "Run without --dry-run to perform the migration"
        else
            print_success "✅ managed_alloc! migration completed successfully!"
            print_status "$GREEN" "All files migrated to safe_managed_alloc!"
            print_status "$BLUE" "Backup available at: $BACKUP_DIR"
        fi
    else
        print_error "❌ managed_alloc! migration encountered issues"
        if [[ "$DRY_RUN" != "true" ]]; then
            print_status "$YELLOW" "Backup available for recovery at: $BACKUP_DIR"
        fi
    fi
    
    # Cleanup temporary files
    rm -f "${WORKSPACE_ROOT}/.files-to-migrate"
    
    exit $exit_code
}

# Parse arguments and execute
parse_args "$@"
main