#!/usr/bin/env python3

with open('wrt-decoder/src/resource_limits_section.rs', 'r') as f:
    lines = f.readlines()

in_string = False
string_start_line = 0

for i, line in enumerate(lines):
    line_num = i + 1
    for j, char in enumerate(line):
        if char == '"':
            if in_string:
                # End of string
                in_string = False
                print(f"String ends at line {line_num}")
            else:
                # Start of string
                in_string = True
                string_start_line = line_num
                print(f"String starts at line {line_num}")

if in_string:
    print(f"ERROR: Unclosed string starting at line {string_start_line}")
else:
    print("All strings properly closed")