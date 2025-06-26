#!/usr/bin/env python3

import sys
import os

def find_unterminated_string(filepath):
    """Find the exact location of unterminated strings."""
    try:
        with open(filepath, 'r', encoding='utf-8') as f:
            content = f.read()
    except Exception as e:
        print(f"Error reading {filepath}: {e}")
        return None
    
    in_string = False
    escape_next = False
    quote_start = 0
    quote_count = 0
    
    for i, char in enumerate(content):
        if not in_string and char == '"':
            # Start of string
            in_string = True
            escape_next = False
            quote_start = i
            quote_count += 1
            line_num = content[:i].count('\n') + 1
            print(f"String starts at line {line_num}, pos {i}")
        elif in_string:
            if escape_next:
                escape_next = False
            elif char == '\\':
                escape_next = True
            elif char == '"':
                # End of string
                in_string = False
                quote_count += 1
                line_num = content[:i].count('\n') + 1
                print(f"String ends at line {line_num}, pos {i}")
    
    if in_string:
        line_num = content[:quote_start].count('\n') + 1
        print(f"\n*** UNTERMINATED STRING ***")
        print(f"Starts at line {line_num}, position {quote_start}")
        
        # Show context
        start = max(0, quote_start - 50)
        end = min(len(content), quote_start + 150)
        context = content[start:end]
        print(f"Context: {repr(context)}")
        
        return quote_start, line_num
    
    print(f"Total quotes: {quote_count} (all properly terminated)")
    return None

if __name__ == "__main__":
    if len(sys.argv) != 2:
        print("Usage: python3 fix_file_quotes.py <filepath>")
        sys.exit(1)
    
    filepath = sys.argv[1]
    find_unterminated_string(filepath)