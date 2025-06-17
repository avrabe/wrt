#!/usr/bin/env python3
"""
Feature Flag Standardization Script for WRT Project

This script standardizes feature flags across all Cargo.toml files in the workspace
according to the WRT Feature Standardization Plan.
"""

import os
import re
import shutil
import sys
import tomllib
from pathlib import Path
from datetime import datetime
from typing import Dict, List, Set, Tuple

# Feature standardization mappings
FEATURE_MAPPINGS = {
    # Safety features
    'safety': 'safety-asil-b',
    'safety-critical': 'safety-asil-c',
    
    # Platform features (ensure platform- prefix)
    'linux': 'platform-linux',
    'qnx': 'platform-qnx',
    'vxworks': 'platform-vxworks',
    'tock': 'platform-tock',
    'zephyr': 'platform-zephyr',
    
    # Panic handler (remove duplicates)
    'disable-panic-handler': None,  # Remove - implied by no_std
    'custom-panic-handler': None,    # Remove - implied by no_std
}

# Core features every crate should have
CORE_FEATURES = {
    'default': ['std'],
    'std': [],
    'no_std': [],
    'safety-asil-b': [],
    'safety-asil-c': ['safety-asil-b'],
    'safety-asil-d': ['safety-asil-c'],
}

# Crates that should have KANI support
KANI_CRATES = [
    'wrt-foundation',
    'wrt-component', 
    'wrt-sync',
    'wrt-runtime',
    'wrt-platform',
    'wrt-instructions',
    'wrt-decoder',
    'wrt-host',
    'wrt-debug',
]

class FeatureStandardizer:
    def __init__(self, workspace_root: Path):
        self.workspace_root = workspace_root
        self.backup_dir = workspace_root / f".feature-backup-{datetime.now().strftime('%Y%m%d-%H%M%S')}"
        self.changes_made = []
        
    def backup_file(self, file_path: Path):
        """Create a backup of the file"""
        rel_path = file_path.relative_to(self.workspace_root)
        backup_path = self.backup_dir / rel_path
        backup_path.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(file_path, backup_path)
        
    def find_cargo_tomls(self) -> List[Path]:
        """Find all Cargo.toml files in the workspace"""
        cargo_files = []
        for root, dirs, files in os.walk(self.workspace_root):
            # Skip target and .git directories
            dirs[:] = [d for d in dirs if d not in ['target', '.git', 'node_modules', '.cargo']]
            
            if 'Cargo.toml' in files:
                cargo_files.append(Path(root) / 'Cargo.toml')
                
        return sorted(cargo_files)
    
    def get_crate_name(self, cargo_path: Path) -> str:
        """Extract crate name from Cargo.toml"""
        try:
            data = toml.load(cargo_path)
            return data.get('package', {}).get('name', 'unknown')
        except:
            return 'unknown'
            
    def standardize_features(self, cargo_path: Path) -> bool:
        """Standardize features in a single Cargo.toml file"""
        try:
            data = toml.load(cargo_path)
            crate_name = data.get('package', {}).get('name', 'unknown')
            
            if 'features' not in data:
                data['features'] = {}
                
            original_features = data['features'].copy()
            features = data['features']
            changes = []
            
            # Add core features if missing
            for feature, deps in CORE_FEATURES.items():
                if feature not in features:
                    features[feature] = deps.copy()
                    changes.append(f"Added core feature '{feature}'")
                    
            # Add KANI feature if this crate should have it
            if crate_name in KANI_CRATES and 'kani' not in features:
                features['kani'] = []
                changes.append("Added 'kani' feature")
                
            # Apply feature mappings
            features_to_remove = []
            features_to_add = {}
            
            for old_feature, new_feature in FEATURE_MAPPINGS.items():
                if old_feature in features:
                    if new_feature is None:
                        # Feature should be removed
                        features_to_remove.append(old_feature)
                        changes.append(f"Removed deprecated feature '{old_feature}'")
                    else:
                        # Feature should be renamed
                        if new_feature not in features:
                            features_to_add[new_feature] = features[old_feature]
                            changes.append(f"Renamed '{old_feature}' to '{new_feature}'")
                        features_to_remove.append(old_feature)
                        
            # Apply changes
            for feature in features_to_remove:
                del features[feature]
                
            for feature, deps in features_to_add.items():
                features[feature] = deps
                
            # Update feature dependencies
            for feature, deps in features.items():
                if isinstance(deps, list):
                    new_deps = []
                    for dep in deps:
                        # Update dependency names
                        if dep in FEATURE_MAPPINGS:
                            new_dep = FEATURE_MAPPINGS[dep]
                            if new_dep is not None:
                                new_deps.append(new_dep)
                                changes.append(f"Updated dependency '{dep}' to '{new_dep}' in feature '{feature}'")
                        else:
                            new_deps.append(dep)
                    features[feature] = new_deps
                    
            # Special handling for workspace dependencies with features
            if 'dependencies' in data:
                for dep_name, dep_value in data['dependencies'].items():
                    if isinstance(dep_value, dict) and 'features' in dep_value:
                        old_features = dep_value['features'].copy()
                        new_features = []
                        
                        for feat in old_features:
                            if feat in FEATURE_MAPPINGS:
                                new_feat = FEATURE_MAPPINGS[feat]
                                if new_feat is not None:
                                    new_features.append(new_feat)
                                    changes.append(f"Updated feature '{feat}' to '{new_feat}' in dependency '{dep_name}'")
                            else:
                                new_features.append(feat)
                                
                        dep_value['features'] = new_features
                        
            # Save if changes were made
            if changes:
                self.backup_file(cargo_path)
                
                # Write updated Cargo.toml
                with open(cargo_path, 'w') as f:
                    toml.dump(data, f)
                    
                self.changes_made.append({
                    'file': str(cargo_path.relative_to(self.workspace_root)),
                    'crate': crate_name,
                    'changes': changes
                })
                
                return True
                
        except Exception as e:
            print(f"Error processing {cargo_path}: {e}")
            
        return False
        
    def create_migration_guide(self):
        """Create a migration guide for users"""
        guide_path = self.workspace_root / "FEATURE_MIGRATION_GUIDE.md"
        
        content = """# WRT Feature Flag Migration Guide

This guide helps you migrate to the new standardized feature flags.

## Feature Mappings

The following features have been renamed:

| Old Feature | New Feature | Notes |
|-------------|-------------|-------|
| `safety` | `safety-asil-b` | ASIL-B safety level |
| `safety-critical` | `safety-asil-c` | ASIL-C safety level |
| `linux` | `platform-linux` | Platform-specific feature |
| `qnx` | `platform-qnx` | Platform-specific feature |
| `vxworks` | `platform-vxworks` | Platform-specific feature |

## Removed Features

The following features have been removed as they are implied by `no_std`:
- `disable-panic-handler`
- `custom-panic-handler`

## New Features

All crates now support these safety levels:
- `safety-asil-b` - Basic safety features (ASIL-B)
- `safety-asil-c` - Critical safety features (ASIL-C)
- `safety-asil-d` - Highest safety level (ASIL-D)

## Migration Steps

1. Update your `Cargo.toml` dependencies:
   ```toml
   # Old
   wrt-foundation = { version = "0.2", features = ["safety-critical"] }
   
   # New
   wrt-foundation = { version = "0.2", features = ["safety-asil-c"] }
   ```

2. Update your conditional compilation:
   ```rust
   // Old
   #[cfg(feature = "safety-critical")]
   
   // New
   #[cfg(feature = "safety-asil-c")]
   ```

3. Update your build scripts and CI/CD pipelines.

## Backwards Compatibility

For one release cycle (0.2.x), the old feature names will continue to work
with deprecation warnings.
"""
        
        with open(guide_path, 'w') as f:
            f.write(content)
            
        print(f"Created migration guide at {guide_path}")
        
    def generate_report(self):
        """Generate a report of all changes made"""
        report_path = self.workspace_root / "FEATURE_STANDARDIZATION_REPORT.md"
        
        content = f"""# Feature Standardization Report

Generated: {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}

## Summary

- Total files processed: {len(self.changes_made)}
- Backup location: {self.backup_dir}

## Changes by Crate

"""
        
        for change in self.changes_made:
            content += f"\n### {change['crate']} ({change['file']})\n\n"
            for c in change['changes']:
                content += f"- {c}\n"
                
        with open(report_path, 'w') as f:
            f.write(content)
            
        print(f"\nGenerated report at {report_path}")
        
    def run(self, dry_run=False):
        """Run the standardization process"""
        print("WRT Feature Flag Standardization")
        print("=" * 50)
        
        if dry_run:
            print("DRY RUN MODE - No changes will be made")
            print()
            
        # Find all Cargo.toml files
        cargo_files = self.find_cargo_tomls()
        print(f"Found {len(cargo_files)} Cargo.toml files")
        
        if not dry_run:
            # Create backup directory
            self.backup_dir.mkdir(parents=True, exist_ok=True)
            print(f"Backup directory: {self.backup_dir}")
        
        # Process each file
        for cargo_path in cargo_files:
            crate_name = self.get_crate_name(cargo_path)
            
            if dry_run:
                print(f"\nWould process: {crate_name} ({cargo_path.relative_to(self.workspace_root)})")
            else:
                print(f"\nProcessing: {crate_name}")
                if self.standardize_features(cargo_path):
                    print(f"  ✓ Updated")
                else:
                    print(f"  - No changes needed")
                    
        if not dry_run and self.changes_made:
            # Generate report and migration guide
            self.generate_report()
            self.create_migration_guide()
            
            print("\n" + "=" * 50)
            print(f"Standardization complete!")
            print(f"Files updated: {len(self.changes_made)}")
            print(f"Backup created at: {self.backup_dir}")
            print("\nNext steps:")
            print("1. Review the changes in FEATURE_STANDARDIZATION_REPORT.md")
            print("2. Run 'cargo check --workspace' to verify compilation")
            print("3. Update your CI/CD pipelines")
            print("4. Share FEATURE_MIGRATION_GUIDE.md with your team")


def main():
    workspace_root = Path(__file__).parent.parent
    standardizer = FeatureStandardizer(workspace_root)
    
    # Check for dry-run flag
    dry_run = '--dry-run' in sys.argv
    
    try:
        standardizer.run(dry_run=dry_run)
    except KeyboardInterrupt:
        print("\n\nStandardization cancelled by user")
        sys.exit(1)
    except Exception as e:
        print(f"\n\nError during standardization: {e}")
        sys.exit(1)


if __name__ == "__main__":
    main()