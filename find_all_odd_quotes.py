#!/usr/bin/env python3

import sys

def find_all_odd_quote_positions(filepath):
    """Find ALL positions where quote count becomes odd."""
    try:
        with open(filepath, 'r', encoding='utf-8') as f:
            content = f.read()
    except Exception as e:
        print(f"Error reading {filepath}: {e}")
        return
    
    print(f"Scanning {filepath} for all odd quote positions...")
    
    quote_count = 0
    odd_positions = []
    
    for i, char in enumerate(content):
        if char == '"':
            quote_count += 1
            line_num = content[:i].count('\n') + 1
            char_in_line = i - content.rfind('\n', 0, i)
            
            if quote_count % 2 == 1:
                odd_positions.append((i, line_num, char_in_line))
                print(f"ODD quote #{quote_count:3d} at pos {i:5d}, line {line_num:4d}, char {char_in_line:3d}")
                
                # Show context
                start = max(0, i - 30)
                end = min(len(content), i + 30)
                context = content[start:end]
                quote_pos = i - start
                before = context[:quote_pos]
                after = context[quote_pos+1:]
                print(f"    Context: {repr(before)}>>>\"{repr(after[:20])}")
            else:
                print(f"even quote #{quote_count:3d} at pos {i:5d}, line {line_num:4d}, char {char_in_line:3d}")
    
    print(f"\nTotal quotes: {quote_count}")
    print(f"Odd quote positions: {len(odd_positions)}")
    
    if quote_count % 2 == 1:
        print("⚠️  Final quote count is ODD - file has unterminated string!")
        if odd_positions:
            last_odd = odd_positions[-1]
            print(f"Last unterminated quote at position {last_odd[0]}, line {last_odd[1]}")
    else:
        print("✅ Final quote count is EVEN - all strings should be terminated")

if __name__ == "__main__":
    if len(sys.argv) != 2:
        print("Usage: python3 find_all_odd_quotes.py <filepath>")
        sys.exit(1)
    
    find_all_odd_quote_positions(sys.argv[1])