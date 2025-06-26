#!/usr/bin/env python3

def check_file_quotes_line_by_line(filepath):
    """Check each line for odd number of quotes"""
    print(f"Checking {filepath} line by line for odd quotes...")
    
    with open(filepath, 'r', encoding='utf-8') as f:
        lines = f.readlines()
    
    odd_quote_lines = []
    total_quotes = 0
    
    for line_num, line in enumerate(lines, 1):
        quote_count = line.count('"')
        total_quotes += quote_count
        
        if quote_count % 2 == 1:
            odd_quote_lines.append((line_num, quote_count, line.rstrip()))
            print(f"Line {line_num:4d}: {quote_count} quotes (ODD) - {repr(line.rstrip())}")
    
    print(f"\nSummary:")
    print(f"Total quotes in file: {total_quotes} ({'odd' if total_quotes % 2 == 1 else 'even'})")
    print(f"Lines with odd quotes: {len(odd_quote_lines)}")
    
    if total_quotes % 2 == 1:
        print("⚠️  FILE HAS UNTERMINATED STRING!")
    else:
        print("✅ File has even quotes")
    
    return odd_quote_lines

if __name__ == "__main__":
    odd_lines = check_file_quotes_line_by_line("wrt-decoder/src/resource_limits_section.rs")