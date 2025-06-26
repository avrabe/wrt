#!/usr/bin/env python3

with open('wrt-decoder/src/resource_limits_section.rs', 'r') as f:
    content = f.read()

in_string = False
escape_next = False
quote_start = 0
line_start = 0

for i, char in enumerate(content):
    if char == '\n':
        line_start = i + 1
    
    if not in_string and char == '"':
        # Start of string
        in_string = True
        escape_next = False
        quote_start = i
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
            line_num = content[:i].count('\n') + 1
            print(f"String ends at line {line_num}, pos {i}")

if in_string:
    line_num = content[:quote_start].count('\n') + 1
    print(f"ERROR: Unterminated string starting at line {line_num}, position {quote_start}")
    start = max(0, quote_start - 50)
    end = min(len(content), quote_start + 100)
    context = content[start:end]
    print(f"Context: {repr(context)}")
else:
    print("All strings are properly terminated")