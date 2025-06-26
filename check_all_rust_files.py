#!/usr/bin/env python3

import os
import glob

def count_quotes_in_file(filepath):
    """Count quotes in a file, handling escape sequences properly."""
    try:
        with open(filepath, 'r', encoding='utf-8') as f:
            content = f.read()
    except Exception as e:
        return None, f"Error reading file: {e}"
    
    quote_count = 0
    in_string = False
    escape_next = False
    
    for char in content:
        if not in_string and char == '"':
            # Start of string
            in_string = True
            escape_next = False
            quote_count += 1
        elif in_string:
            if escape_next:
                escape_next = False
            elif char == '\\':
                escape_next = True
            elif char == '"':
                # End of string
                in_string = False
                quote_count += 1
    
    return quote_count, "unterminated" if in_string else "ok"

def find_all_rust_files():
    """Find all .rs files in the project."""
    rust_files = []
    for root, dirs, files in os.walk('.'):
        # Skip target directory and other build artifacts
        if 'target' in root or '.git' in root:
            continue
        for file in files:
            if file.endswith('.rs'):
                rust_files.append(os.path.join(root, file))
    return sorted(rust_files)

def main():
    rust_files = find_all_rust_files()
    print(f"Found {len(rust_files)} Rust files to check\n")
    
    files_with_odd_quotes = []
    files_with_unterminated = []
    
    for filepath in rust_files:
        quote_count, status = count_quotes_in_file(filepath)
        
        if quote_count is None:
            print(f"ERROR: {filepath} - {status}")
            continue
            
        if status == "unterminated":
            files_with_unterminated.append(filepath)
            print(f"UNTERMINATED: {filepath} - {quote_count} quotes")
        elif quote_count % 2 != 0:
            files_with_odd_quotes.append(filepath)
            print(f"ODD QUOTES: {filepath} - {quote_count} quotes")
    
    print(f"\n=== SUMMARY ===")
    print(f"Total Rust files checked: {len(rust_files)}")
    print(f"Files with odd quote counts: {len(files_with_odd_quotes)}")
    print(f"Files with unterminated strings: {len(files_with_unterminated)}")
    
    if files_with_odd_quotes:
        print(f"\nFiles with odd quote counts:")
        for f in files_with_odd_quotes:
            print(f"  - {f}")
    
    if files_with_unterminated:
        print(f"\nFiles with unterminated strings:")
        for f in files_with_unterminated:
            print(f"  - {f}")
    
    if not files_with_odd_quotes and not files_with_unterminated:
        print("\n✅ All Rust files have properly balanced quotes!")

if __name__ == "__main__":
    main()