#!/usr/bin/env python3

import sys

def find_odd_quotes_sliding_window(filepath, window_size=100):
    """Use sliding window to find exact location of odd quotes."""
    try:
        with open(filepath, 'r', encoding='utf-8') as f:
            content = f.read()
    except Exception as e:
        print(f"Error reading {filepath}: {e}")
        return
    
    print(f"Checking {filepath} with sliding window approach...")
    print(f"Total file length: {len(content)} characters")
    
    # First, get the total quote count
    total_quotes = content.count('"')
    print(f"Total quotes in file: {total_quotes}")
    
    if total_quotes % 2 == 0:
        print("File has even number of quotes - should be fine")
        return
    
    print(f"File has ODD number of quotes ({total_quotes}) - finding location...")
    
    # Use sliding window to find where quotes become odd
    quote_count = 0
    last_even_position = 0
    
    for i in range(0, len(content), window_size):
        window_end = min(i + window_size, len(content))
        window = content[i:window_end]
        window_quotes = window.count('"')
        quote_count += window_quotes
        
        line_num = content[:window_end].count('\n') + 1
        
        if quote_count % 2 == 0:
            last_even_position = window_end
            print(f"Position {window_end:6d} (line ~{line_num:4d}): quotes={quote_count:3d} (even) ✓")
        else:
            print(f"Position {window_end:6d} (line ~{line_num:4d}): quotes={quote_count:3d} (odd)  ✗")
            
            # Found where it goes odd - narrow down
            print(f"\n🔍 Narrowing down between positions {last_even_position} and {window_end}")
            
            # Search character by character in this window
            search_start = max(0, last_even_position - 10)
            search_end = min(len(content), window_end + 10)
            
            char_quote_count = content[:search_start].count('"')
            print(f"Starting search at position {search_start} with {char_quote_count} quotes")
            
            for j in range(search_start, search_end):
                if content[j] == '"':
                    char_quote_count += 1
                    line_num = content[:j].count('\n') + 1
                    char_in_line = j - content.rfind('\n', 0, j)
                    
                    if char_quote_count % 2 == 1:
                        print(f"📍 ODD quote #{char_quote_count} at position {j}, line {line_num}, char {char_in_line}")
                        
                        # Show context around this quote
                        context_start = max(0, j - 50)
                        context_end = min(len(content), j + 50)
                        context = content[context_start:context_end]
                        quote_pos_in_context = j - context_start
                        
                        print(f"Context (quote marked with >>><<<):")
                        before = context[:quote_pos_in_context]
                        after = context[quote_pos_in_context+1:]
                        print(f"{repr(before)}>>>\"{after[:50]}<<<{repr(after[50:])}")
                        
                        # Show the actual line
                        line_start = content.rfind('\n', 0, j) + 1
                        line_end = content.find('\n', j)
                        if line_end == -1:
                            line_end = len(content)
                        line_content = content[line_start:line_end]
                        print(f"\nLine {line_num}: {repr(line_content)}")
                        return j, line_num
                    else:
                        print(f"   EVEN quote #{char_quote_count} at position {j}, line {line_num}, char {char_in_line}")
            
            break
    
    print("Could not find the exact odd quote location")

if __name__ == "__main__":
    if len(sys.argv) != 2:
        print("Usage: python3 sliding_window_quote_finder.py <filepath>")
        sys.exit(1)
    
    filepath = sys.argv[1]
    find_odd_quotes_sliding_window(filepath)