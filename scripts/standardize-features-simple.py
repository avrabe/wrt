#!/usr/bin/env python3
"""
Simple Feature Flag Standardization Script for WRT Project

This script standardizes feature flags using text processing for better compatibility.
"""

import os
import re
import shutil
import sys
from pathlib import Path
from datetime import datetime
from typing import Dict, List, Set, Tuple

# Feature standardization mappings
FEATURE_MAPPINGS = {
    'safety': 'safety-asil-b',
    'safety-critical': 'safety-asil-c',
    'linux': 'platform-linux',
    'qnx': 'platform-qnx',
    'vxworks': 'platform-vxworks',
    'tock': 'platform-tock',
    'zephyr': 'platform-zephyr',
}

# Features to remove
DEPRECATED_FEATURES = [
    'disable-panic-handler',
    'custom-panic-handler',
]

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

class SimpleFeatureStandardizer:
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
            dirs[:] = [d for d in dirs if d not in ['target', '.git', 'node_modules', '.cargo']]
            
            if 'Cargo.toml' in files:
                cargo_files.append(Path(root) / 'Cargo.toml')
                
        return sorted(cargo_files)
    
    def get_crate_name(self, cargo_path: Path) -> str:
        """Extract crate name from Cargo.toml"""
        try:
            with open(cargo_path, 'r') as f:
                content = f.read()
                
            # Find package name
            match = re.search(r'^\s*name\s*=\s*["\']([^"\']+)["\']', content, re.MULTILINE)
            if match:
                return match.group(1)
        except:
            pass
        return 'unknown'
        
    def has_features_section(self, content: str) -> bool:
        """Check if the file has a [features] section"""
        return bool(re.search(r'^\s*\[features\]', content, re.MULTILINE))
        
    def add_features_section(self, content: str, crate_name: str) -> Tuple[str, List[str]]:
        """Add a features section if missing"""
        if self.has_features_section(content):
            return content, []
            
        changes = []
        
        # Core features for every crate
        features_section = '\n[features]\n'
        features_section += 'default = ["std"]\n'
        features_section += 'std = []\n'
        features_section += 'no_std = []\n'
        features_section += 'safety-asil-b = []\n'
        features_section += 'safety-asil-c = ["safety-asil-b"]\n'
        features_section += 'safety-asil-d = ["safety-asil-c"]\n'
        
        # Add KANI if appropriate
        if crate_name in KANI_CRATES:
            features_section += 'kani = []\n'
            changes.append("Added KANI feature")
            
        # Find a good place to insert the features section
        # Try to insert after [package] section or at the end
        package_match = re.search(r'(\[package\].*?)(?=\n\s*\[|\n\s*$)', content, re.DOTALL)
        if package_match:
            # Insert after package section
            end_pos = package_match.end()
            content = content[:end_pos] + '\n' + features_section + content[end_pos:]
        else:
            # Append at the end
            content += '\n' + features_section
            
        changes.append("Added core features section")
        return content, changes
        
    def standardize_features_in_content(self, content: str, crate_name: str) -> Tuple[str, List[str]]:
        """Standardize features in the content"""
        changes = []
        original_content = content
        
        # Add features section if missing
        content, section_changes = self.add_features_section(content, crate_name)
        changes.extend(section_changes)
        
        # Apply feature mappings
        for old_feature, new_feature in FEATURE_MAPPINGS.items():
            # Replace in feature definitions
            pattern = f'(^\\s*){re.escape(old_feature)}(\\s*=)'
            replacement = f'\\g<1>{new_feature}\\g<2>'
            
            if re.search(pattern, content, re.MULTILINE):
                content = re.sub(pattern, replacement, content, flags=re.MULTILINE)
                changes.append(f"Renamed feature '{old_feature}' to '{new_feature}'")
                
            # Replace in feature dependencies
            dep_pattern = f'(["\'])({re.escape(old_feature)})(["\'])'
            def replace_dep(match):
                quote = match.group(1)
                return f'{quote}{new_feature}{quote}'
                
            if re.search(dep_pattern, content):
                content = re.sub(dep_pattern, replace_dep, content)
                changes.append(f"Updated feature dependency '{old_feature}' to '{new_feature}'")
                
        # Remove deprecated features
        for deprecated in DEPRECATED_FEATURES:
            pattern = f'^\\s*{re.escape(deprecated)}\\s*=.*\n'
            if re.search(pattern, content, re.MULTILINE):
                content = re.sub(pattern, '', content, flags=re.MULTILINE)
                changes.append(f"Removed deprecated feature '{deprecated}'")
                
        # Add missing core safety features if in features section
        if self.has_features_section(content):
            features_content = self.extract_features_section(content)
            
            # Check for missing ASIL features
            missing_features = []
            if 'safety-asil-b' not in features_content:
                missing_features.append('safety-asil-b = []')
                changes.append("Added safety-asil-b feature")
                
            if 'safety-asil-c' not in features_content:
                missing_features.append('safety-asil-c = ["safety-asil-b"]')
                changes.append("Added safety-asil-c feature")
                
            if 'safety-asil-d' not in features_content:
                missing_features.append('safety-asil-d = ["safety-asil-c"]')
                changes.append("Added safety-asil-d feature")
                
            if crate_name in KANI_CRATES and 'kani' not in features_content:
                missing_features.append('kani = []')
                changes.append("Added kani feature")
                
            # Insert missing features
            if missing_features:
                features_section_end = re.search(r'(\[features\].*?)(?=\n\s*\[|\n\s*$)', content, re.DOTALL)
                if features_section_end:
                    end_pos = features_section_end.end()
                    new_features = '\n' + '\n'.join(missing_features) + '\n'
                    content = content[:end_pos] + new_features + content[end_pos:]
                    
        return content, changes
        
    def extract_features_section(self, content: str) -> str:
        """Extract the content of the [features] section"""
        match = re.search(r'\[features\](.*?)(?=\n\s*\[|\n\s*$)', content, re.DOTALL)
        return match.group(1) if match else ""
        
    def standardize_cargo_file(self, cargo_path: Path, dry_run: bool = False) -> bool:
        """Standardize a single Cargo.toml file"""
        try:
            with open(cargo_path, 'r') as f:
                content = f.read()
                
            crate_name = self.get_crate_name(cargo_path)
            new_content, changes = self.standardize_features_in_content(content, crate_name)
            
            if changes and content != new_content:
                if not dry_run:
                    self.backup_file(cargo_path)
                    
                    with open(cargo_path, 'w') as f:
                        f.write(new_content)
                        
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

| Old Feature | New Feature | Notes |
|-------------|-------------|-------|
| `safety` | `safety-asil-b` | ASIL-B safety level |
| `safety-critical` | `safety-asil-c` | ASIL-C safety level |
| `linux` | `platform-linux` | Platform-specific feature |
| `qnx` | `platform-qnx` | Platform-specific feature |
| `vxworks` | `platform-vxworks` | Platform-specific feature |

## Removed Features

The following features have been removed:
- `disable-panic-handler` (implied by `no_std`)
- `custom-panic-handler` (implied by `no_std`)

## New Safety Levels

All crates now support these safety levels:
- `safety-asil-b` - Basic safety features (ASIL-B)
- `safety-asil-c` - Critical safety features (ASIL-C)  
- `safety-asil-d` - Highest safety level (ASIL-D)

## KANI Support

The following crates now have KANI formal verification support:
- wrt-foundation, wrt-component, wrt-sync
- wrt-runtime, wrt-platform, wrt-instructions
- wrt-decoder, wrt-host, wrt-debug
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
            
        cargo_files = self.find_cargo_tomls()
        print(f"Found {len(cargo_files)} Cargo.toml files")
        
        if not dry_run:
            self.backup_dir.mkdir(parents=True, exist_ok=True)
            print(f"Backup directory: {self.backup_dir}")
        
        for cargo_path in cargo_files:
            crate_name = self.get_crate_name(cargo_path)
            
            if dry_run:
                print(f"\nAnalyzing: {crate_name} ({cargo_path.relative_to(self.workspace_root)})")
                # Run in dry-run mode to see what would change
                self.standardize_cargo_file(cargo_path, dry_run=True)
            else:
                print(f"\nProcessing: {crate_name}")
                if self.standardize_cargo_file(cargo_path, dry_run=False):
                    print(f"  ✓ Updated")
                else:
                    print(f"  - No changes needed")
                    
        if self.changes_made:
            print(f"\nSummary: {len(self.changes_made)} files would be updated" if dry_run else f"\nSummary: {len(self.changes_made)} files updated")
            
            if not dry_run:
                self.generate_report()
                self.create_migration_guide()
                
                print("\n" + "=" * 50)
                print("Standardization complete!")
                print("Next steps:")
                print("1. Review FEATURE_STANDARDIZATION_REPORT.md")
                print("2. Run 'cargo check --workspace'")
                print("3. Update CI/CD pipelines")
            else:
                print("\nRun without --dry-run to apply changes")


def main():
    workspace_root = Path(__file__).parent.parent
    standardizer = SimpleFeatureStandardizer(workspace_root)
    
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