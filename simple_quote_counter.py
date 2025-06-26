#!/usr/bin/env python3

import sys

def simple_quote_count(filepath):
    """Simple quote counter - just count " characters."""
    try:
        with open(filepath, 'r', encoding='utf-8') as f:
            content = f.read()
        quotes = content.count('"')
        print(f"{filepath}: {quotes} quotes ({'odd' if quotes % 2 == 1 else 'even'})")
        return quotes
    except Exception as e:
        print(f"Error reading {filepath}: {e}")
        return 0

if __name__ == "__main__":
    if len(sys.argv) != 2:
        print("Usage: python3 simple_quote_counter.py <filepath>")
        sys.exit(1)
    
    simple_quote_count(sys.argv[1])