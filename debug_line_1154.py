#!/usr/bin/env python3

with open('wrt-decoder/src/resource_limits_section.rs', 'r') as f:
    lines = f.readlines()

# Check lines around 1154
start_line = max(0, 1154 - 10)
end_line = min(len(lines), 1154 + 10)

print(f"Checking lines {start_line + 1} to {end_line}")
print("=" * 50)

for i in range(start_line, end_line):
    line_num = i + 1
    line = lines[i]
    quote_count = line.count('"')
    print(f"Line {line_num:4d} (quotes: {quote_count}): {line.rstrip()}")
    
    # Show each quote position
    for j, char in enumerate(line):
        if char == '"':
            print(f"    Quote at position {j}: '{line[max(0, j-5):j+6]}'")